use std::ffi::CString;
use std::{ptr, mem};
use glutin::{self, PossiblyCurrent};
use self::gl::types::*;
use rusttype::{point, Scale};

const VS_SRC_TEXT: &[u8] = b"
#version 330 core

layout (location = 0) in vec2 position;
layout (location = 1) in vec2 tex_coords;
layout (location = 2) in vec4 color;

out vec2 v_tex_coords;
out vec4 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_tex_coords = tex_coords;
    v_color = color;
}
\0";

const FS_SRC_TEXT: &[u8] = b"
#version 330 core

uniform sampler2D tex;
in vec2 v_tex_coords;
in vec4 v_color;
out vec4 f_color;

void main() {
    f_color = v_color * vec4(1.0, 1.0, 1.0, texture(tex, v_tex_coords).r);
}
\0";

pub struct Font {
    font: rusttype::Font<'static>,
}

impl<'a> Font {
    pub fn from_ttf_data(data: &'static [u8]) -> Self {
        Self {
            font: rusttype::Font::try_from_bytes(data as &[u8]).unwrap()
        }
    }

    pub fn draw_text(&self, gl: &gl::Gl, window_width: i32, window_height: i32, text: &str, x: f32, y: f32, scale: f32, color: [f32; 4]) {

        // Draw the font data into a buffer
        let font_scale = Scale::uniform(scale);
        let v_metrics = self.font.v_metrics(font_scale);
        let glyphs: Vec<_> = self.font
            .layout(text, font_scale, point(x, y + v_metrics.ascent))
            .collect();

        let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as usize;
        let glyphs_width = glyphs
            .iter()
            .rev()
            .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
            .next()
            .unwrap_or(0.0)
            .ceil() as usize;

        let mut buffer: Vec<f32> = vec![0.0; glyphs_width * glyphs_height];

        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {

                let min_x = bounding_box.min.x as usize;
                let min_y = bounding_box.min.y as usize;

                glyph.draw(|x, y, v| {
                    let x = x as usize + min_x - 1;
                    let y = y as usize + min_y - 1;
                    let index = y * glyphs_width + x;
                    buffer[index] = v;
                });
            }
        }

        // Load the texture from the buffer
        let (program, uniform, id) = unsafe {
            let mut id: u32 = 0;
            gl.GenTextures(1, &mut id);
            gl.ActiveTexture(gl::TEXTURE0);
            gl.BindTexture(gl::TEXTURE_2D, id);

            // TODO Decide what these should be.
            gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
            gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
            gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
            gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);

            gl.TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RED as GLint,
                glyphs_width as GLint,
                glyphs_height as GLint,
                0,
                gl::RED,
                gl::FLOAT,
                buffer.as_ptr() as *const _
            );
            let program = create_program(gl, VS_SRC_TEXT, FS_SRC_TEXT);
            let uniform = gl.GetUniformLocation(program, b"tex\0".as_ptr() as *const _);
            (program, uniform, id)
        };

        let height = glyphs_height as f32 * 2.0 / window_height as f32;
        let width = glyphs_width as f32 / window_width as f32;
        let vertices = [
            x, y, 0.0, 1.0, color[0], color[1], color[2], color[3],
            x + width, y, 1.0, 1.0, color[0], color[1], color[2], color[3],
            x + width, y + height, 1.0, 0.0, color[0], color[1], color[2], color[3],
            x, y, 0.0, 1.0, color[0], color[1], color[2], color[3],
            x + width, y + height, 1.0, 0.0, color[0], color[1], color[2], color[3],
            x, y + height, 0.0, 0.0, color[0], color[1], color[2], color[3],
        ];

        let (mut vao, mut vbo) = (0, 0);
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
            let stride = 8 * mem::size_of::<GLfloat>() as GLsizei;

            gl.ActiveTexture(gl::TEXTURE0);
            gl.BindTexture(gl::TEXTURE_2D, id);
            gl.Uniform1i(uniform, 0);

            gl.EnableVertexAttribArray(0);
            gl.VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl.EnableVertexAttribArray(1);
            gl.VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, (2 * mem::size_of::<GLfloat>()) as *const _);
            gl.EnableVertexAttribArray(2);
            gl.VertexAttribPointer(2, 4, gl::FLOAT, gl::FALSE, stride, (4 * mem::size_of::<GLfloat>()) as *const _);

            gl.UseProgram(program);

            gl.DrawArrays(gl::TRIANGLES, 0, vertices.len() as GLsizei);

            gl.BindBuffer(gl::ARRAY_BUFFER, 0);
            gl.BindVertexArray(0);
        }
    }
}

