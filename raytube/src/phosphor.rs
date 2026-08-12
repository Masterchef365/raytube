use egui;
use egui::Vec2;
use egui_glow;
use egui_glow::glow as gl;
use gl::HasContext;

const BEAM_TEXTURE_SIDE: usize = 128;
pub const MAX_BEAM_SAMPLES: usize = BEAM_TEXTURE_SIDE.pow(2);

pub struct PhosphorDisplay {
    beam_program: gl::Program,
    //beam_buffer: gl::Buffer,
    phosphor_program: gl::Program,
    phosphor_vertex_array: gl::VertexArray,

    beam_texture: gl::Texture,

    last_sample: Option<Vec2>,

    pub front_buffer: PhosphorBuffer,
    pub back_buffer: PhosphorBuffer,

    pub width: i32,
    pub height: i32,
}

pub struct PhosphorBuffer {
    pub framebuffer: gl::Framebuffer,
    pub texture: gl::Texture,
}

#[expect(unsafe_code)] // we need unsafe code to use gl
impl PhosphorDisplay {
    pub fn new(gl: &gl::Context, width: i32, height: i32) -> Self {
        let beam_program = crate::create_program(
            gl,
            &[
                (gl::VERTEX_SHADER, include_str!("shaders/beam.vert")),
                (gl::FRAGMENT_SHADER, include_str!("shaders/beam.frag")),
            ],
        );

        let phosphor_program = crate::create_program(
            gl,
            &[
                (gl::VERTEX_SHADER, include_str!("shaders/phosphor.vert")),
                (gl::FRAGMENT_SHADER, include_str!("shaders/phosphor.frag")),
            ],
        );

        let front_buffer = PhosphorBuffer::new(gl, width, height);
        let back_buffer = PhosphorBuffer::new(gl, width, height);

        let phosphor_vertex_array = unsafe {
            gl.create_vertex_array()
                .expect("Cannot create vertex array")
        };

        let beam_texture = unsafe { gl.create_texture().expect("Cannot create texture") };

        unsafe {
            gl.bind_texture(gl::TEXTURE_2D, Some(beam_texture));
            gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
            gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
        }

        Self {
            width,
            height,
            front_buffer,
            back_buffer,
            phosphor_program,
            beam_program,
            //beam_buffer,
            beam_texture,
            phosphor_vertex_array,
            last_sample: None,
        }
    }

