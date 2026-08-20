use gui::gpu::pipelines::planet::{
    camera::Camera,
    shapes::ShapesPipeline,
    text::TextPipeline,
    uniforms::Uniforms,
};
use gui::model::shapes::Shapes;
use gui::text::{self, TEXT_VERTEX_FLOATS};
use gui::viewer::{ArrowAction, CameraControl, subscription};
use iced::mouse;
use iced::wgpu;
use iced::widget::{column, container, shader, text as ui_text};
use iced::{Element, Length};
use nalgebra::{Point3, Vector3};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Event(iced::event::Event),
    Shot(gui::screenshot::ShotMessage),
}

struct ShapesSimulation {
    camera: Camera,
    control: CameraControl,
    shape_vertices: Vec<[f32; 7]>,
    shape_ranges: Vec<(u32, u32)>,
    text_quads: Vec<[f32; TEXT_VERTEX_FLOATS]>,
    help_text: String,
    shot: gui::screenshot::AutoShot,
}

impl ShapesSimulation {
    fn new() -> Self {
        let mut shapes = Shapes::new();
        shapes.add_frame(
            gui::model::FrameMode::Eci,
            [-6.0, 0.0, 0.0],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            2.0,
            "Axes",
        );
        shapes.add_orbital_elements(3.5, 35.0, 20.0, 55.0);

        let (shape_vertices, shape_ranges, text_quads) = shapes.get_all();

        let mut text_quads = text_quads;
        text_quads.extend(create_sample_text_quads());

        let camera_eye = Point3::new(-6.0, -12.0, 4.0);
        let camera_target = Point3::new(-6.0, 0.0, 0.0);
        let mut camera = Camera::new(camera_eye, camera_target, 1600.0, 900.0);
        camera.fovy = 50.0;

        let mut control = CameraControl::default();
        control.arrow_action = ArrowAction::Pan;
        control.wheel_zoom_fraction = 0.1;
        control.pan_amount = 0.75;
        control.drag_sensitivity = 0.005 * 0.07;
        control.wheel_pixel_divisor = 10.0;
        control.wheel_sign = -1.0;

        ShapesSimulation {
            camera,
            control,
            shape_vertices,
            shape_ranges,
            text_quads,
            help_text: "Shapes: Pan arrows | Rotate right-drag | Zoom +/-".to_string(),
            shot: gui::screenshot::AutoShot::from_env(),
        }
    }
}

#[derive(Debug)]
struct TextVerticesPrimitive {
    shape_vertices: Vec<[f32; 7]>,
    shape_ranges: Vec<(u32, u32)>,
    text_quads: Vec<[f32; TEXT_VERTEX_FLOATS]>,
    camera: Camera,
    uniforms_state: Arc<Mutex<Option<UniformsState>>>,
}

#[derive(Debug)]
struct UniformsState {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

struct ShapesPipelineRenderer {
    pipeline: ShapesPipeline,
    text_pipeline: TextPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    depth_texture: Option<wgpu::Texture>,
}

impl ShapesPipelineRenderer {
    fn prepare_depth_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            self.depth_texture = None;
            return;
        }

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Vertices Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gui::gpu::pipelines::planet::consts::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        self.depth_texture = Some(depth_texture);
    }
}

impl shader::Pipeline for ShapesPipelineRenderer {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Text Vertices Uniforms bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        ShapesPipelineRenderer {
            pipeline: ShapesPipeline::new(device, format, &uniform_bind_group_layout, 1),
            text_pipeline: TextPipeline::new(device, queue, format, 1),
            uniform_bind_group_layout,
            depth_texture: None,
        }
    }
}

impl shader::Program<Message> for ShapesSimulation {
    type State = ();
    type Primitive = TextVerticesPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: iced::Rectangle,
    ) -> Self::Primitive {
        TextVerticesPrimitive {
            shape_vertices: self.shape_vertices.clone(),
            shape_ranges: self.shape_ranges.clone(),
            text_quads: self.text_quads.clone(),
            camera: self.camera.clone(),
            uniforms_state: Arc::new(Mutex::new(None)),
        }
    }
}

fn update(sim: &mut ShapesSimulation, message: Message) -> iced::Task<Message> {
    match message {
        Message::Tick => {
            if let Some(task) = sim.shot.on_frame() {
                return task.map(Message::Shot);
            }
        }
        Message::Event(event) => {
            sim.control.handle_event(&event, &mut sim.camera);
        }
        Message::Shot(msg) => {
            if let Some(task) = sim.shot.handle(msg) {
                return task.map(Message::Shot);
            }
        }
    }
    iced::Task::none()
}