pub mod gl {
    pub use self::Gles2 as Gl;
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

pub struct Graphics {
    pub window_width: i32,
    pub window_height: i32,
    program: u32,
    program_2d: u32,
    vao_2d: u32,
    vertices_2d: Vec<f32>,
    pub gl: gl::Gl,
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
    window_width: i32,
    window_height: i32,
    vertex_shader: &'static [u8],
    fragment_shader: &'static [u8],
    vertex_shader_2d: &'static [u8],
    fragment_shader_2d: &'static [u8],
    vertices_2d: Vec<f32>,
) -> Graphics {

    let gl = gl::Gl::load_with(|ptr| gl_context.get_proc_address(ptr) as *const _);

    unsafe {
        gl.Enable(gl::DEPTH_TEST);
        gl.DepthFunc(gl::LESS);
        // gl.Disable(gl::CULL_FACE);
        gl.Enable(gl::BLEND);
        gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        gl.Viewport(0, 0, window_width, window_height);
    }

    let program = create_program(&gl, vertex_shader, fragment_shader);
    let program_2d = create_program(&gl, vertex_shader_2d, fragment_shader_2d);
    let (mut vao_2d, mut vbo_2d, mut vao_text, mut vbo_text) = (0, 0, 0, 0);
    unsafe {
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

        gl.GenVertexArrays(1, &mut vao_text);
        gl.GenBuffers(1, &mut vbo_text);
        gl.BindBuffer(gl::ARRAY_BUFFER, vbo_text); 
        Graphics {
            window_height,
            window_width,
            program,
            program_2d,
            vao_2d,
            // vertices,
            vertices_2d,
            gl,
        }
    }
}

impl Graphics {

    pub fn clear(&self, color: [f32; 4]) {
        unsafe {
            self.gl.ClearColor(color[0], color[1], color[2], color[3]);
            self.gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }

    // TODO remove this function, figure out what we want to do with drawing 2d
    pub fn draw_2d(&self) {
        unsafe {
            // 2d
            self.gl.UseProgram(self.program_2d);
            self.gl.BindVertexArray(self.vao_2d);
            self.gl.DrawArrays(gl::TRIANGLES, 0, self.vertices_2d.len() as GLsizei);
            self.gl.BindVertexArray(0);
        }
    }

    pub fn set_screen_size(&mut self, x: i32, y: i32) {
        unsafe {
            self.gl.Viewport(0, 0, x, y);
        }
        self.window_width = x;
        self.window_height = y;
    }

    pub fn draw_model(&self, vertices: &[f32], world: [f32; 16], view: [f32; 16], proj: [f32; 16], view_position: [f32; 3], light: [f32; 15]) {
        let gl = &self.gl;
        let (mut vao, mut vbo) = (0, 0);
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

            self.gl.Uniform3f(
                self.gl.GetUniformLocation(self.program, "view_position\0".as_ptr() as *const _),
                view_position[0],
                view_position[1],
                view_position[2]
            );
            self.gl.Uniform3f(
                self.gl.GetUniformLocation(self.program, "light.position\0".as_ptr() as *const _),
                light[0],
                light[1],
                light[2],
            );
            self.gl.Uniform3f(
                self.gl.GetUniformLocation(self.program, "light.direction\0".as_ptr() as *const _),
                light[3],
                light[4],
                light[5],
            );
            self.gl.Uniform3f(
                self.gl.GetUniformLocation(self.program, "light.ambient\0".as_ptr() as *const _),
                light[6],
                light[7],
                light[8],
            );
            self.gl.Uniform3f(
                self.gl.GetUniformLocation(self.program, "light.diffuse\0".as_ptr() as *const _),
                light[9],
                light[10],
                light[11],
            );
            self.gl.Uniform3f(
                self.gl.GetUniformLocation(self.program, "light.specular\0".as_ptr() as *const _),
                light[12],
                light[13],
                light[14],
            );

            self.gl.BindVertexArray(vao);
            self.gl.DrawArrays(gl::TRIANGLES, 0, vertices.len() as GLsizei);
            self.gl.BindVertexArray(0);
            self.gl.Disable(gl::DEPTH_TEST);
        }
    }
}
