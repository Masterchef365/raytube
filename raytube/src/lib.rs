use egui_glow::glow as gl;
pub mod egui_wrapper;
pub mod phosphor;

use gl::HasContext;

pub(crate) fn create_program(gl: &gl::Context, shader_sources: &[(u32, &str)]) -> gl::Program {
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