fn view(sim: &ShapesSimulation) -> Element<'_, Message> {
    let scene = shader(sim).width(Length::Fill).height(Length::Fill);
    let info = ui_text(&sim.help_text).size(16);

    container(column![info, scene].spacing(6))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(10)
        .into()
}

impl shader::Primitive for TextVerticesPrimitive {
    type Pipeline = ShapesPipelineRenderer;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &iced::Rectangle,
        viewport: &shader::Viewport,
    ) {
        let uniforms = Uniforms::new(&self.camera, [0.0, 0.0, 0.0], 0.0, 0.0, 0.0, 0.0, 1.0);
        let mut state = self.uniforms_state.lock().unwrap();

        if let Some(state) = state.as_mut() {
            queue.write_buffer(&state.buffer, 0, bytemuck::bytes_of(&uniforms));
        } else {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Text Vertices Uniform Buffer"),
                size: std::mem::size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&buffer, 0, bytemuck::bytes_of(&uniforms));

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Text Vertices Uniform Bind Group"),
                layout: &pipeline.uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

            *state = Some(UniformsState { buffer, bind_group });
        }

        let colored: Vec<gui::gpu::pipelines::planet::vertex::ColoredVertex> = self
            .shape_vertices
            .iter()
            .map(|v| gui::gpu::pipelines::planet::vertex::ColoredVertex {
                position: [v[0], v[1], v[2]],
                color: [v[3], v[4], v[5]],
                rotate_with_earth: v[6],
            })
            .collect();
        pipeline
            .pipeline
            .set_data(device, queue, &colored, &self.shape_ranges);
        pipeline
            .text_pipeline
            .prepare(device, queue, &self.camera, &self.text_quads, 0.0);
        pipeline.prepare_depth_texture(
            device,
            viewport.physical_width(),
            viewport.physical_height(),
        );
    }

    fn draw(&self, _pipeline: &Self::Pipeline, _render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        false
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &iced::Rectangle<u32>,
    ) {
        let state = self.uniforms_state.lock().unwrap();
        if let Some(state) = state.as_ref() {
            let depth_view = pipeline
                .depth_texture
                .as_ref()
                .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text Vertices Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_view.as_ref().map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_viewport(
                clip_bounds.x as f32,
                clip_bounds.y as f32,
                clip_bounds.width as f32,
                clip_bounds.height as f32,
                0.0,
                1.0,
            );
            render_pass.set_scissor_rect(
                clip_bounds.x,
                clip_bounds.y,
                clip_bounds.width,
                clip_bounds.height,
            );

            pipeline
                .pipeline
                .render(&mut render_pass, &state.bind_group);
            pipeline.text_pipeline.render(&mut render_pass);
        }
    }
}

fn create_sample_text_quads() -> Vec<[f32; TEXT_VERTEX_FLOATS]> {
    let mut quads = Vec::new();

    quads.extend(text::build_text_quads(
        Vector3::new(-6.0, 0.0, 1.5),
        0.35,
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        [0.95, 0.55, 0.30],
    ));

    quads.extend(text::build_text_quads(
        Vector3::new(-6.0, 0.0, 0.0),
        0.35,
        "abcdefghijklmnopqrstuvwxyz",
        [0.30, 0.85, 0.45],
    ));

    quads.extend(text::build_text_quads(
        Vector3::new(-6.0, 0.0, -1.5),
        0.35,
        "0123456789.,:()+-=/",
        [0.45, 0.60, 0.95],
    ));

    quads.extend(text::build_text_quads(
        Vector3::new(-6.0, 0.0, -3.0),
        0.26,
        "The quick brown fox jumps over the lazy dog",
        [0.95, 0.95, 0.40],
    ));

    quads.extend(text::build_axis_label_quads(
        Vector3::new(3.0, 0.0, 0.0),
        0,
        0.35,
        [1.0, 0.3, 0.3],
    ));
    quads.extend(text::build_axis_label_quads(
        Vector3::new(0.0, 3.0, 0.0),
        1,
        0.35,
        [0.3, 1.0, 0.3],
    ));
    quads.extend(text::build_axis_label_quads(
        Vector3::new(0.0, 0.0, 3.0),
        2,
        0.35,
        [0.3, 0.5, 1.0],
    ));

    quads
}

fn main() -> iced::Result {
    env_logger::init();

    let mut app = iced::application(ShapesSimulation::new, update, view);
    if let Some(size) = gui::screenshot::window_size() {
        app = app.window_size(size);
    }
    app.subscription(|_state: &ShapesSimulation| {
        subscription(|_| Message::Tick, Message::Event)
    })
    .run()
}