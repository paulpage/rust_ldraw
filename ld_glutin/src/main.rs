use glutin::event::{Event, WindowEvent, VirtualKeyCode, ElementState, MouseScrollDelta, MouseButton};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::{ContextBuilder, WindowedContext, PossiblyCurrent};
use cgmath::{Matrix3, Matrix4, Rad, Deg, Vector3, Point3, SquareMatrix};
use std::time::Instant;
use std::collections::HashMap;

mod graphics;
mod parser;

const VS_SRC_2D: &'static [u8] = b"
#version 330 core

layout (location = 0) in vec2 position;
layout (location = 1) in vec4 color;

out vec4 v_color;

void main() {
    v_color = color;
    gl_Position = vec4(position, 0.0, 1.0);
}
\0";

const FS_SRC_2D: &'static [u8] = b"
#version 330 core

in vec4 v_color;

void main() {
    gl_FragColor = v_color;
}
\0";

const VS_SRC: &'static [u8] = b"
#version 330 core

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec4 color;

uniform mat4 world;
uniform mat4 view;
uniform mat4 proj;

out vec3 v_normal;
out vec4 v_color;

void main() {
    mat4 worldview = view * world;
    v_normal = transpose(inverse(mat3(worldview))) * normal;
    v_color = color;
    gl_Position = proj * worldview * vec4(position, 1.0);
}
\0";

const FS_SRC: &'static [u8] = b"
#version 330 core

in vec3 v_normal;
in vec4 v_color;

const vec3 LIGHT = vec3(1.0, 1.0, 1.0);

void main() {
    float brightness = dot(normalize(v_normal), normalize(LIGHT));
    vec3 dark_color = v_color.xyz * 0.2;
    vec3 regular_color = v_color.xyz;

    gl_FragColor = vec4(mix(dark_color, regular_color, brightness), v_color.w);
}
\0";

struct Camera {
    focus: Point3<f32>,
    distance: f32,
    rot_horizontal: f32,
    rot_vertical: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            focus: Point3::new(0.0, 0.0, 0.0),
            distance: 10.0,
            rot_horizontal: 0.5,
            rot_vertical: 0.5,
        }
    }

    fn rotate(&mut self, horizontal: f32, vertical: f32) {
        self.rot_horizontal += horizontal;
        self.rot_vertical += vertical;
        if self.rot_vertical < 0.001 {
            self.rot_vertical = 0.001;
        }
        if self.rot_vertical > std::f32::consts::PI {
            self.rot_vertical = std::f32::consts::PI - 0.001;
        }
    }
}

struct State {
    fovy: f32,
    near: f32,
    far: f32,
    camera: Camera,
    up_pressed: bool,
    down_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    mouse_x: f32,
    mouse_y: f32,
    middle_pressed: bool,
}

impl State {
    fn new() -> Self {
        Self {
            fovy: std::f32::consts::FRAC_PI_2 * 0.5,
            near: 0.01,
            far: 100.0,
            camera: Camera::new(),
            up_pressed: false,
            down_pressed: false,
            left_pressed: false,
            right_pressed: false,
            mouse_x: 0.0,
            mouse_y: 0.0,
            middle_pressed: false,
        }
    }
}

fn get_transforms(
    windowed_context: &WindowedContext<PossiblyCurrent>,
    state: &State
) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
    let aspect = {
        let size = windowed_context.window().inner_size();
        size.width as f32 / size.height as f32
    };
    let cam = &state.camera;
    let camera_position = Point3::new(
        cam.focus.z + cam.distance * cam.rot_vertical.sin() * cam.rot_horizontal.sin(),
        cam.focus.y + cam.distance * cam.rot_vertical.cos(),
        cam.focus.x + cam.distance * cam.rot_vertical.sin() * cam.rot_horizontal.cos()
    );
    let view = Matrix4::look_at(
        camera_position,
        state.camera.focus,
        Vector3::new(0.0, 1.0, 0.0),
        );

    let proj = cgmath::perspective(Rad(state.fovy), aspect, state.near, state.far);
    let world = Matrix4::identity();
    (world, view, proj)
}

fn mat_to_array(m: Matrix4<f32>) -> [f32; 16] {
    [
        // OpenGL is column-major by default
        m.x.x, m.x.y, m.x.z, m.x.w,
        m.y.x, m.y.y, m.y.z, m.y.w,
        m.z.x, m.z.y, m.z.z, m.z.w,
        m.w.x, m.w.y, m.w.z, m.w.w,
    ]
}