    pub fn paint(
        &mut self,
        gl: &gl::Context,
        display_settings: &DisplaySettings,
        points: &[Vec2],
        dt: f32,
    ) {
        assert!(points.len() + 1 <= MAX_BEAM_SAMPLES);

        if display_settings.paused {
            unsafe {
                gl.bind_framebuffer(gl::FRAMEBUFFER, None);
            }
            return;
        }

        // amplitude = initial * background_intensity^(# of frames n)
        // T = n * dt
        // amplitude = initial * background_intensity^(T/dt)
        // amplitude = initial * (background_intensity^(1/dt))^T
        // const amplitude => background_intensity = k^dt => amplitude = initial * (k^T)

        let background_intensity = display_settings.memory_time.powf(dt);

        unsafe {
            // Upload beam buffer
            gl.bind_texture(gl::TEXTURE_2D, Some(self.beam_texture));
            let mut texture_buffer: Vec<f32> = vec![0_f32; MAX_BEAM_SAMPLES * 4];

            let maybe_first_pos = self.last_sample.filter(|_| display_settings.continue_from_last_sample);

            let mut first_index = 0;
            if let Some(first_pos) = maybe_first_pos {
                texture_buffer[0] = first_pos.x * display_settings.amplitude;
                texture_buffer[1] = first_pos.y * display_settings.amplitude;
                first_index += 1;
            }

            for i in 0..points.len() {
                let idx = first_index + i;
                texture_buffer[idx * 4 + 0] = points[i].x * display_settings.amplitude;
                texture_buffer[idx * 4 + 1] = points[i].y * display_settings.amplitude;
            }

            gl.tex_image_2d(
                gl::TEXTURE_2D,
                0,
                gl::RGBA32F as _,
                BEAM_TEXTURE_SIDE as _,
                BEAM_TEXTURE_SIDE as _,
                0,
                gl::RGBA,
                gl::FLOAT,
                gl::PixelUnpackData::Slice(Some(bytemuck::cast_slice(&texture_buffer))),
            );

            // Bind front framebuffer for writing
            gl.bind_framebuffer(gl::FRAMEBUFFER, Some(self.front_buffer.framebuffer));
            gl.viewport(0, 0, self.width, self.height);
            gl.scissor(0, 0, self.width, self.height);

            // Blend mode
            gl.enable(gl::BLEND);
            gl.blend_func(gl::SRC_ALPHA, gl::ONE);

            // Clear it
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(gl::COLOR_BUFFER_BIT);

            // Draw back buffer (memory)
            gl.use_program(Some(self.phosphor_program));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.phosphor_program, "intensity")
                    .as_ref(),
                background_intensity,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.phosphor_program, "saturation_point")
                    .as_ref(),
                display_settings.saturation_point,
            );

            gl.active_texture(gl::TEXTURE0);
            gl.bind_texture(gl::TEXTURE_2D, Some(self.back_buffer.texture));

            gl.bind_vertex_array(Some(self.phosphor_vertex_array));
            gl.draw_arrays(gl::TRIANGLES, 0, 3);

            // Draw lines
            gl.use_program(Some(self.beam_program));

            // points are stored in a texture
            gl.active_texture(gl::TEXTURE0);
            gl.bind_texture(gl::TEXTURE_2D, Some(self.beam_texture));

            gl.uniform_1_f32(
                gl.get_uniform_location(self.beam_program, "intensity")
                    .as_ref(),
                display_settings.intensity,
            );
            let mut n = points.len() as i32 * 2 - 1;
            if self.last_sample.filter(|_| display_settings.continue_from_last_sample).is_some() {
                n += 1;
            }
            gl.draw_arrays(gl::LINES, 0, n);

            std::mem::swap(&mut self.front_buffer, &mut self.back_buffer);

            // Bind main framebuffer
            gl.bind_framebuffer(gl::FRAMEBUFFER, None);
        }

        self.last_sample = points.last().copied();
    }

    pub fn destroy(&self, gl: &gl::Context) {
        unsafe {
            //gl.delete_buffer(self.beam_buffer);
            gl.delete_vertex_array(self.phosphor_vertex_array);
            gl.delete_program(self.beam_program);
            gl.delete_program(self.phosphor_program);
            gl.delete_texture(self.beam_texture);
        }
        self.front_buffer.destroy(gl);
        self.back_buffer.destroy(gl);
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
                gl::RGBA32F as _,
                width,
                height,
                0,
                gl::RGBA,
                gl::FLOAT,
                gl::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
            gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
        }

        // Bind to framebuffer
        unsafe {
            gl.bind_framebuffer(gl::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                Some(texture),
                0,
            );
            gl.draw_buffers(&[gl::COLOR_ATTACHMENT0]);

            // Clear it
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(gl::COLOR_BUFFER_BIT);
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

    pub fn destroy(&self, gl: &gl::Context) {
        unsafe {
            gl.delete_framebuffer(self.framebuffer);
            gl.delete_texture(self.texture);
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DisplaySettings {
    pub memory_time: f32,
    pub saturation_point: f32,
    pub intensity: f32,
    pub paused: bool,
    pub amplitude: f32,
    pub phosphor_color: [f32; 3],
    /// Remember the last sample from the previous paint() call and use it as the first vertex in
    /// the next?
    pub continue_from_last_sample: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            memory_time: 1e-6,
            saturation_point: 100.0,
            intensity: 0.1,
            paused: false,
            amplitude: 1.0,
            phosphor_color: [0.1, 1.0, 0.2],
            continue_from_last_sample: true,
        }
    }
}
