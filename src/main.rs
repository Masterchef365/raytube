use eframe::egui;
use eframe::egui::DragValue;
use eframe::egui::Pos2;
use eframe::egui::Vec2;
use eframe::egui_glow;
use eframe::glow as gl;
use egui::PaintCallbackInfo;
use gl::HasContext;
use rodio::Source;
use rodio::microphone::Microphone;
use rodio::microphone::MicrophoneBuilder;
use std::sync::{Arc, Mutex};

mod phosphor;
mod egui_wrapper;
use phosphor::{DisplaySettings, PhosphorDisplay};
use egui_wrapper::EguiGlProgram;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let mut web_options = eframe::WebOptions::default();
    web_options.webgl_context_option = eframe::WebGlContextOption::WebGl2;

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

pub struct TemplateApp {
    pub save_data: SaveData,
    rendering: Arc<Mutex<EguiGlProgram>>,
    time: f64,
    microphone: Microphone,
    amplitude: f32,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SaveData {
    display_settings: DisplaySettings,
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let save_data = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        let gl = cc.gl.as_ref().unwrap();

        let microphone = MicrophoneBuilder::new().default_device().unwrap().default_config().unwrap().open_stream().unwrap();

        Self {
            rendering: Arc::new(Mutex::new(EguiGlProgram::new(gl, 1024, 1024).unwrap())),
            microphone,
            time: 0.0,
            save_data,
            amplitude: 1.0,
        }
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let time = ui.time();
        let dt = time - self.time;
        self.time = time;

        let (width, height) = {
            let lock = self.rendering.lock().unwrap();
            (lock.disp.width, lock.disp.height)
        };

        let (rect, _response) =
            ui.allocate_exact_size(egui::Vec2::new(width as f32, height as f32) / ui.pixels_per_point(), egui::Sense::drag());

        // Clone locals so we can move them into the paint callback:
        let rendering = self.rendering.clone();

        let display_settings = self.save_data.display_settings.clone();

        // Get samples
        let sample_rate_hz: u32 = self.microphone.sample_rate().into();
        let mut n_samp = (dt * sample_rate_hz as f64).ceil().clamp(0., sample_rate_hz as f64) as usize;
        if n_samp & 1 == 1 {
            n_samp += 1;
        }

        //let sweep_time_per_line = dt as f32 / n_samp as f32;

        let mut mic = self.microphone.by_ref().take(n_samp);
        let mut lines = Vec::new();
        let mut last_pos = Pos2::new(0., 0.);
        while let (Some(x), Some(y)) = (mic.next(), mic.next()) {
            let pos = (Vec2::new(x, y) * self.amplitude).to_pos2();
            let dist = last_pos.distance(pos);
            let intensity = if dist < 1.0 { 1.0 } else { 1.0 / dist };
            lines.push([pos.x * self.amplitude, pos.y * self.amplitude, intensity]);
            last_pos = pos;
        }

        // Ensure an even number of samples
        let cb = egui_glow::CallbackFn::new(move |info, painter| {
            rendering
                .lock()
                .unwrap()
                .paint(info, painter.gl(), &display_settings, &lines, dt as f32);
        });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.save_data);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint();

        egui::Panel::left("cfg").show(ui, |ui| {
            ui.strong("Phosphor");
            ui.horizontal(|ui| {
                ui.label("Intensity: ");
                ui.add(DragValue::new(&mut self.save_data.display_settings.intensity).speed(1e-3));
            });
            ui.horizontal(|ui| {
                ui.label("Memory time: ");
                ui.add(DragValue::new(&mut self.save_data.display_settings.memory_time).speed(1e-3));
            });
            ui.horizontal(|ui| {
                ui.label("Saturation pt.: ");
                ui.add(DragValue::new(&mut self.save_data.display_settings.saturation_point).speed(1e-3));
            });
            ui.horizontal(|ui| {
                ui.label("Amplitude: ");
                ui.add(DragValue::new(&mut self.amplitude).speed(1e-3));
            });

            ui.separator();
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                self.custom_painting(ui);
            });
        });
    }

    fn on_exit(&mut self, gl: Option<&gl::Context>) {
        self.rendering.lock().unwrap().destroy(gl.unwrap());
    }
}

pub fn create_program(gl: &gl::Context, shader_sources: &[(u32, &str)]) -> gl::Program {
    unsafe {
        let program = gl.create_program().expect("Cannot create program");
        let shader_version = egui_glow::ShaderVersion::get(gl);

        let shaders: Vec<_> = shader_sources
            .iter()
            .map(|(shader_type, shader_source)| {
                let shader = gl
                    .create_shader(*shader_type)
                    .expect("Cannot create shader");
                gl.shader_source(
                    shader,
                    &format!(
                        "{}\n{}",
                        shader_version.version_declaration(),
                        shader_source
                    ),
                );
                gl.compile_shader(shader);
                assert!(
                    gl.get_shader_compile_status(shader),
                    "Failed to compile custom_3d_glow {shader_type}: {}",
                    gl.get_shader_info_log(shader)
                );

                gl.attach_shader(program, shader);
                shader
            })
            .collect();

        gl.link_program(program);
        assert!(
            gl.get_program_link_status(program),
            "{}",
            gl.get_program_info_log(program)
        );

        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        program
    }
}
