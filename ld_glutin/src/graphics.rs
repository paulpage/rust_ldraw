use std::ffi::{CString, CStr, c_void};
use std::{ptr, mem};
use glutin::{self, PossiblyCurrent};
use self::gl::types::*;
use rusttype::gpu_cache::Cache;
use rusttype::{point, vector, PositionedGlyph, Rect, Scale};

const VS_SRC_TEXT: &'static [u8] = b"
#version 330 core

in vec2 position;
in vec2 tex_coords;
in vec4 color;

out vec2 v_tex_coords;
out vec4 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_tex_coords = tex_coords;
    v_color = color;
}
\0";

const FS_SRC_TEXT: &'static [u8] = b"
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

    pub fn draw_text(&self, gl: &gl::Gl, text: &str, x: f32, y: f32, scale: f32, color: [f32; 4]) {

        // Draw the font data into a buffer
        let scale = Scale::uniform(scale);
        let v_metrics = self.font.v_metrics(scale);
        let glyphs: Vec<_> = self.font
            .layout(text, scale, point(x, y + v_metrics.ascent))
            .collect();

        let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        let glyphs_width = {
            let min_x = glyphs
                .first()
                .map(|g| g.pixel_bounding_box().unwrap().min.x)
                .unwrap();
            let max_x = glyphs
                .last()
                .map(|g| g.pixel_bounding_box().unwrap().max.x)
                .unwrap();
            (max_x - min_x) as u32
        };

        let tex_width = glyphs_width as usize;
        let tex_height = glyphs_height as usize;

        let mut buffer = vec![0.0; tex_width * tex_height * 4];



        // let buffer = Vec::with_capacity((glyphs_width * glyphs_height * 4) as usize);
        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {

                let min_x = bounding_box.min.x as usize;
                let min_y = bounding_box.min.y as usize;

                glyph.draw(|x, y, v| {
                    let x = x as usize;
                    let y = y as usize;
                    let index = (y + min_y) * tex_height + x + min_x;
                    // let index = ((y as i32 + bounding_box.min.y) * glyphs_height as i32 + x + bounding_box.min.x) as usize;
                    buffer[index] = color[0];
                    buffer[index + 1] = color[1];
                    buffer[index + 2] = color[2];
                    buffer[index + 3] = v * color[3];
                });
            }
        }

        // Load the texture from the buffer
        let mut id: u32 = 0;
        let (program, uniform) = unsafe {
            gl.GenTextures(1, &mut id);
            gl.ActiveTexture(gl::TEXTURE0);
            gl.BindTexture(gl::TEXTURE_2D, id);
            gl.TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as GLint,
                tex_width as GLint,
                tex_height as GLint,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                buffer.as_ptr() as *const _
            );
            gl.BindTexture(gl::TEXTURE_2D, 0);

            let program = create_program(gl, VS_SRC_TEXT, FS_SRC_TEXT);

            let uniform = gl.GetUniformLocation(program, b"tex\0".as_ptr() as *const _);
            (program, uniform)
        };


        let twidth = glyphs_width as f32;
        let theight = glyphs_height as f32;
        let width = 1.0;
        let height = 1.0;
        let vertices = [
            x, y, 0.0, 0.0, color[0], color[1], color[2], color[3],
            x + width, y, twidth, 0.0, color[0], color[1], color[2], color[3],
            x + width, y + height, twidth, theight, color[0], color[1], color[2], color[3],
            x + width, y, twidth, 0.0, color[0], color[1], color[2], color[3],
            x + width, y + height, twidth, theight, color[0], color[1], color[2], color[3],
            x, y + height, 0.0, theight, color[0], color[1], color[2], color[3]
        ];
        // let vertices = [
        //     x, y, 0.0, 0.0, color[0], color[1], color[2], color[3],
        //     x + glyphs_width as f32, y, glyphs_width as f32, 0.0, color[0], color[1], color[2], color[3],
        //     x + glyphs_width as f32, y + glyphs_height as f32, glyphs_width as f32, glyphs_height as f32, color[0], color[1], color[2], color[3],
        //     x + glyphs_width as f32, y, glyphs_width as f32, 0.0, color[0], color[1], color[2], color[3],
        //     x + glyphs_width, y + glyphs_height, glyphs_width, glyphs_height, color[0], color[1], color[2], color[3],
        //     x, y + glyphs_height, 0.0, glyphs_height, color[0], color[1], color[2], color[3]
        // ];

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
            gl.EnableVertexAttribArray(0);
            gl.VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl.EnableVertexAttribArray(1);
            gl.VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, (2 * mem::size_of::<GLfloat>()) as *const _);
            gl.EnableVertexAttribArray(2);
            gl.VertexAttribPointer(2, 4, gl::FLOAT, gl::FALSE, stride, (4 * mem::size_of::<GLfloat>()) as *const _);

            gl.UseProgram(program);

            gl.ActiveTexture(gl::TEXTURE0);
            gl.BindTexture(gl::TEXTURE_2D, id);
            gl.Uniform1i(uniform, 0);

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
    program: u32,
    program_2d: u32,
    vao: u32,
    vao_2d: u32,
    vertices: Vec<f32>,
    vertices_2d: Vec<f32>,
    pub gl: gl::Gl,
}

fn load_texture(gl: &gl::Gl, data: &[u8], width: i32, height: i32) -> GLuint {
    let mut id: u32 = 0;
    unsafe {
        gl.GenTextures(1, &mut id);
        gl.BindTexture(gl::TEXTURE_2D, id);
        gl.TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as GLint,
            width,
            height,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            data.as_ptr() as *const _,
            );
        gl.BindTexture(gl::TEXTURE_2D, 0);
    }
    id
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
    text_texture_data: Vec<u8>,
) -> Graphics {

    let gl = gl::Gl::load_with(|ptr| gl_context.get_proc_address(ptr) as *const _);

    unsafe {
        gl.Enable(gl::DEPTH_TEST);
        gl.DepthFunc(gl::LESS);
        // gl.Disable(gl::CULL_FACE);
        gl.Enable(gl::BLEND);
        gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }

    let program = create_program(&gl, vertex_shader, fragment_shader);
    let program_2d = create_program(&gl, vertex_shader_2d, fragment_shader_2d);
    let program_text = create_program(&gl, VS_SRC_TEXT, VS_SRC_TEXT);
    let (mut vao, mut vbo, mut vao_2d, mut vbo_2d, mut vao_text, mut vbo_text) = (0, 0, 0, 0, 0, 0);
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

        gl.GenVertexArrays(1, &mut vao_text);
        gl.GenBuffers(1, &mut vbo_text);
        gl.BindBuffer(gl::ARRAY_BUFFER, vbo_text); 
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
    pub fn draw(&self, world: [f32; 16], view: [f32; 16], proj: [f32; 16], view_position: [f32; 3], light: [f32; 15]) {
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

            self.gl.BindVertexArray(self.vao);
            self.gl.DrawArrays(gl::TRIANGLES, 0, self.vertices.len() as GLsizei);
            self.gl.BindVertexArray(0);
            self.gl.Disable(gl::DEPTH_TEST);

            // 2d
            self.gl.UseProgram(self.program_2d);
            self.gl.BindVertexArray(self.vao_2d);
            // self.gl.DrawArrays(gl::TRIANGLES, 0, self.vertices_2d.len() as GLsizei);
            self.gl.BindVertexArray(0);
        }
    }
}
