use cgmath::{Point3, Vector3};
use std::time::Instant;
use std::f32::consts::PI;
use winit::event::{DeviceEvent, ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod parser;
use parser::{norm, read_file, LdrawColor};

mod renderer_vulkano;
use renderer_vulkano::{VulkanoRenderer, Vertex};

fn main() {
    let start = Instant::now();
    let polygons = read_file("/home/paul/Downloads/ldraw/", "3001.dat", false);
    println!(
        "Loaded {} polygons in {} ms.",
        polygons.len(),
        start.elapsed().as_millis()
    );

    let mut vertices = Vec::new();

    for polygon in &polygons {
        let color = match polygon.color {
            LdrawColor::RGBA(r, g, b, a) => [r, g, b, a],
            _ => [0.0, 1.0, 0.0, 1.0],
        };
        if polygon.points.len() == 3 {
            let n = norm(polygon);
            vertices.push(Vertex {
                position: [
                    polygon.points[0].x * 0.5,
                    polygon.points[0].y * 0.5,
                    polygon.points[0].z * 0.5,
                ],
                normal: [n.x, n.y, n.z],
                color: color,
            });
            vertices.push(Vertex {
                position: [
                    polygon.points[1].x * 0.5,
                    polygon.points[1].y * 0.5,
                    polygon.points[1].z * 0.5,
                ],
                normal: [n.x, n.y, n.z],
                color: color,
            });
            vertices.push(Vertex {
                position: [
                    polygon.points[2].x * 0.5,
                    polygon.points[2].y * 0.5,
                    polygon.points[2].z * 0.5,
                ],
                normal: [n.x, n.y, n.z],
                color: color,
            });
        }
    }

    ////////////////////

    let mut event_loop = EventLoop::new();
    let mut renderer = VulkanoRenderer::new(vertices, &event_loop);

    //////////////////////

    let mut rotation = Vector3::new(0.0, 0.0, 0.0);
    let mut d_rotation = Vector3::new(0.0, 0.0, 0.0);
    let mut camera_position = Point3::new(0.0, 0.0, 0.0);
    let mut camera_relative: Vector3<f32> = Vector3::new(1.23, 2.52, 4.12);
    let mut d_camera_position = Point3::new(0.0, 0.0, 0.0);
    let mut d_camera_relative = Vector3::new(0.0, 0.0, 0.0);

    let mut left = 0.0;
    let mut right = 0.0;
    let mut up = 0.0;
    let mut down = 0.0;
    let mut zoom_in = 0.0;
    let mut zoom_out = 0.0;

    loop {

        rotation.x += d_rotation.x;
        rotation.y += d_rotation.y;
        camera_position.x += d_camera_position.x;
        camera_position.y += d_camera_position.y;
        camera_position.z += d_camera_position.z;
        camera_relative.x += zoom_in - zoom_out;
        camera_relative.y += down - up;
        camera_relative.z += left - right;
        if camera_relative.x < 0.01 {
            camera_relative.x = 0.01;
        }
        if camera_relative.y < 0.001 {
            camera_relative.y = 0.001;
        }
        if camera_relative.y > PI - 0.001 {
            camera_relative.y = PI - 0.001;
        }

        let mut done = false;
        event_loop.run(move |ev, _, control_flow| {
            match ev {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => renderer.resize_window(),
                Event::DeviceEvent {
                    event: DeviceEvent::Key(s),
                    ..
                } => {
                    match s.state {
                        ElementState::Released => {
                            match s.scancode {
                                16 => zoom_out = 0.0,
                                17 => up = 0.0,
                                18 => zoom_in = 0.0,
                                30 => left = 0.0,
                                31 => down = 0.0,
                                32 => right = 0.0,
                                _ => {}
                            }
                        }
                        ElementState::Pressed => {
                            match s.scancode {
                                16 => zoom_out = 0.1,
                                17 => up = 0.1,
                                18 => zoom_in = 0.1,
                                30 => left = 0.1,
                                31 => down = 0.1,
                                32 => right = 0.1,
                                k => println!("Keycode: {}", k),
                            }
                        }
                    }
                }
                _ => (),
            }

            renderer.draw(rotation, d_rotation, camera_position, camera_relative, d_camera_position, d_camera_relative);
        });

    }
}
