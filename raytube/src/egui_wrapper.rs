use crate::phosphor::{DisplaySettings, PhosphorDisplay};
use egui::PaintCallbackInfo;
use egui::Vec2;
use egui::{self, DragValue};
use egui_glow::glow as gl;
use gl::HasContext;

pub struct EguiGlProgram {
    program: gl::Program,
    vertex_array: gl::VertexArray,
    pub disp: PhosphorDisplay,
}

#[expect(unsafe_code)] // we need unsafe code to use gl
impl EguiGlProgram {
    pub fn new(gl: &gl::Context, width: i32, height: i32) -> Option<Self> {
        unsafe {
            let program = crate::create_program(
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

    pub fn destroy(&self, gl: &gl::Context) {
        use gl::HasContext as _;
        unsafe {
            self.disp.destroy(gl);
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    pub fn paint(
        &mut self,
        info: PaintCallbackInfo,
        gl: &gl::Context,
        display_settings: &DisplaySettings,
        points: &[Vec2],
        dt: f32,
    ) {
        use gl::HasContext as _;

        unsafe {
            // Do internal painting
            self.disp.paint(gl, display_settings, points, dt);

            // Set variables
            gl.use_program(Some(self.program));
            let [r, g, b] = display_settings.phosphor_color;
            gl.uniform_3_f32(
                gl.get_uniform_location(self.program, "phosphor_color").as_ref(),
                r, g, b,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "barrel").as_ref(),
                display_settings.barrel_distortion,
            );

            gl.active_texture(gl::TEXTURE0);
            gl.bind_texture(gl::TEXTURE_2D, Some(self.disp.back_buffer.texture));

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

impl egui::Widget for &mut DisplaySettings {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Intensity: ");
                ui.add(DragValue::new(&mut self.intensity).speed(1e-3));
            });
            ui.horizontal(|ui| {
                ui.label("Memory time: ");
                ui.add(DragValue::new(&mut self.memory_time).speed(1e-9));
            });
            ui.horizontal(|ui| {
                ui.label("Saturation pt.: ");
                ui.add(DragValue::new(&mut self.saturation_point).speed(1e-3));
            });
            ui.horizontal(|ui| {
                ui.label("Amplitude: ");
                ui.add(DragValue::new(&mut self.amplitude).speed(1e-3));
            });
            ui.horizontal(|ui| {
                ui.label("Pause: ");
                if ui
                    .button(if self.paused {
                        "[Pause] / Play"
                    } else {
                        "Pause / [Play]"
                    })
                    .clicked()
                {
                    self.paused = !self.paused;
                }
            });
           ui.horizontal(|ui| {
                ui.label("Barrel distortion: ");
                ui.add(DragValue::new(&mut self.barrel_distortion).speed(1e-3));
            });
            ui.horizontal(|ui| {
                ui.label("Phosphor color: ");
                ui.color_edit_button_rgb(&mut self.phosphor_color);
            });
            ui.horizontal(|ui| {
                ui.label("Continue from last sample: ");
                ui.checkbox(&mut self.continue_from_last_sample, "Enable");
            });


            if ui.button("Restore defaults").clicked() {
                *self = DisplaySettings::default();
            }
        }).response
    }
}
