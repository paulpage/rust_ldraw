use glutin::event::{Event, WindowEvent, VirtualKeyCode, ElementState, MouseScrollDelta};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::{ContextBuilder, WindowedContext, PossiblyCurrent};
use cgmath::{Matrix3, Matrix4, Rad, Deg, Vector3, Point3};
use std::time::Instant;
use std::collections::HashMap;

mod graphics;
mod parser;

const VS_SRC: &'static [u8] = b"
#version 330
precision mediump float;

attribute vec3 position;
attribute vec3 normal;
attribute vec4 color;

uniform mat4 world;
uniform mat4 view;
uniform mat4 proj;

varying vec3 v_normal;
varying vec4 v_color;

void main() {
    mat4 worldview = view * world;
    v_normal = transpose(inverse(mat3(worldview))) * normal;
    v_color = color;
    gl_Position = proj * worldview * vec4(position, 1.0);
}
\0";

const FS_SRC: &'static [u8] = b"
#version 330
precision mediump float;

varying vec3 v_normal;
varying vec4 v_color;

const vec3 LIGHT = vec3(1.0, 1.0, 1.0);

void main() {
    float brightness = dot(normalize(v_normal), normalize(LIGHT));
    vec3 dark_color = v_color.xyz * 0.2;
    vec3 regular_color = v_color.xyz;

    gl_FragColor = vec4(mix(dark_color, regular_color, brightness), v_color.w);
}
\0";

struct Camera {
    position: Point3<f32>,
    focus: Point3<f32>,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Point3::new(4.0, 3.0, -3.0),
            focus: Point3::new(0.0, 0.0, 0.0),
        }
    }
}

struct State {
    rotation: Vector3<f32>,
    scale: f32,
    fovy: f32,
    near: f32,
    far: f32,
    camera: Camera,
}

impl State {
    fn new() -> Self {
        Self {
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale: 1.0,
            fovy: std::f32::consts::FRAC_PI_2 * 0.5,
            near: 0.01,
            far: 100.0,
            camera: Camera::new(),
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
    // let camera_relative: Vector3<f32> = Vector3::new(1.23, 2.52, 4.12);
    let rotation_mat = Matrix3::from_angle_y(Deg(state.rotation.y))
        * Matrix3::from_angle_x(Rad(state.rotation.x));
    let view = Matrix4::look_at(
        state.camera.position,
        state.camera.focus,
        Vector3::new(0.0, 1.0, 0.0),
        );
    let scale = Matrix4::from_scale(state.scale);

    let proj = cgmath::perspective(Rad(state.fovy), aspect, state.near, state.far);
    let world: Matrix4<f32> = Matrix4::from(rotation_mat).into();
    let view: Matrix4<f32> = (view * scale).into();
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
        ContextBuilder::new().build_windowed(window_builder, &event_loop).unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let mut state = State::new();

    let mut left = 0.0;
    let mut right = 0.0;
    let mut up = 0.0;
    let mut down = 0.0;
    let mut zoom_in = 0.0;
    let mut zoom_out = 0.0;

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

    let gl = graphics::init(
        &windowed_context.context(),
        VS_SRC,
        FS_SRC,
        &vertices,
    );

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        windowed_context.window().request_redraw();

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
                    if input.state == ElementState::Pressed {
                        match input.virtual_keycode {
                            Some(VirtualKeyCode::A) => {
                                state.rotation.y += 2.0;
                            }
                            Some(VirtualKeyCode::D) => {
                                state.rotation.y -= 2.0;
                            }
                            Some(VirtualKeyCode::R) => {
                                state.far += 1.0;
                            }
                            Some(VirtualKeyCode::F) => {
                                if state.far - 1.0 > state.near {
                                    state.far -= 1.0;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            state.scale *= (10.0 + y as f32) / 10.0;
                        }
                        MouseScrollDelta::PixelDelta(d) => {
                            state.scale *= (100.0 + d.y as f32) / 100.0;
                        }
                    }
                }
                _ => (),
            },
            Event::RedrawRequested(_) => {
                gl.draw_frame(mat_to_array(world), mat_to_array(view), mat_to_array(proj), [1.0, 0.5, 0.7, 1.0]);
                windowed_context.swap_buffers().unwrap();
            },
            _ => (),
        }
    });
}