fn main() {
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new().with_title("A fantastic window!");

    let windowed_context =
        ContextBuilder::new().with_vsync(true).build_windowed(window_builder, &event_loop).unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let mut state = State::new();

    let mut x_min = f32::MAX;
    let mut y_min = f32::MAX;
    let mut z_min = f32::MAX;
    let mut x_max = f32::MIN;
    let mut y_max = f32::MIN;
    let mut z_max = f32::MIN;
    let start = Instant::now();
    let polygons = parser::load("/home/paul/Downloads/ldraw/", "car.ldr");
    let middle = Instant::now();
    let mut vertices = Vec::new();
    for polygon in &polygons {
        let color = match polygon.color {
            parser::LdrawColor::RGBA(r, g, b, a) => [r, g, b, a],
            _ => [0.0, 1.0, 0.0, 1.0],
        };
        if polygon.points.len() == 3 {
            let n = parser::norm(polygon);
            for point in &polygon.points {

                if point.x < x_min {
                    x_min = point.x;
                }
                if point.x > x_max {
                    x_max = point.x;
                }
                if point.y < y_min {
                    y_min = point.y;
                }
                if point.y > y_max {
                    y_max = point.y;
                }
                if point.z < z_min {
                    z_min = point.z;
                }
                if point.z > z_max {
                    z_max = point.z;
                }

                vertices.push(point.x / 40.0);
                vertices.push(point.y / -40.0);
                vertices.push(point.z / 40.0);
                vertices.push(n.x);
                vertices.push(n.y);
                vertices.push(n.z);
                // TODO Might have to sort transparent faces
                vertices.push(color[0]);
                vertices.push(color[1]);
                vertices.push(color[2]);
                vertices.push(color[3]);
            }
        }
    }
    println!(
        "Total load time: {} ms (bake time: {} ms)",
        start.elapsed().as_millis(),
        middle.elapsed().as_millis()
    );

    let vertices_2d = vec![
        -0.5, -0.5, 1.0, 0.0, 0.0, 1.0,
        0.5, -0.5, 1.0, 0.0, 0.0, 1.0,
        0.5, 0.5, 1.0, 0.0, 0.0, 1.0,
    ];

    let gl: graphics::Graphics = graphics::init(
        &windowed_context.context(),
        VS_SRC,
        FS_SRC,
        VS_SRC_2D,
        FS_SRC_2D,
        vertices,
        vertices_2d
    );

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        windowed_context.window().request_redraw();

        if state.left_pressed {
            state.camera.rot_horizontal += 0.02;
        }
        if state.right_pressed {
            state.camera.rot_horizontal -= 0.02;
        }
        if state.up_pressed {
            state.camera.rot_vertical -= 0.02;
            if state.camera.rot_vertical < 0.001 {
                state.camera.rot_vertical = 0.001;
            }
        }
        if state.down_pressed {
            state.camera.rot_vertical += 0.02;
            if state.camera.rot_vertical > std::f32::consts::PI {
                state.camera.rot_vertical = std::f32::consts::PI - 0.001;
            }
        }
        let (world, view, proj) = get_transforms(&windowed_context, &state);

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    windowed_context.resize(physical_size)
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    let pressed = input.state == ElementState::Pressed;
                    match input.virtual_keycode {
                        Some(VirtualKeyCode::A) => state.left_pressed = pressed,
                        Some(VirtualKeyCode::D) => state.right_pressed = pressed,
                        Some(VirtualKeyCode::W) => state.up_pressed = pressed,
                        Some(VirtualKeyCode::S) => state.down_pressed = pressed,
                        _ => {}
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            state.camera.distance *= (10.0 - y as f32) / 10.0;
                        }
                        MouseScrollDelta::PixelDelta(d) => {
                            state.camera.distance *= (100.0 - d.y as f32) / 100.0;
                        }
                    }
                }
                WindowEvent::MouseInput { button, state: mouse_state, .. } => {
                    let pressed = mouse_state == ElementState::Pressed;
                    match button {
                        MouseButton::Middle => state.middle_pressed = pressed,
                        _ => {}
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let dx = position.x as f32 - state.mouse_x;
                    let dy = position.y as f32 - state.mouse_y;
                    if state.middle_pressed {
                        state.camera.rotate(dx * -0.005, dy * -0.005);
                    }
                    state.mouse_x = position.x as f32;
                    state.mouse_y = position.y as f32;
                }
                _ => (),
            },
            Event::RedrawRequested(_) => {
                gl.draw(mat_to_array(world), mat_to_array(view), mat_to_array(proj));
                windowed_context.swap_buffers().unwrap();
            },
            _ => (),
        }
    });
}
