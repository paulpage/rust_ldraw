use cgmath::{Matrix3, Matrix4, Point3, Rad, Vector3};
use std::iter;
use std::sync::Arc;
use std::time::Instant;
use std::f32::consts::PI;
use vulkano::buffer::cpu_pool::CpuBufferPool;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, DeviceExtensions};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::attachment::AttachmentImage;
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::swapchain::{
    self, AcquireError, PresentMode, SurfaceTransform, Swapchain, SwapchainCreationError,
};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano_win::VkSurfaceBuild;
use winit::{DeviceEvent, ElementState, Event, EventsLoop, Window, WindowBuilder, WindowEvent};

mod parser;
use parser::{norm, read_file, LdrawColor};

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
        #version 450

        layout(location = 0) in vec3 position;
        layout(location = 1) in vec3 normal;
        layout(location = 2) in vec4 color;

        layout(location = 0) out vec3 v_normal;
        layout(location = 1) out vec4 v_color;

        layout(set = 0, binding = 0) uniform Data {
            mat4 world;
            mat4 view;
            mat4 proj;
        } uniforms;

        void main() {
            mat4 worldview = uniforms.view * uniforms.world;
            v_normal = transpose(inverse(mat3(worldview))) * normal;
            v_color = color;
            gl_Position = uniforms.proj * worldview * vec4(position, 1.0);
        }"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
        #version 450

        layout(location = 0) in vec3 v_normal;
        layout(location = 1) in vec4 v_color;
        layout(location = 0) out vec4 f_color;

        const vec3 LIGHT = vec3(0.0, 0.0, 1.0);

        void main() {
            float brightness = dot(normalize(v_normal), normalize(LIGHT));
            vec3 dark_color = vec3(0.6, 0.0, 0.0);
            vec3 regular_color = v_color.xyz;

            f_color = vec4(mix(dark_color, regular_color, brightness), v_color.w);
        }"
    }
}

#[derive(Default, Debug, Clone)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
}

