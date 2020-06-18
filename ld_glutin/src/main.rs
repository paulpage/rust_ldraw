use glutin::event::{Event, WindowEvent, VirtualKeyCode, ElementState, MouseScrollDelta, MouseButton};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;
use cgmath::{Matrix4, Vector2, Deg, Vector3, Point3, SquareMatrix, Vector4};
use std::time::Instant;

mod graphics;
mod parser;

fn fmin(a: f32, b: f32) -> f32 {
    if b < a { b } else { a }
}

fn fmax(a: f32, b: f32) -> f32 {
    if b > a { b } else { a }
}

fn lerp(a: f32, b: f32, by: f32) -> f32 {
    a + (b - a) * by
}

struct Camera {
    focus: Point3<f32>,
    distance: f32,
    rot_horizontal: f32,
    rot_vertical: f32,
    fovy: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            focus: Point3::new(0.0, 0.0, 0.0),
            distance: 10.0,
            rot_horizontal: 0.5,
            rot_vertical: 0.5,
            fovy: 45.0,
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

    fn position(&self) -> Point3<f32> {
        Point3::new(
            self.focus.z + self.distance * self.rot_vertical.sin() * self.rot_horizontal.sin(),
            self.focus.y + self.distance * self.rot_vertical.cos(),
            self.focus.x + self.distance * self.rot_vertical.sin() * self.rot_horizontal.cos()
        )
    }
}

struct Model {
    vertices: Vec<f32>,
    position: Vector3<i32>,
    rotation: Vector3<i32>,
    animation_position_offset: Vector3<f32>,
    animation_rotation_offset: Vector3<f32>,
    bounding_box: BoundingBox,
}

struct BoundingBox {
    min: Point3<f32>,
    max: Point3<f32>,
}

struct State {
    fovy: f32,
    camera: Camera,
    aspect_ratio: f32,
    up_pressed: bool,
    down_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    mouse_x: f32,
    mouse_y: f32,
    middle_pressed: bool,
    active_model_idx: usize,
}

impl State {
    fn new() -> Self {
        Self {
            aspect_ratio: 1.0,
            fovy: 90.0,
            camera: Camera::new(),
            up_pressed: false,
            down_pressed: false,
            left_pressed: false,
            right_pressed: false,
            mouse_x: 0.0,
            mouse_y: 0.0,
            middle_pressed: false,
            active_model_idx: 0,
        }
    }
}

fn unproject(source: Vector3<f32>, view: Matrix4<f32>, proj: Matrix4<f32>) -> Vector3<f32> {
    let view_proj = (proj * view).invert().unwrap();
    let q = view_proj * Vector4::new(source.x, source.y, source.z, 1.0);
    Vector3::new(q.x / q.w, q.y / q.w, q.z / q.w)
}

fn get_mouse_ray(state: &State, mouse_position: Vector2<f32>, camera: &Camera) -> (Point3<f32>, Vector3<f32>) {
    let view = Matrix4::look_at(camera.position(), camera.focus, Vector3::new(0.0, 1.0, 0.0));
    let proj = cgmath::perspective(Deg(camera.fovy), state.aspect_ratio, 0.01, 100.0);
    let near = unproject(Vector3::new(mouse_position.x, mouse_position.y, 0.0), view, proj);
    let far = unproject(Vector3::new(mouse_position.x, mouse_position.y, 1.0), view, proj);
    let direction = far - near;
    (camera.position(), direction)
}

fn get_transforms(state: &State, model: &Model) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
    let view = Matrix4::look_at(
        state.camera.position(),
        state.camera.focus,
        Vector3::new(0.0, 1.0, 0.0)
    );
    let proj = cgmath::perspective(Deg(state.camera.fovy), state.aspect_ratio, 0.01, 100.0);
    let position = Vector3::new(model.position.x as f32 * 0.5, model.position.y as f32 * 0.2, model.position.z as f32 * 0.5);
    let world = Matrix4::from_translation(position - model.animation_position_offset)
        * Matrix4::from_angle_x(Deg((model.rotation.x * 90) as f32 - model.animation_rotation_offset.x))
        * Matrix4::from_angle_y(Deg((model.rotation.y * 90) as f32 - model.animation_rotation_offset.y))
        * Matrix4::from_angle_z(Deg((model.rotation.z * 90) as f32 - model.animation_rotation_offset.z));
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

