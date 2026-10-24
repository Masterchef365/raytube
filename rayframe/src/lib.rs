use eframe::egui;
use eframe::egui::Vec2;
use eframe::egui_glow;
use eframe::glow as gl;
use std::sync::{Arc, Mutex};

use raytube::egui_wrapper::EguiGlProgram;
use raytube::phosphor::{MAX_BEAM_SAMPLES, DisplaySettings};

pub type SharedSamples = Arc<Mutex<Vec<[f32; 2]>>>;

pub trait RayFrameApp: Send + Sync + 'static {
    fn update(&mut self, frame: &mut [[f32; 2]], sample_rate: u32);
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SaveData {
    display_settings: DisplaySettings,
}

struct RayFrame<A> {
    save_data: SaveData,
    rendering: Arc<Mutex<EguiGlProgram>>,
    shared_resources: Arc<Mutex<SharedResources<A>>>,
}

impl<A: RayFrameApp> RayFrame<A> {
    pub fn new(app: A, cc: &eframe::CreationContext<'_>) -> Self {
        let save_data: SaveData = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        let gl = cc.gl.as_ref().unwrap();

        Self {
            save_data,
            // TODO: Configurable screen resolution, size
            rendering: Arc::new(Mutex::new(EguiGlProgram::new(gl, 1024, 1024).unwrap())),
            shared_resources: SharedResources::new(app),
        }
    }
}

struct SharedResources<A> {
    app: A,
    front_buffer: SharedSamples,
    back_buffer: SharedSamples,
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
pub fn mainloop<A: RayFrameApp>(app: A) -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Ok(Box::new(RayFrame::new(app, cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
pub fn mainloop<A: RayFrameApp>(app: A) {
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
                Box::new(|cc| Ok(Box::new(RayFrame::new(app, cc)))),
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

impl<A: RayFrameApp> RayFrame<A> {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
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

        /*
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
        */
    }
}

impl<A: RayFrameApp> eframe::App for RayFrame<A> {
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

impl<A: RayFrameApp> SharedResources<A> {
    #[cfg(feature = "audio")]
    pub fn new(app: A) -> Arc<Mutex<Self>> {
        use cpal::Data;
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();

        let device = host.default_output_device().expect("no output device available");

        let mut supported_configs_range = device.supported_output_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .find(|p| p.channels() == 2)
            .expect("no supported config?!")
            .with_max_sample_rate();

        let inst = Arc::new(Mutex::new(Self::from_app(app)));

        let audio_inst = inst.clone();

        let sample_rate = supported_config.config().sample_rate;

        let stream = device.build_output_stream(
            supported_config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                audio_inst.lock().unwrap().app.update(bytemuck::cast_slice_mut(data), sample_rate);
            },
            move |err| {
                // TODO: Logging on WASM
                eprintln!("Error: {err}");
            },
            None // Timeout for stream initialization: None = wait indefinitely,
                 // Some(Duration) = time to wait for the backend to initialize the stream.
        );

        std::mem::forget(stream);

        inst
    }

    #[cfg(not(feature = "audio"))]
    pub fn new(app: A) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::from_app(app)))
    }

    pub fn from_app(app: A) -> Self {
        Self {
            back_buffer: Default::default(),
            front_buffer: Default::default(),
            app,
        }
    }

    pub fn update(&mut self) -> SharedSamples {
        todo!()
    }
}
