use cgmath::{Matrix3, Matrix4, Point3, Rad, Vector3};
use vulkano::buffer::cpu_pool::CpuBufferPool;
use std::iter;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, DeviceExtensions, Queue};
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
use winit::window::{Window, WindowBuilder};
use winit::event_loop::{EventLoop};
use std::sync::Arc;

#[derive(Default, Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}


pub mod vs {
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

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
        #version 450

        layout(location = 0) in vec3 v_normal;
        layout(location = 1) in vec4 v_color;
        layout(location = 0) out vec4 f_color;

        const vec3 LIGHT = vec3(0.8, 0.8, 0.8);

        void main() {
            float brightness = dot(normalize(v_normal), normalize(LIGHT));
            vec3 dark_color = v_color.xyz * 0.1;
            vec3 regular_color = v_color.xyz;

            f_color = vec4(mix(dark_color, regular_color, brightness), v_color.w);
        }"
    }
}

pub struct VulkanoRenderer {
    vertices: Vec<Vertex>,
    previous_frame_end: Box<dyn GpuFuture>,
    recreate_swapchain: bool,
    dimensions: [u32; 2],
    window: Window,
    device: Arc<Device>,
    vs: vs::Shader,
    fs: fs::Shader,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    uniform_buffer: CpuBufferPool<vs::ty::Data>,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    queue: Arc<Queue>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    swapchain: Arc<Swapchain<Window>>,
}

impl VulkanoRenderer {
    pub fn new(vertices: Vec<Vertex>, event_loop: &EventLoop<()>) -> Self {
        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).expect("failed to create Vulkan instance")
        };
        let physical = PhysicalDevice::enumerate(&instance).next().unwrap();
        let surface = WindowBuilder::new()
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();
        let window = surface.window();

        let mut dimensions = if let Some(dimensions) = window.get_inner_size() {
            let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
            [dimensions.0, dimensions.1]
        } else {
            panic!("Failed to set window dimensions.");
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
                panic!("Failed to set window dimensions.");
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

            ///////

            let mut previous_frame_end = Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>;

            VulkanoRenderer {
                vertices,
                previous_frame_end,
                recreate_swapchain,
                dimensions,
                swapchain,
                device,
                vs,
                fs,
                render_pass,
                pipeline,
                uniform_buffer,
                framebuffers,
                queue,
                vertex_buffer,
                window: *window,
            }
    }

    pub fn draw(&mut self, rotation: Vector3<f32>, d_rotation: Vector3<f32>, camera_position: Point3<f32>, camera_relative: Vector3<f32>, d_camera_position: Point3<f32>, d_camera_relative: Vector3<f32>) {
        self.previous_frame_end.cleanup_finished();
        if self.recreate_swapchain {
            self.dimensions = if let Some(dimensions) = self.window.get_inner_size() {
                let dimensions: (u32, u32) =
                    dimensions.to_physical(self.window.get_hidpi_factor()).into();
                [dimensions.0, dimensions.1]
            } else {
                return;
            };
            let (new_swapchain, new_images) = match self.swapchain.recreate_with_dimension(self.dimensions) {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => return,
                Err(err) => panic!("{:?}", err),
            };
            self.swapchain = new_swapchain;
            let (new_pipeline, new_framebuffers) = window_size_dependent_setup(
                self.device.clone(),
                &self.vs,
                &self.fs,
                &new_images,
                self.render_pass.clone(),
            );
            self.pipeline = new_pipeline;
            self.framebuffers = new_framebuffers;
            self.recreate_swapchain = false;
        }


        let uniform_buffer_subbuffer = {
            let rotation = Matrix3::from_angle_y(Rad(rotation.y + d_rotation.y))
                * Matrix3::from_angle_x(Rad(rotation.x + d_rotation.x));

            let aspect_ratio = self.dimensions[0] as f32 / self.dimensions[1] as f32;
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

            self.uniform_buffer.next(uniform_data).unwrap()
        };


        // TODO - When this breaks in a future release, use this version
        // let layout = pipeline.descriptor_set_layout(0).unwrap();
        // let set = Arc::new(PersistentDescriptorSet::start(pipeline.clone())
        let set = Arc::new(
            PersistentDescriptorSet::start(self.pipeline.clone(), 0)
                .add_buffer(uniform_buffer_subbuffer)
                .unwrap()
                .build()
                .unwrap(),
        );

        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(err) => panic!("{:?}", err),
            };

        let command_buffer =
            AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family())
                .unwrap()
                .begin_render_pass(
                    self.framebuffers[image_num].clone(),
                    false,
                    vec![[0.0, 1.0, 1.0, 1.0].into(), 1f32.into()], // TODO background color
                )
                .unwrap()
                .draw(
                    self.pipeline.clone(),
                    &DynamicState::none(),
                    vec![self.vertex_buffer.clone()],
                    set.clone(),
                    (),
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap();

        let future = &self.previous_frame_end
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)
            .then_signal_fence_and_flush();
        match future {
            Ok(future) => {
                future.wait(None).unwrap();
                self.previous_frame_end = future.boxed();
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Box::new(sync::now(self.device.clone())) as Box<_>;
            }
            Err(_) => {
                self.previous_frame_end = Box::new(sync::now(self.device.clone())) as Box<_>;
            }
        }
    }

    pub fn resize_window(&mut self) {
        self.recreate_swapchain = true;
    }
}

pub fn window_size_dependent_setup(
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
