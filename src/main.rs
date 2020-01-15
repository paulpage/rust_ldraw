mod support;

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write, Result};
use std::path::PathBuf;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::device::{Device, DeviceExtensions};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, Subpass, RenderPassAbstract};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::viewport::Viewport;
use vulkano::sync::{self, GpuFuture, FlushError};
use vulkano::swapchain::{self, AcquireError, PresentMode, SurfaceTransform, Swapchain, SwapchainCreationError};
use vulkano_win::VkSurfaceBuild;
use winit::{EventsLoop, Window, WindowBuilder, Event, WindowEvent};

// TODO - actually use this
type LColor = u32;

#[derive(Clone)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

struct Transformation {
    x: f32,
    y: f32,
    z: f32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
    g: f32,
    h: f32,
    i: f32,
}

struct Polygon {
    vertices: Vec<Vec3>,
}

fn vertex_from(data: &Vec<&str>, x: usize, y: usize, z: usize) -> Vec3 {
    Vec3 {
        x: data[x].parse::<f32>().unwrap(),
        y: data[y].parse::<f32>().unwrap(),
        z: data[z].parse::<f32>().unwrap(),
    }
}

fn transform(p: &Vec3, t: &Transformation) -> Vec3 {
    Vec3 {
        x: t.a * p.x + t.b * p.y + t.c * p.z + t.x,
        y: t.d * p.x + t.e * p.y + t.f * p.z + t.y,
        z: t.g * p.x + t.h * p.y + t.i * p.z + t.z,
    }
}

fn determinant(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, g: f32, h: f32, i: f32) -> f32 {
    a*e*i + b*f*g + c*d*h - c*e*g - b*d*i - a*f*h
}

fn norm(p: &Polygon) -> Vec3 {
    let u = Vec3 {
        x: p.vertices[1].x - p.vertices[0].x,
        y: p.vertices[1].y - p.vertices[0].y,
        z: p.vertices[1].z - p.vertices[0].z
    };
    let v = Vec3 {
        x: p.vertices[2].x - p.vertices[0].x,
        y: p.vertices[2].y - p.vertices[0].y,
        z: p.vertices[2].z - p.vertices[0].z
    };
    Vec3 {
        x: (u.y * v.z - u.z * v.y),
        y: (u.x * v.z - u.z * v.x) * -1.0,
        z: (u.x * v.y - u.y * v.x),
    }
}

