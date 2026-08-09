use eframe::egui_glow;
use eframe::glow as gl;
use egui::PaintCallbackInfo;
use gl::HasContext;
use std::sync::{Arc, Mutex};

use egui::DragValue;

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

    let web_options = eframe::WebOptions::default();

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
    angle: f32,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SaveData {}

impl Default for SaveData {
    fn default() -> Self {
        Self {}
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let save_data = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        let gl = cc.gl.as_ref().unwrap();

        Self {
            rendering: Arc::new(Mutex::new(EguiGlProgram::new(gl, 1024, 1024).unwrap())),
            angle: 0.0,
            save_data,
        }
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag());

        self.angle += response.drag_motion().x * 0.01;

        // Clone locals so we can move them into the paint callback:
        let angle = self.angle;
        let rendering = self.rendering.clone();

        let cb = egui_glow::CallbackFn::new(move |info, painter| {
            rendering.lock().unwrap().paint(info, painter.gl(), angle);
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

struct EguiGlProgram {
    program: gl::Program,
    vertex_array: gl::VertexArray,
    disp: PhosphorDisplay,
}

struct PhosphorDisplay {
    program: gl::Program,
    vertex_array: gl::VertexArray,
    front_buffer: PhosphorBuffer,
    back_buffer: PhosphorBuffer,
    width: i32,
    height: i32,
}

struct PhosphorBuffer {
    framebuffer: gl::NativeFramebuffer,
    texture: gl::NativeTexture,
}

#[expect(unsafe_code)] // we need unsafe code to use gl
impl PhosphorDisplay {
    fn new(gl: &gl::Context, width: i32, height: i32) -> Self {
        let program = create_program(
            gl,
            &[
                (gl::VERTEX_SHADER, include_str!("shaders/phosphor.vert")),
                (gl::FRAGMENT_SHADER, include_str!("shaders/phosphor.frag")),
            ],
        );

        let front_buffer = PhosphorBuffer::new(gl, width, height);
        let back_buffer = PhosphorBuffer::new(gl, width, height);

        let vertex_array = unsafe {
            gl.create_vertex_array()
                .expect("Cannot create vertex array")
        };

        Self {
            width: width,
            height: height,
            front_buffer,
            back_buffer,
            program,
            vertex_array,
        }
    }

    fn paint(&mut self, gl: &gl::Context, angle: f32) {
        use gl::HasContext as _;
        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_angle").as_ref(),
                angle,
            );
            gl.bind_vertex_array(Some(self.vertex_array));

            // Bind framebuffer
            gl.bind_framebuffer(gl::FRAMEBUFFER, Some(self.front_buffer.framebuffer));
            gl.viewport(0, 0, self.width, self.height);

            gl.draw_arrays(gl::TRIANGLES, 0, 3);
            std::mem::swap(&mut self.front_buffer, &mut self.back_buffer);
        }
    }
}

#[expect(unsafe_code)] // we need unsafe code to use gl
impl EguiGlProgram {
    fn new(gl: &gl::Context, width: i32, height: i32) -> Option<Self> {
        unsafe {
            let program = create_program(
                gl,
                &[
                    (gl::VERTEX_SHADER, include_str!("shaders/egui.vert")),
                    (gl::FRAGMENT_SHADER, include_str!("shaders/egui.frag")),
                ],
            );

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            let disp = PhosphorDisplay::new(gl, width, height);

            Some(Self {
                disp,
                program,
                vertex_array,
            })
        }
    }

    fn destroy(&self, gl: &gl::Context) {
        use gl::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    fn paint(&mut self, info: PaintCallbackInfo, gl: &gl::Context, angle: f32) {
        use gl::HasContext as _;
        unsafe {
            self.disp.paint(gl, angle);

            gl.use_program(Some(self.program));

            // Set variables
            let viewport_px = info.viewport_in_pixels();
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "width").as_ref(),
                viewport_px.width_px as f32,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "height").as_ref(),
                viewport_px.height_px as f32,
            );
            gl.active_texture(gl::TEXTURE0);
            gl.bind_texture(gl::TEXTURE_2D, Some(self.disp.front_buffer.texture));

            // Bind framebuffer
            gl.bind_framebuffer(gl::FRAMEBUFFER, None);

            // Fix the viewport config
            let viewport_px = info.viewport_in_pixels();
            gl.viewport(
                viewport_px.left_px,
                viewport_px.from_bottom_px,
                viewport_px.width_px,
                viewport_px.height_px,
            );

            let clip_rect = info.clip_rect_in_pixels();
            gl.scissor(
                clip_rect.left_px,
                clip_rect.from_bottom_px,
                clip_rect.width_px,
                clip_rect.height_px,
            );

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(gl::TRIANGLES, 0, 3);

        }
    }
}

impl PhosphorBuffer {
    pub fn new(gl: &gl::Context, width: i32, height: i32) -> Self {
        let framebuffer = unsafe { gl.create_framebuffer().unwrap() };

        // Create texture
        let texture = unsafe { gl.create_texture().unwrap() };
        unsafe {
            gl.bind_texture(gl::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                gl::TEXTURE_2D,
                0,
                gl::RGB as _,
                width,
                height,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                gl::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
            gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
        }

        // Bind to framebuffer
        unsafe {
            gl.bind_framebuffer(gl::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, Some(texture), 0);
            gl.draw_buffers(&[gl::COLOR_ATTACHMENT0]);
        }

        unsafe {
            if gl.check_framebuffer_status(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE as u32 {
                panic!("Framebuffer creation failed");
            }
        }

        Self {
            framebuffer,
            texture,
        }
    }

    
}

#[track_caller]
fn create_program(gl: &gl::Context, shader_sources: &[(u32, &str)]) -> gl::NativeProgram {
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
