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

pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SaveData {
    display_settings: DisplaySettings,
}

struct RayFrame<A> {
    save_data: SaveData,
    rendering: Arc<Mutex<EguiGlProgram>>,
    shared_resources: Arc<Mutex<SharedResources<A>>>,
    sample_rate: u32,
    last_timestamp: f64,
}

impl<A: RayFrameApp> RayFrame<A> {
    pub fn new(app: A, cc: &eframe::CreationContext<'_>) -> Self {
        let save_data: SaveData = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        let gl = cc.gl.as_ref().unwrap();

        let shared_resources = SharedResources::new(app);
        let sample_rate = shared_resources.lock().unwrap().sample_rate;

        Self {
            save_data,
            // TODO: Configurable screen resolution, size
            rendering: Arc::new(Mutex::new(EguiGlProgram::new(gl, 1024, 1024).unwrap())),
            sample_rate,
            shared_resources,
            last_timestamp: cc.egui_ctx.time(),
        }
    }
}

struct SharedResources<A> {
    app: A,
    sample_rate: u32,
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
pub fn mainloop<A: RayFrameApp>(app: A) -> eframe::Result {
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
    Ok(())
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

        // Time
        let time = ui.time();
        let dt = time - self.last_timestamp;
        self.last_timestamp = time;

        // Get points
        let mut points = self.shared_resources.lock().unwrap().update(dt as f32);

        let dt = points.len() as f32 / self.sample_rate as f32;

        points.truncate(MAX_BEAM_SAMPLES - 1);

        // Ensure an even number of samples
        let cb = egui_glow::CallbackFn::new(move |info, painter| {
            rendering.lock().unwrap().paint(
                info,
                painter.gl(),
                &display_settings,
                bytemuck::cast_slice(points.as_slice()),
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
            .with_standard_sample_rate();

        let sample_rate = supported_config.config().sample_rate;

        let inst = Arc::new(Mutex::new(Self::from_app(app, sample_rate)));

        let audio_inst = inst.clone();

        let stream = device.build_output_stream(
            supported_config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut lck = audio_inst.lock().unwrap();
                lck.app.update(bytemuck::cast_slice_mut(data), sample_rate);
                lck.put_data(bytemuck::cast_slice(data));
            },
            move |err| {
                // TODO: Logging on WASM
                eprintln!("Error: {err}");
            },
            None // Timeout for stream initialization: None = wait indefinitely,
                 // Some(Duration) = time to wait for the backend to initialize the stream.
        ).unwrap();

        stream.play().unwrap();

        std::mem::forget(stream);

        inst
    }

    #[cfg(not(feature = "audio"))]
    pub fn new(app: A) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::from_app(app, DEFAULT_SAMPLE_RATE)))
    }

    pub fn from_app(app: A, sample_rate: u32) -> Self {
        Self {
            back_buffer: Arc::new(Mutex::new(vec![])),
            front_buffer: Arc::new(Mutex::new(vec![])),
            sample_rate,
            app,
        }
    }

    pub fn put_data(&mut self, frame: &[[f32; 2]]) {
        self.back_buffer.lock().unwrap().extend_from_slice(frame);
    }

    pub fn update(&mut self, real_dt: f32) -> Vec<[f32; 2]> {
        #[cfg(not(feature = "audio"))]
        {
            let n_samples_this_frame = (real_dt * DEFAULT_SAMPLE_RATE as f32).ceil() as usize;
            let mut buffer = vec![[0.0; 2]; n_samples_this_frame];
            self.app.update(&mut buffer, DEFAULT_SAMPLE_RATE);
            self.put_data(bytemuck::cast_slice(&buffer));
        }

        std::mem::swap(&mut self.back_buffer, &mut self.front_buffer);
        std::mem::take(&mut self.front_buffer.lock().unwrap())
    }
}