fn load_ldraw_file(ldraw_dir: &str, filename: &str, custom_color: Option<[f32; 4]>) -> Model {
    let polygons = parser::load(ldraw_dir, filename);
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

    Model {
        vertices,
        position: Vector3::new(0, 0, 0),
        rotation: Vector3::new(0, 0, 0),
        animation_position_offset: Vector3::new(0.0, 0.0, 0.0),
        animation_rotation_offset: Vector3::new(0.0, 0.0, 0.0),
        bounding_box,
    }
}

fn main() {

    let ldraw_dir = "/home/paul/Downloads/ldraw";
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new().with_title("Bricks");
    let windowed_context =
        ContextBuilder::new().with_vsync(true).build_windowed(window_builder, &event_loop).unwrap();
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let baseplate = load_ldraw_file(ldraw_dir, "3811.dat", None);
    let mut state = State::new();
    let mut models = Vec::new();

    let start = Instant::now();
    models.push(load_ldraw_file(ldraw_dir, "car.ldr", None));

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

    let font = graphics::Font::from_ttf_data(include_bytes!("../data/LiberationSans-Regular.ttf"));

    let mut new_brick_position = Vector3::new(2, 2, 2);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

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
                        Some(VirtualKeyCode::A) => state.left_pressed = pressed,
                        Some(VirtualKeyCode::D) => state.right_pressed = pressed,
                        Some(VirtualKeyCode::W) => state.up_pressed = pressed,
                        Some(VirtualKeyCode::S) => state.down_pressed = pressed,
                        Some(VirtualKeyCode::T) => {
                            if pressed {
                                let mut model = load_ldraw_file(ldraw_dir, "3001.dat", Some([1.0, 0.0, 0.0, 1.0]));
                                model.position = new_brick_position;
                                new_brick_position.y += 3;
                                new_brick_position.z += 1;
                                models.push(model);
                                state.active_model_idx = models.len() - 1;
                            }
                        }
                        Some(VirtualKeyCode::R) => {
                            if pressed {
                                models[state.active_model_idx].rotation.y += 1;
                                models[state.active_model_idx].animation_rotation_offset.y = 90.0;
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
            Event::MainEventsCleared => {
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
                let (world, view, proj) = get_transforms(&state, &baseplate);
                gl.draw_model(&baseplate.vertices, mat_to_array(world), mat_to_array(view), mat_to_array(proj), view_position, light);
                // gl.draw_rect(0.0, 0.0, 0.4, 0.4, [0.0, 0.0, 1.0, 1.0]);
                for model in &mut models {
                    let (world, view, proj) = get_transforms(&state, &model);
                    gl.draw_model(&model.vertices, mat_to_array(world), mat_to_array(view), mat_to_array(proj), view_position, light);

                    if model.animation_rotation_offset.y.abs() > std::f32::EPSILON {
                        let direction = model.animation_rotation_offset.y / model.animation_rotation_offset.y.abs();
                        model.animation_rotation_offset.y -= 15.0 * direction;
                        if model.animation_rotation_offset.y < 15.0 {
                            model.animation_rotation_offset.y = 0.0;
                        }
                    }
                    let delta = model.animation_rotation_offset.y - model.rotation.y as f32;
                }
                // font.draw_text(&gl.gl, gl.window_width, gl.window_height, "Hello", -0.5, 0.0, 256.0, [1.0, 0.0, 0.5, 1.0]);
                windowed_context.swap_buffers().unwrap();
            },
            _ => (),
        }
    });
}