fn main() {
    let start = Instant::now();
    let polygons = read_file("/home/paul/Downloads/ldraw/", "car.ldr", false);
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

    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
    };
    let physical = PhysicalDevice::enumerate(&instance).next().unwrap();
    let mut event_loop = EventsLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();
    let window = surface.window();

    let mut dimensions = if let Some(dimensions) = window.get_inner_size() {
        let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
        [dimensions.0, dimensions.1]
    } else {
        return;
    };

    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
        .unwrap();
    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (device, mut queues) = Device::new(
        physical,
        physical.supported_features(),
        &device_ext,
        [(queue_family, 0.5)].iter().cloned(),
    )
    .unwrap();
    let queue = queues.next().unwrap();
    let (mut swapchain, images) = {
        let caps = surface.capabilities(physical).unwrap();
        let usage = caps.supported_usage_flags;
        let format = caps.supported_formats[0].0;
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let initial_dimensions = if let Some(dimensions) = window.get_inner_size() {
            let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
            [dimensions.0, dimensions.1]
        } else {
            return;
        };
        Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            initial_dimensions,
            1,
            usage,
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            true,
            None,
        )
        .unwrap()
    };

    let vertex_buffer = {
        vulkano::impl_vertex!(Vertex, position, normal, color);
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), vertices.iter().cloned())
            .unwrap()
    };
    let uniform_buffer = CpuBufferPool::<vs::ty::Data>::new(device.clone(), BufferUsage::all());

    let vs = vs::Shader::load(device.clone()).expect("failed to create shader module");
    let fs = fs::Shader::load(device.clone()).expect("failed to create shader module");

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16Unorm,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )
        .unwrap(),
    );
    let (mut pipeline, mut framebuffers) =
        window_size_dependent_setup(device.clone(), &vs, &fs, &images, render_pass.clone());
    let mut recreate_swapchain = false;

    let mut rotation = Vector3::new(0.0, 0.0, 0.0);
    let mut d_rotation = Vector3::new(0.0, 0.0, 0.0);
    let mut camera_position = Point3::new(0.0, 0.0, 0.0);
    // let mut camera_position = Point3::new(-0.4, -0.6, -1.0);
    let mut camera_relative: Vector3<f32> = Vector3::new(1.23, 2.52, 4.12);
    let mut d_camera_position = Point3::new(0.0, 0.0, 0.0);
    let mut d_camera_relative = Vector3::new(0.0, 0.0, 0.0);

    let mut previous_frame_end = Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>;
    loop {
        previous_frame_end.cleanup_finished();
        if recreate_swapchain {
            dimensions = if let Some(dimensions) = window.get_inner_size() {
                let dimensions: (u32, u32) =
                    dimensions.to_physical(window.get_hidpi_factor()).into();
                [dimensions.0, dimensions.1]
            } else {
                return;
            };
            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => continue,
                Err(err) => panic!("{:?}", err),
            };
            swapchain = new_swapchain;
            let (new_pipeline, new_framebuffers) = window_size_dependent_setup(
                device.clone(),
                &vs,
                &fs,
                &new_images,
                render_pass.clone(),
            );
            pipeline = new_pipeline;
            framebuffers= new_framebuffers;
            recreate_swapchain = false;
        }

        rotation.x += d_rotation.x;
        rotation.y += d_rotation.y;
        camera_position.x += d_camera_position.x;
        camera_position.y += d_camera_position.y;
        camera_position.z += d_camera_position.z;
        camera_relative.x += d_camera_relative.x;
        camera_relative.y += d_camera_relative.y;
        camera_relative.z += d_camera_relative.z;
        if camera_relative.x < 0.01 {
            camera_relative.x = 0.01;
        }
        if camera_relative.y < 0.001 {
            camera_relative.y = 0.001;
        }
        if camera_relative.y > PI - 0.001 {
            camera_relative.y = PI - 0.001;
        }
        let uniform_buffer_subbuffer = {
            let rotation = Matrix3::from_angle_y(Rad(rotation.y + d_rotation.y))
                * Matrix3::from_angle_x(Rad(rotation.x + d_rotation.x));

            let aspect_ratio = dimensions[0] as f32 / dimensions[1] as f32;
            let proj =
                cgmath::perspective(Rad(std::f32::consts::FRAC_PI_2), aspect_ratio, 0.01, 100.0);
            let z = camera_relative.x * camera_relative.y.sin() * camera_relative.z.cos();
            let x = camera_relative.x * camera_relative.y.sin() * camera_relative.z.sin();
            let y = camera_relative.x * camera_relative.y.cos();
            let mut eye = camera_position;
            eye.x += x;
            eye.y += y;
            eye.z += z;
            let view = Matrix4::look_at(
                eye,
                camera_position,
                Vector3::new(0.0, 1.0, 0.0),
            );
            let scale = Matrix4::from_scale(0.01);

            let uniform_data = vs::ty::Data {
                world: Matrix4::from(rotation).into(),
                view: (view * scale).into(),
                proj: proj.into(),
            };

            uniform_buffer.next(uniform_data).unwrap()
        };

        // TODO - When this breaks in a future release, use this version
        // let layout = pipeline.descriptor_set_layout(0).unwrap();
        // let set = Arc::new(PersistentDescriptorSet::start(pipeline.clone())
        let set = Arc::new(
            PersistentDescriptorSet::start(pipeline.clone(), 0)
                .add_buffer(uniform_buffer_subbuffer)
                .unwrap()
                .build()
                .unwrap(),
        );

        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    recreate_swapchain = true;
                    continue;
                }
                Err(err) => panic!("{:?}", err),
            };

        let command_buffer =
            AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
                .unwrap()
                .begin_render_pass(
                    framebuffers[image_num].clone(),
                    false,
                    vec![[0.0, 1.0, 1.0, 1.0].into(), 1f32.into()], // TODO background color
                )
                .unwrap()
                .draw(
                    pipeline.clone(),
                    &DynamicState::none(),
                    vec![vertex_buffer.clone()],
                    set.clone(),
                    (),
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap();

        let future = previous_frame_end
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
            .then_signal_fence_and_flush();
        match future {
            Ok(future) => {
                future.wait(None).unwrap();
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
            Err(_) => {
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
        }

        let mut done = false;
        event_loop.poll_events(|ev| {
            match ev {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => done = true,
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => recreate_swapchain = true,
                Event::DeviceEvent {
                    event: DeviceEvent::Key(s),
                    ..
                } => {
                    match s.state {
                        ElementState::Released => {
                            match s.scancode {
                                16 => d_camera_relative.x = 0.0,
                                17 => d_camera_relative.y = 0.0,
                                18 => d_camera_relative.x = 0.0,
                                30 => d_camera_relative.z = 0.0,
                                31 => d_camera_relative.y = 0.0,
                                32 => d_camera_relative.z = 0.0,
                                // 103 => d_camera_position.z = 0.0,
                                // 106 => d_camera_position.x = 0.0,
                                // 108 => d_camera_position.z = 0.0,
                                // 105 => d_camera_position.x = 0.0,
                                // 103 => d_rotation.x = 0.0, // up
                                // 106 => d_rotation.y = 0.0, // right
                                // 108 => d_rotation.x = 0.0, // down
                                // 105 => d_rotation.y = 0.0, // left
                                _ => {}
                            }
                        }
                        ElementState::Pressed => {
                            match s.scancode {
                                16 => d_camera_relative.x += 0.1,
                                17 => d_camera_relative.y += 0.1,
                                18 => d_camera_relative.x -= 0.1,
                                30 => d_camera_relative.z -= 0.1,
                                31 => d_camera_relative.y -= 0.1,
                                32 => d_camera_relative.z += 0.1,
                                // 103 => d_camera_position.z += 0.1,
                                // 106 => d_camera_position.x += 0.1,
                                // 108 => d_camera_position.z -= 0.1,
                                // 105 => d_camera_position.x -= 0.1,
                                // 103 => d_rotation.x = 0.2,  // up
                                // 106 => d_rotation.y = 0.2,  // right
                                // 108 => d_rotation.x = -0.2, // down
                                // 105 => d_rotation.y = -0.2, // left
                                k => println!("Keycode: {}", k),
                            }
                        }
                    }
                }
                _ => (),
            }
        });
        if done {
            return;
        }
    }
}

fn window_size_dependent_setup(
    device: Arc<Device>,
    vs: &vs::Shader,
    fs: &fs::Shader,
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
) -> (
    Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
) {
    let dimensions = images[0].dimensions();
    let depth_buffer =
        AttachmentImage::transient(device.clone(), dimensions, Format::D16Unorm).unwrap();

    let framebuffers = images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .add(depth_buffer.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>();

    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input(SingleBufferDefinition::<Vertex>::new())
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .viewports(iter::once(Viewport {
                origin: [0.0, 0.0],
                dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                depth_range: 0.0..1.0,
            }))
            .fragment_shader(fs.main_entry_point(), ())
            .depth_stencil_simple_depth()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );
    (pipeline, framebuffers)
}
