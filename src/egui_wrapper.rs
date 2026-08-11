use eframe::egui;
use eframe::egui::DragValue;
use eframe::egui::Vec2;
use eframe::egui_glow;
use eframe::glow as gl;
use egui::PaintCallbackInfo;
use gl::HasContext;
use std::sync::{Arc, Mutex};
use crate::phosphor::{PhosphorDisplay, DisplaySettings};

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

    pub fn paint(&mut self, info: PaintCallbackInfo, gl: &gl::Context, display_settings: &DisplaySettings, lines: &[[f32; 3]], dt: f32) {
        use gl::HasContext as _;

        unsafe {
            // Do internal painting
            self.disp.paint(gl, display_settings, lines, dt);

            // Set variables
            gl.use_program(Some(self.program));
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
