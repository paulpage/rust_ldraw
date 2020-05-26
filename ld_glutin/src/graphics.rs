use std::ffi::{CString, CStr};
use std::{ptr, mem};
use glutin::{self, PossiblyCurrent};
use self::gl::types::*;

pub mod gl {
    pub use self::Gles2 as Gl;
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

pub struct Graphics {
    program: u32,
    program_2d: u32,
    vao: u32,
    vao_2d: u32,
    vertices: Vec<f32>,
    vertices_2d: Vec<f32>,
    gl: gl::Gl,
}

fn create_shader(gl: &gl::Gl, shader_type: u32, source: &'static [u8]) -> u32 {
    unsafe {
        let id = gl.CreateShader(shader_type);
        gl.ShaderSource(
            id,
            1,
            [source.as_ptr() as *const _].as_ptr(),
            std::ptr::null()
        );
        gl.CompileShader(id);
        let mut success: gl::types::GLint = 1;
        gl.GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
        if success == 0 {
            let mut len: gl::types::GLint = 0;
            gl.GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
            let error = {
                let mut buffer: Vec<u8> = Vec::with_capacity(len as usize + 1);
                buffer.extend([b' '].iter().cycle().take(len as usize));
                CString::from_vec_unchecked(buffer)
            };
            gl.GetShaderInfoLog(id, len, std::ptr::null_mut(), error.as_ptr() as *mut gl::types::GLchar);
            eprintln!("{}", error.to_string_lossy());
        }
        id
    }
}

fn create_program(
    gl: &gl::Gl,
    vertex_shader: &'static [u8],
    fragment_shader: &'static [u8],
) -> u32 {
    let vs = create_shader(gl, gl::VERTEX_SHADER, vertex_shader);
    let fs = create_shader(gl, gl::FRAGMENT_SHADER, fragment_shader);
    
    unsafe {
        let program = gl.CreateProgram();
        gl.AttachShader(program, vs);
        gl.AttachShader(program, fs);
        gl.LinkProgram(program);
        let mut success: gl::types::GLint = 1;
        gl.GetProgramiv(program, gl::LINK_STATUS, &mut success);
        if success == 0 {
            let mut len: gl::types::GLint = 0;
            gl.GetShaderiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let error = {
                let mut buffer: Vec<u8> = Vec::with_capacity(len as usize + 1);
                buffer.extend([b' '].iter().cycle().take(len as usize));
                CString::from_vec_unchecked(buffer)
            };
            gl.GetProgramInfoLog(program, len, std::ptr::null_mut(), error.as_ptr() as *mut gl::types::GLchar);
            eprintln!("{}", error.to_string_lossy());
        }
        gl.DeleteShader(vs);
        gl.DeleteShader(fs);
        program
    }
}

pub fn init(
    gl_context: &glutin::Context<PossiblyCurrent>,
    vertex_shader: &'static [u8],
    fragment_shader: &'static [u8],
    vertex_shader_2d: &'static [u8],
    fragment_shader_2d: &'static [u8],
    vertices: Vec<f32>,
    vertices_2d: Vec<f32>,
) -> Graphics {

    let gl = gl::Gl::load_with(|ptr| gl_context.get_proc_address(ptr) as *const _);

    unsafe {
        gl.Enable(gl::DEPTH_TEST);
        gl.DepthFunc(gl::LESS);
        gl.Disable(gl::CULL_FACE);
        gl.Enable(gl::BLEND);
        gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }

    let program = create_program(&gl, vertex_shader, fragment_shader);
    let program_2d = create_program(&gl, vertex_shader_2d, fragment_shader_2d);
    let (mut vao, mut vbo, mut vao_2d, mut vbo_2d) = (0, 0, 0, 0);
    unsafe {
        gl.GenVertexArrays(1, &mut vao);
        gl.GenBuffers(1, &mut vbo);
        gl.BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl.BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            vertices.as_ptr() as *const _,
            gl::STATIC_DRAW
        );
        gl.BindVertexArray(vao);
        let stride = 10 * mem::size_of::<GLfloat>() as GLsizei;
        gl.EnableVertexAttribArray(0);
        gl.VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, ptr::null());
        gl.EnableVertexAttribArray(1);
        gl.VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, stride, (3 * mem::size_of::<GLfloat>()) as *const _);
        gl.EnableVertexAttribArray(2);
        gl.VertexAttribPointer(2, 4, gl::FLOAT, gl::FALSE, stride, (6 * mem::size_of::<GLfloat>()) as *const _);
        gl.BindBuffer(gl::ARRAY_BUFFER, 0);
        gl.BindVertexArray(0);

        // TODO textures maybe

        gl.GenVertexArrays(1, &mut vao_2d);
        gl.GenBuffers(1, &mut vbo_2d);
        gl.BindBuffer(gl::ARRAY_BUFFER, vbo_2d);
        gl.BufferData(
            gl::ARRAY_BUFFER,
            (vertices_2d.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            vertices_2d.as_ptr() as *const _,
            gl::STATIC_DRAW
        );
        gl.BindVertexArray(vao_2d);
        let stride = 6 * mem::size_of::<GLfloat>() as GLsizei;
        gl.EnableVertexAttribArray(0);
        gl.VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, ptr::null());
        gl.EnableVertexAttribArray(1);
        gl.VertexAttribPointer(1, 4, gl::FLOAT, gl::FALSE, stride, (2 * mem::size_of::<GLfloat>()) as *const _);
        gl.BindBuffer(gl::ARRAY_BUFFER, 0);
        gl.BindVertexArray(0);

        Graphics {
            program,
            program_2d,
            vao,
            vao_2d,
            vertices,
            vertices_2d,
            gl,
        }
    }
}

impl Graphics {
    pub fn draw(&self, world: [f32; 16], view: [f32; 16], proj: [f32; 16]) {
        unsafe {
            self.gl.ClearColor(0.0, 1.0, 0.0, 1.0);
            self.gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            // 3d
            self.gl.Enable(gl::DEPTH_TEST);
            self.gl.UseProgram(self.program);
            self.gl.UniformMatrix4fv(
                self.gl.GetUniformLocation(self.program, b"world\0".as_ptr() as *const _),
                1,
                gl::FALSE,
                world.as_ptr()
            );
            self.gl.UniformMatrix4fv(
                self.gl.GetUniformLocation(self.program, b"view\0".as_ptr() as *const _),
                1,
                gl::FALSE,
                view.as_ptr()
            );
            self.gl.UniformMatrix4fv(
                self.gl.GetUniformLocation(self.program, "proj\0".as_ptr() as *const _),
                1,
                gl::FALSE,
                proj.as_ptr()
            );

            self.gl.BindVertexArray(self.vao);
            self.gl.DrawArrays(gl::TRIANGLES, 0, self.vertices.len() as GLsizei);
            self.gl.BindVertexArray(0);
            self.gl.Disable(gl::DEPTH_TEST);

            // 2d
            self.gl.UseProgram(self.program_2d);
            self.gl.BindVertexArray(self.vao_2d);
            self.gl.DrawArrays(gl::TRIANGLES, 0, self.vertices_2d.len() as GLsizei);
            self.gl.BindVertexArray(0);
        }
    }
}