fn read_file(ldraw_directory: &str, filename: &str, inverted: bool) -> Vec<Polygon> {

    // TODO: Also allow current part's directory
    let filename = filename.to_lowercase();
    let paths: Vec<PathBuf> = vec!{
        PathBuf::new().join(ldraw_directory).join(&filename),
        PathBuf::new().join(ldraw_directory).join("parts").join(&filename),
        PathBuf::new().join(ldraw_directory).join("p").join(&filename),
        PathBuf::new().join(ldraw_directory).join("models").join(&filename),
        PathBuf::new().join(&filename),
    };
    let mut my_path = PathBuf::new();
    for path in paths {
        if path.exists() {
            my_path = path.clone();
            break;
        }
    }
    let input = File::open(my_path).unwrap();
    let input = BufReader::new(input);


    let mut polygons = Vec::new();

    let mut vertex_direction = "CCW";
    let mut invert_next = false;
    for line in input.lines() {
        let line = line.unwrap();
        // TODO: This is not correct because some whitespace (e.g. filenames)
        // should not be tokenized.
        let blocks: Vec<&str> = line.split_whitespace().collect();
        match blocks.len() {
            0..=2 => {}
            _ => {
                let command_type = blocks[0];
                let maybe_color = blocks[1];
                let data: Vec<&str> = blocks[2..].to_vec();
                match command_type {
                    "0" => {
                        if maybe_color == "BFC" {
                            if data[0] == "INVERTNEXT" {
                                invert_next = true;
                            }
                        }
                        if maybe_color == "BFC" && data.len() == 2 && data[0] == "CERTIFY" && data[1] == "CW" {
                            vertex_direction = "CW"; 
                        }
                    } // TODO 0 on the first line is the title
                    "1" => {
                        let mut invert_this = if invert_next { !inverted } else { inverted };
                        let t = Transformation {
                            x: data[0].parse::<f32>().unwrap(),
                            y: data[1].parse::<f32>().unwrap(),
                            z: data[2].parse::<f32>().unwrap(),
                            a: data[3].parse::<f32>().unwrap(),
                            b: data[4].parse::<f32>().unwrap(),
                            c: data[5].parse::<f32>().unwrap(),
                            d: data[6].parse::<f32>().unwrap(),
                            e: data[7].parse::<f32>().unwrap(),
                            f: data[8].parse::<f32>().unwrap(),
                            g: data[9].parse::<f32>().unwrap(),
                            h: data[10].parse::<f32>().unwrap(),
                            i: data[11].parse::<f32>().unwrap(),
                        };
                        if determinant(t.a, t.b, t.c, t.d, t.e, t.f, t.g, t.h, t.i) < 0.0 {
                            invert_this = !invert_this;
                        }
                        let sub_polygons = read_file(ldraw_directory, &str::replace(data[12], "\\", "/"), invert_this);
                        invert_next = false;
                        for polygon in sub_polygons {
                            let mut new_polygon = polygon;
                            new_polygon.vertices = new_polygon.vertices.iter().map(|v| transform(v, &t)).collect();
                            polygons.push(new_polygon);
                        }
                    }
                    "2" => {
                        // TODO line
                    }
                    "3" => {
                        match data.len() {
                            9 => {
                                let mut polygon = Polygon {
                                    vertices: Vec::new()
                                };
                                if (vertex_direction == "CW" && !inverted) || (vertex_direction == "CCW" && inverted) {
                                    polygon.vertices.push(vertex_from(&data, 6, 7, 8));
                                    polygon.vertices.push(vertex_from(&data, 3, 4, 5));
                                    polygon.vertices.push(vertex_from(&data, 0, 1, 2));
                                } else {
                                    polygon.vertices.push(vertex_from(&data, 0, 1, 2));
                                    polygon.vertices.push(vertex_from(&data, 3, 4, 5));
                                    polygon.vertices.push(vertex_from(&data, 6, 7, 8));
                                }
                                polygons.push(polygon);
                            }
                            _ => {} // TODO
                        }
                    }
                    "4" => {
                        match data.len() {
                            12 => {
                                let mut polygon = Polygon {
                                    vertices: Vec::new()
                                };
                                if (vertex_direction == "CW" && !inverted) || (vertex_direction == "CCW" && inverted) {
                                    polygon.vertices.push(vertex_from(&data, 9, 10, 11));
                                    polygon.vertices.push(vertex_from(&data, 6, 7, 8));
                                    polygon.vertices.push(vertex_from(&data, 3, 4, 5));
                                    polygon.vertices.push(vertex_from(&data, 0, 1, 2));
                                } else {
                                    polygon.vertices.push(vertex_from(&data, 0, 1, 2));
                                    polygon.vertices.push(vertex_from(&data, 3, 4, 5));
                                    polygon.vertices.push(vertex_from(&data, 6, 7, 8));
                                    polygon.vertices.push(vertex_from(&data, 9, 10, 11));
                                }
                                polygons.push(polygon);
                            }
                            _ => {} // TODO
                        }
                    }
                    "5" => {
                        // TODO optional line
                    }
                    _ => {}
                }
            }
        }
    }
    polygons
}

fn write_obj(polygons: &Vec<Polygon>, filename: &str) -> Result<()> {
    let output = OpenOptions::new().write(true).create(true).truncate(true).open(filename)?;
    let mut output = BufWriter::new(output);
    // TODO - Material doesn't seem to work, figure out why
    writeln!(output, "mtllib test.mtl")?;

    for p in polygons {
        for v in &p.vertices {
            writeln!(output, "v {} {} {}", v.x, v.y * -1.0, v.z)?;
        }
    }
    let norms: Vec<Vec3> = polygons.clone().iter().map(|p| norm(p)).collect();
    for n in norms {
        writeln!(output, "vn {} {} {}", n.x, n.y, n.z)?;
    }

    writeln!(output, "g thing")?;
    writeln!(output, "usemtl red")?;

    let mut vertex_count = 1;
    // writeln!(output, "s off");
    for (polygon_count, p) in polygons.iter().enumerate() {
        write!(output, "f")?;
        for _ in &p.vertices {
            write!(output, " {}/{}/{}", vertex_count, vertex_count, polygon_count + 1)?;
            vertex_count += 1;
        }
        writeln!(output, "")?;
    }
    Ok(())
}

mod vs {
    vulkano_shaders::shader!{
        ty: "vertex", // TODO what?
        src: "
        #version 450

        layout(location = 0) in vec2 position;

        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }"
    }
}

mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        src: "
        #version 450

        layout(location = 0) out vec4 f_color;

        void main() {
            f_color = vec4(1.0, 0.0, 0.0, 1.0);
        }"
    }
}

#[derive(Default, Debug, Clone)]
struct Vertex { position: [f32; 2] }

fn main() {

    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
    };
    let physical = PhysicalDevice::enumerate(&instance).next().unwrap();
    let mut event_loop = EventsLoop::new();
    let surface = WindowBuilder::new().build_vk_surface(&event_loop, instance.clone()).unwrap();
    let window = surface.window();
    let queue_family = physical.queue_families().find(|&q| {
        q.supports_graphics() && surface.is_supported(q).unwrap_or(false)
    }).unwrap();
    let device_ext = DeviceExtensions { khr_swapchain: true, .. DeviceExtensions::none() };
    let (device, mut queues) = Device::new(physical, physical.supported_features(), &device_ext, [(queue_family, 0.5)].iter().cloned()).unwrap();
    let queue = queues.next().unwrap();
    let (mut swapchain, images) = {
        let caps = surface.capabilities(physical).unwrap();
        let usage = caps.supported_usage_flags;
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;
        let initial_dimensions = if let Some(dimensions) = window.get_inner_size() {
            let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
            [dimensions.0, dimensions.1]
        } else {
            return;
        };
        Swapchain::new(device.clone(), surface.clone(), caps.min_image_count, format, initial_dimensions, 1, usage, &queue, SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None).unwrap()
    };


    let vertex_buffer = {
        vulkano::impl_vertex!(Vertex, position);
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), [
            Vertex { position: [-0.5, -0.25] },
            Vertex { position: [0.0, 0.5] },
            Vertex { position: [0.25, -0.1] },
        ].iter().cloned()).unwrap()
    };

    let vs = vs::Shader::load(device.clone()).expect("failed to create shader module");
    let fs = fs::Shader::load(device.clone()).expect("failed to create shader module");

    let render_pass = Arc::new(vulkano::single_pass_renderpass!(device.clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    ).unwrap());
    let pipeline = Arc::new(GraphicsPipeline::start()
        .vertex_input_single_buffer::<Vertex>()
        .vertex_shader(vs.main_entry_point(), ())
        .triangle_list()
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap());
    let mut dynamic_state = DynamicState { line_width: None, viewports: None, scissors: None, compare_mask: None, reference: None, write_mask: None };
    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    let mut recreate_swapchain = false;
    let mut previous_frame_end = Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>;
    loop {
        previous_frame_end.cleanup_finished();
        if recreate_swapchain {
            let dimensions = if let Some(dimensions) = window.get_inner_size() {
                let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
                [dimensions.0, dimensions.1]
            } else {
                return;
            };
            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => continue,
                Err(err) => panic!("{:?}", err)
            };
            swapchain = new_swapchain;
            framebuffers = window_size_dependent_setup(&new_images, render_pass.clone(), &mut dynamic_state);
            recreate_swapchain = false;
        }

        let (image_num, acquire_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                recreate_swapchain = true;
                continue;
            },
            Err(err) => panic!("{:?}", err)
        };


        let clear_values = vec!([0.0, 0.0, 1.0, 1.0].into());


        let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family()).unwrap()
            .begin_render_pass(framebuffers[image_num].clone(), false, clear_values)
            .unwrap()
            .draw(pipeline.clone(), &dynamic_state, vertex_buffer.clone(), (), ())
            .unwrap()
            .end_render_pass()
            .unwrap()
            .build().unwrap();
        let future = previous_frame_end.join(acquire_future)
            .then_execute(queue.clone(), command_buffer).unwrap()
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
            .then_signal_fence_and_flush();
        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
            Err(e) => {
                println!("{:?}", e);
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
        }


        let mut done = false;
        event_loop.poll_events(|ev| {
            match ev {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => done = true,
                Event::WindowEvent { event: WindowEvent::Resized(_), .. } => recreate_swapchain = true,
                _ => ()
            }
        });
        if done { return; }
    }
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0 .. 1.0,
    };
    dynamic_state.viewports = Some(vec!(viewport));

    images.iter().map(|image| {
        Arc::new(
            Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()
        ) as Arc<dyn FramebufferAbstract + Send + Sync>
    }).collect::<Vec<_>>()
}
