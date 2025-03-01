use std::ffi::CString;
use std::str;

use gl::types::{GLchar, GLint, GLuint};
use nalgebra::{Matrix4, Vector2, Vector3, Vector4};

pub struct Shader {
    program: GLuint,
}

impl Shader {
    fn load_shader(source: &str, path: &str, typename: &str, type_: u32) -> GLuint {
        unsafe {
            let shader = gl::CreateShader(type_);
            gl::ShaderSource(shader, 1, &CString::new(source.as_bytes()).unwrap().as_ptr(), std::ptr::null());
            gl::CompileShader(shader);

            let mut log_length: GLint = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut log_length);

            let mut log: Vec<u8> = vec![0; log_length as usize];
            gl::GetShaderInfoLog(shader, log_length, std::ptr::null_mut(), log.as_mut_ptr() as *mut GLchar);

            let log = std::str::from_utf8(&log).unwrap();

            let mut success: GLint = 0;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);

            if success == gl::FALSE as GLint {
                gl::DeleteShader(shader);

                panic!(
                    "Failed to compile {} shader at: {}. Error: {}.",
                    typename,
                    path,
                    log
                );
            }

            shader
        }
    }
    fn delete_shaders(vertex_shader: GLuint, fragment_shader: GLuint) {
        unsafe {
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
        }
    }

    pub fn new(vertex_path: &str, fragment_path: &str) -> Self {
        let vertex_source = std::fs::read_to_string(vertex_path);
        if let Err(error) = vertex_source {
            panic!("Failed to read vertex shader source at: {}. Error: {}", vertex_path, error);
        }

        let fragment_source = std::fs::read_to_string(fragment_path);
        if let Err(error) = fragment_source {
            panic!("Failed to read fragment shader source at: {}. Error: {}", fragment_path, error);
        }

        unsafe {
            let vertex_shader = Self::load_shader(
                vertex_source.unwrap().as_str(),
                vertex_path,
                "vertex",
                gl::VERTEX_SHADER
            );
            let fragment_shader = Self::load_shader(
                fragment_source.unwrap().as_str(),
                fragment_path,
                "fragment",
                gl::FRAGMENT_SHADER
            );

            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);

            let mut log_length: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut log_length);

            let mut log: Vec<u8> = vec![0; log_length as usize];
            gl::GetProgramInfoLog(program, log_length, std::ptr::null_mut(), log.as_mut_ptr() as *mut GLchar);

            let log = std::str::from_utf8(&log).unwrap();

            let mut success: GLint = 0;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);

            if success == gl::FALSE as GLint {
                Self::delete_shaders(vertex_shader, fragment_shader);
                panic!(
                    "Failed to link program with shaders: Vertex({}), Fragment({}). Error: {}.",
                    vertex_path,
                    fragment_path,
                    log,
                );
            }

            Self::delete_shaders(vertex_shader, fragment_shader);
            Self { program }
        }
    }

    pub fn bind(&self) {
        unsafe { gl::UseProgram(self.program); }
    }
    pub fn unbind() {
        unsafe { gl::UseProgram(0); }
    }

    fn get_uniform_location(&self, name: &str) -> GLint {
        unsafe { gl::GetUniformLocation(self.program, CString::new(name).unwrap().as_ptr() as *const GLchar) }
    }

    pub fn set_int(&self, name: &str, value: i32) {
        unsafe { gl::Uniform1i(self.get_uniform_location(name), value); }
    }
    pub fn set_float(&self, name: &str, value: f32) {
        unsafe { gl::Uniform1f(self.get_uniform_location(name), value); }
    }
    pub fn set_vec2(&self, name: &str, value: &Vector2<f32>) {
        unsafe { gl::Uniform2f(self.get_uniform_location(name), value.x, value.y); }
    }
    pub fn set_ivec2(&self, name: &str, value: &Vector2<i32>) {
        unsafe { gl::Uniform2i(self.get_uniform_location(name), value.x, value.y); }
    }
    pub fn set_vec3(&self, name: &str, value: &Vector3<f32>) {
        unsafe { gl::Uniform3f(self.get_uniform_location(name), value.x, value.y, value.z); }
    }
    pub fn set_vec4(&self, name: &str, value: &Vector4<f32>) {
        unsafe { gl::Uniform4f(self.get_uniform_location(name), value.x, value.y, value.z, value.w); }
    }
    pub fn set_mat4(&self, name: &str, value: &Matrix4<f32>) {
        unsafe { gl::UniformMatrix4fv(self.get_uniform_location(name), 1, gl::FALSE, value.as_ptr()); }
    }
}
impl Drop for Shader {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.program); }
    }
}