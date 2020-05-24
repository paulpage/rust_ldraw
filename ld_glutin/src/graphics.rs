use std::ffi::CString;
use glutin::{self, PossiblyCurrent};

pub mod gl {
    pub use self::Gles2 as Gl;
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

pub struct Gl {
    pub gl: gl::Gl,
    vertex_count: usize,
    program: u32,
}

fn create_shader(gl: &self::gl::Gles2, shader_type: u32, source: &'static [u8]) -> u32 {
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

pub fn init(
    gl_context: &glutin::Context<PossiblyCurrent>,
    vertex_shader: &'static [u8],
    fragment_shader: &'static [u8],
    vertex_data: &[f32]
) -> Gl {
    let gl = gl::Gl::load_with(|ptr| gl_context.get_proc_address(ptr) as *const _);


    let program = unsafe {

        // TODO is this effective?
        gl.Enable(gl::DEPTH_TEST);
        gl.DepthFunc(gl::LESS);

        let vs = create_shader(&gl, gl::VERTEX_SHADER, vertex_shader);
        let fs = create_shader(&gl, gl::FRAGMENT_SHADER, fragment_shader);

        let program = gl.CreateProgram();
        gl.AttachShader(program, vs);
        gl.AttachShader(program, fs);
        gl.LinkProgram(program);
        gl.UseProgram(program);

        let mut vb = std::mem::zeroed();
        gl.GenBuffers(1, &mut vb);
        gl.BindBuffer(gl::ARRAY_BUFFER, vb);
        gl.BufferData(
            gl::ARRAY_BUFFER,
            (vertex_data.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
            vertex_data.as_ptr() as *const _,
            gl::STATIC_DRAW
        );

        let pos_attrib = gl.GetAttribLocation(program, b"position\0".as_ptr() as *const _);
        let normal_attrib = gl.GetAttribLocation(program, b"normal\0".as_ptr() as *const _);
        let color_attrib = gl.GetAttribLocation(program, b"color\0".as_ptr() as *const _);
        gl.VertexAttribPointer(
            pos_attrib as gl::types::GLuint,
            3,
            gl::FLOAT,
            0,
            10 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            std::ptr::null()
        );
        gl.VertexAttribPointer(
            normal_attrib as gl::types::GLuint,
            3,
            gl::FLOAT,
            0,
            10 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            (3 * std::mem::size_of::<f32>()) as *const () as *const _
        );
        gl.VertexAttribPointer(
            color_attrib as gl::types::GLuint,
            4,
            gl::FLOAT,
            0,
            10 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            (6 * std::mem::size_of::<f32>()) as *const () as *const _
        );
        gl.EnableVertexAttribArray(pos_attrib as gl::types::GLuint);
        gl.EnableVertexAttribArray(normal_attrib as gl::types::GLuint);
        gl.EnableVertexAttribArray(color_attrib as gl::types::GLuint);

        program
    };

    Gl {
        gl: gl,
        vertex_count: vertex_data.len(),
        program: program,
    }
}

impl Gl {
    pub fn draw_frame(&self, world: [f32; 16], view: [f32; 16], proj: [f32; 16], color: [f32; 4]) {

        unsafe {
            let world_uniform = self.gl.GetUniformLocation(self.program, b"world\0".as_ptr() as *const _);
            let view_uniform = self.gl.GetUniformLocation(self.program, b"view\0".as_ptr() as *const _);
            let proj_uniform = self.gl.GetUniformLocation(self.program, b"proj\0".as_ptr() as *const _);
            self.gl.UniformMatrix4fv(world_uniform, 1, 0, world.as_ptr());
            self.gl.UniformMatrix4fv(view_uniform, 1, 0, view.as_ptr());
            self.gl.UniformMatrix4fv(proj_uniform, 1, 0, proj.as_ptr());
        }

        unsafe {
            self.gl.ClearColor(color[0], color[1], color[2], color[3]);
            self.gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            self.gl.DrawArrays(gl::TRIANGLES, 0, self.vertex_count as i32);
        }
    }
}
