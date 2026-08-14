use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use eframe::egui;
use eframe::egui::Vec2;
use eframe::egui_glow;
use eframe::glow as gl;
use std::sync::{Arc, Mutex};

use raytube::egui_wrapper::EguiGlProgram;
use raytube::phosphor::{MAX_BEAM_SAMPLES, DisplaySettings};

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
    points: Arc<Mutex<Vec<Vec2>>>,
    time: f64,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SaveData {
    display_settings: DisplaySettings,
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut save_data: SaveData = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        save_data.display_settings.paused = false;

        let gl = cc.gl.as_ref().unwrap();

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .expect("no output device available");

        let config = device
            .default_input_config()
            .expect("No output config")
            .config();

        let shared_points = Arc::new(Mutex::new(Vec::new()));
        let shared_points_clone = shared_points.clone();

        let stream = device.build_input_stream(
            config, 
            move |data: &[f32], _info| {
                let mut points = Vec::new();
                for xy in data.chunks_exact(2) {
                    points.push(Vec2::new(xy[0], xy[1]));
                }

                shared_points_clone.lock().unwrap().extend_from_slice(&points);
            }, 
            move |err| {
                eprintln!("Output stream error {err}");
            }, 
            None).unwrap();

        stream.play().unwrap();

        std::mem::forget(stream);

        Self {
            points: shared_points,
            rendering: Arc::new(Mutex::new(EguiGlProgram::new(gl, 1024, 1024).unwrap())),
            time: 0.0,
            save_data,
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

        let (rect, _response) = ui.allocate_exact_size(
            egui::Vec2::new(width as f32, height as f32) / ui.pixels_per_point(),
            egui::Sense::drag(),
        );

        // Clone locals so we can move them into the paint callback:
        let rendering = self.rendering.clone();

        let display_settings = self.save_data.display_settings.clone();

        let mut points = std::mem::take(&mut *self.points.lock().unwrap());
        points.truncate(MAX_BEAM_SAMPLES - 1);

        // Ensure an even number of samples
        let cb = egui_glow::CallbackFn::new(move |info, painter| {
            rendering.lock().unwrap().paint(
                info,
                painter.gl(),
                &display_settings,
                &points,
                dt as f32,
            );
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
            ui.add(&mut self.save_data.display_settings);
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
