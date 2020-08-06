use glutin::event::{Event, WindowEvent, ElementState, MouseScrollDelta};
use glutin::event::VirtualKeyCode as Key;
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;
use cgmath::{Matrix4, Deg, Vector3, Point3, SquareMatrix};
use std::time::Instant;

mod graphics;
use graphics::{BoundingBox, Camera, Graphics, Model};

mod parser;
use parser::Parser;

mod input;
use input::InputState;

fn fmin(a: f32, b: f32) -> f32 {
    if b < a { b } else { a }
}

fn fmax(a: f32, b: f32) -> f32 {
    if b > a { b } else { a }
}

struct State {
    camera: Camera,
    aspect_ratio: f32,
    active_model_idx: usize,
}

impl State {
    fn new() -> Self {
        Self {
            aspect_ratio: 1.0,
            camera: Camera::new(),
            active_model_idx: 0,
        }
    }
}

fn get_global_transforms(state: &State) -> (Matrix4<f32>, Matrix4<f32>) {
    let view = Matrix4::look_at(
        state.camera.position(),
        state.camera.focus,
        Vector3::new(0.0, 1.0, 0.0)
    );
    let proj = cgmath::perspective(Deg(state.camera.fovy), state.aspect_ratio, 0.01, 100.0);
    (view, proj)
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

fn load_ldraw_file(gl: &mut Graphics, parser: &mut Parser, filename: &str, custom_color: Option<[f32; 4]>) -> Model {
    let polygons = parser.load(filename);
    let mut vertices = Vec::new();
    let mut bounding_box = BoundingBox {
        min: Point3::new(f32::MAX, f32::MAX, f32::MAX),
        max: Point3::new(f32::MIN, f32::MIN, f32::MIN),
    };
    for polygon in &polygons {
        let mut color = match polygon.color {
            parser::LdrawColor::RGBA(r, g, b, a) => [r, g, b, a],
            _ => [0.0, 1.0, 0.0, 1.0],
        };
        if let Some(c) = custom_color {
            color = c;
        }

        if polygon.points.len() == 3 {
            let n = parser::norm(polygon);
            for point in &polygon.points {

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

                bounding_box.min.x = fmin(bounding_box.min.x, point.x / 40.0);
                bounding_box.min.y = fmin(bounding_box.min.y, point.y / -40.0);
                bounding_box.min.z = fmin(bounding_box.min.z, point.z / 40.0);
                bounding_box.max.x = fmax(bounding_box.max.x, point.x / 40.0);
                bounding_box.max.y = fmax(bounding_box.max.y, point.y / -40.0);
                bounding_box.max.z = fmax(bounding_box.max.z, point.z / 40.0);
            }
        }
    }

    let (vao, vertex_buffer_length) = gl.load_model(&vertices);

    Model {
        vao,
        vertex_buffer_length,
        // vertices,
        position: Vector3::new(0, 0, 0),
        rotation: Vector3::new(0, 0, 0),
        transform: Matrix4::identity(),
        position_offset: Vector3::new(0.0, 0.0, 0.0),
        rotation_offset: Vector3::new(0.0, 0.0, 0.0),
        bounding_box,
    }
}

fn main() {

    let mut parser = Parser::new("/home/paul/Downloads/ldraw");
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new().with_title("Bricks");
    let windowed_context =
        ContextBuilder::new().with_vsync(true).build_windowed(window_builder, &event_loop).unwrap();
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let mut state = State::new();
    let mut input_state = InputState::new();
    let mut models = Vec::new();

    let start = Instant::now();

    let mut new_position = Vector3::new(0, 0, 0);
    // // models.push(load_ldraw_file(ldraw_dir, "car.ldr", None));

    println!(
        "load time: {} ms",
        start.elapsed().as_millis(),
    );

    let size = windowed_context.window().inner_size();
    let mut gl: graphics::Graphics = graphics::init(
        &windowed_context.context(),
        size.width as i32,
        size.height as i32,
    );

    let baseplate = load_ldraw_file(&mut gl, &mut parser, "3811.dat", None);
    for x in 0..20 {
        for y in 0..20 {
            for z in 0..20 {
                let mut model = load_ldraw_file(&mut gl, &mut parser, "3005.dat", Some([1.0, 0.0, 0.0, 0.5]));
                model.position = new_position;
                new_position.x = x;
                new_position.y = y * 3;
                new_position.z = z;
                model.set_transform();
                models.push(model);
            }
        }
    }

    // let font = graphics::Font::from_ttf_data(include_bytes!("/usr/share/fonts/TTF/DejaVuSans.ttf"));

    let mut new_brick_position = Vector3::new(2, 2, 2);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        input_state.update(&event);

        if input_state.key_down(Key::A) {
            state.camera.rot_horizontal += 0.02;
        }
        if input_state.key_down(Key::D) {
            state.camera.rot_horizontal -= 0.02;
        }
        if input_state.key_down(Key::W) {
            state.camera.rot_vertical -= 0.02;
            if state.camera.rot_vertical < 0.001 {
                state.camera.rot_vertical = 0.001;
            }
        }
        if input_state.key_down(Key::S) {
            state.camera.rot_vertical += 0.02;
            if state.camera.rot_vertical > std::f32::consts::PI {
                state.camera.rot_vertical = std::f32::consts::PI - 0.001;
            }
        }

        match event {
            Event::LoopDestroyed => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    windowed_context.resize(physical_size);
                    gl.set_screen_size(physical_size.width as i32, physical_size.height as i32);
                    state.aspect_ratio = {
                        let size = windowed_context.window().inner_size();
                        size.width as f32 / size.height as f32
                    };
                }
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    let pressed = input.state == ElementState::Pressed;
                    match input.virtual_keycode {
                        // Some(VirtualKeyCode::A) => state.left_pressed = pressed,
                        // Some(VirtualKeyCode::D) => state.right_pressed = pressed,
                        // Some(VirtualKeyCode::W) => state.up_pressed = pressed,
                        // Some(VirtualKeyCode::S) => state.down_pressed = pressed,
                        Some(Key::T) => {
                            if pressed {
                                let mut model = load_ldraw_file(&mut gl, &mut parser, "3005.dat", Some([1.0, 0.0, 0.0, 1.0]));
                                model.position = new_brick_position;
                                new_brick_position.y += 3;
                                new_brick_position.z += 1;
                                model.set_transform();
                                models.push(model);
                                state.active_model_idx = models.len() - 1;
                            }
                        }
                        Some(Key::R) => {
                            if pressed {
                                models[state.active_model_idx].rotation.y += 1;
                                models[state.active_model_idx].rotation_offset.y = 90.0;
                                models[state.active_model_idx].set_transform();
                            }
                        }
                        _ => {}
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    match delta {
                        MouseScrollDelta::LineDelta(_x, y) => {
                            state.camera.distance *= (10.0 - y as f32) / 10.0;
                        }
                        MouseScrollDelta::PixelDelta(d) => {
                            state.camera.distance *= (100.0 - d.y as f32) / 100.0;
                        }
                    }
                }
                // WindowEvent::MouseInput { button, state: mouse_state, .. } => {
                //     let pressed = mouse_state == ElementState::Pressed;
                //     match button {
                //         MouseButton::Middle => state.middle_pressed = pressed,
                //         _ => {}
                //     }
                // }
                WindowEvent::CursorMoved { position, .. } => {
                    let dx = input_state.mouse_delta_x as f32;
                    let dy = input_state.mouse_delta_y as f32;
                    if input_state.mouse_middle_down {
                        state.camera.rotate(dx * -0.005, dy * -0.005);
                    }
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                let start = Instant::now();
                let p = state.camera.position();
                let view_position = [p.x, p.y, p.z];
                let light = [
                    -5.0, -5.0, -5.0,
                    1.0, 1.0, 1.0,

                    0.1, 0.1, 0.1,
                    0.8, 0.8, 0.8,
                    1.0, 1.0, 1.0,
                ];
                gl.clear([0.0, 1.0, 1.0, 1.0]);
                let (view, proj) = get_global_transforms(&state);
                gl.start_3d();
                gl.draw_model(baseplate.vao, baseplate.vertex_buffer_length, mat_to_array(baseplate.transform), mat_to_array(view), mat_to_array(proj), view_position, light);
                for model in &mut models {
                    gl.draw_model(model.vao, model.vertex_buffer_length,mat_to_array(model.transform), mat_to_array(view), mat_to_array(proj), view_position, light);

                    if model.rotation_offset.y.abs() > std::f32::EPSILON {
                        let direction = model.rotation_offset.y / model.rotation_offset.y.abs();
                        model.rotation_offset.y -= 15.0 * direction;
                        if model.rotation_offset.y < 15.0 {
                            model.rotation_offset.y = 0.0;
                        }
                        model.set_transform();
                    }
                }
                gl.draw_rect(0, 0, 100, 100, [0.0, 0.0, 0.0, 1.0]);
                gl.draw_text(
                    &format!("Frame time: {}", start.elapsed().as_millis()),
                    20, 20, 256.0, [1.0, 0.0, 0.5, 1.0]);
                windowed_context.swap_buffers().unwrap();
            },
            _ => (),
        }
    });
}
