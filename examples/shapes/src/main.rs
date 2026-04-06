use gui::gpu::pipelines::planet::{camera::Camera, shapes::ShapesPipeline, uniforms::Uniforms};
use gui::model::shapes::Shapes;
use gui::model::text_vertices::{TextMesh, build_axis_label, build_text};
use iced::keyboard::{self, Key, key::Named};
use iced::mouse;
use iced::time;
use iced::wgpu;
use iced::widget::{column, container, shader, text};
use iced::{Element, Length};
use nalgebra::{Point3, Vector3};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
struct FreeCamera {
    camera: Camera,
    pan_speed: f32,
    rotate_speed: f32,
}

impl FreeCamera {
    fn new(eye: Point3<f32>, target: Point3<f32>, width: f32, height: f32) -> Self {
        let mut camera = Camera::new(eye, target, width, height);
        camera.fovy = 50.0;

        FreeCamera {
            camera,
            pan_speed: 0.75,
            rotate_speed: 0.07,
        }
    }

    fn as_camera(&self) -> &Camera {
        &self.camera
    }

    fn move_left(&mut self) {
        self.pan(-self.pan_speed, 0.0);
    }

    fn move_right(&mut self) {
        self.pan(self.pan_speed, 0.0);
    }

    fn move_up(&mut self) {
        self.pan(0.0, self.pan_speed);
    }

    fn move_down(&mut self) {
        self.pan(0.0, -self.pan_speed);
    }

    fn pan(&mut self, horizontal: f32, vertical: f32) {
        let view_dir = (self.camera.target - self.camera.eye).normalize();
        let mut right = view_dir.cross(&self.camera.up.into_inner());
        if right.norm_squared() < 1e-6 {
            right = Vector3::new(1.0, 0.0, 0.0);
        }
        let right = right.normalize();
        let translation = right * horizontal + self.camera.up.into_inner() * vertical;
        self.camera.eye += translation;
        self.camera.target += translation;
    }

    fn rotate_yaw(&mut self, delta: f32) {
        self.camera.rotate_around_up(delta * self.rotate_speed);
    }

    fn rotate_pitch(&mut self, delta: f32) {
        self.camera.rotate_vertically(delta * self.rotate_speed);
    }

    fn dolly(&mut self, amount: f32) {
        self.camera.dolly(amount);
    }
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Event(iced::event::Event),
}

struct ShapesSimulation {
    camera: FreeCamera,
    text_vertices: Vec<gui::gpu::pipelines::planet::vertex::ColoredVertex>,
    text_ranges: Vec<(u32, u32)>,
    help_text: String,
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
}

impl ShapesSimulation {
    fn new() -> Self {
        let text_mesh = create_text_mesh();
        let text_vertices: Vec<gui::gpu::pipelines::planet::vertex::ColoredVertex> = text_mesh
            .vertices
            .iter()
            .map(|vert| gui::gpu::pipelines::planet::vertex::ColoredVertex {
                position: [vert[0], vert[1], vert[2]],
                color: [vert[3], vert[4], vert[5]],
            })
            .collect();

        let text_ranges = text_mesh.ranges;

        let camera_eye = Point3::new(-6.0, -12.0, 4.0);
        let camera_target = Point3::new(-6.0, 0.0, 0.0);
        let camera = FreeCamera::new(camera_eye, camera_target, 1600.0, 900.0);

        ShapesSimulation {
            camera,
            text_vertices,
            text_ranges,
            help_text: "Shapes: Pan arrows | Rotate right-drag | Zoom +/-".to_string(),
            cursor_position: None,
            drag_start: None,
            right_button_down: false,
        }
    }
}

#[derive(Debug)]
struct TextVerticesPrimitive {
    vertices: Vec<gui::gpu::pipelines::planet::vertex::ColoredVertex>,
    ranges: Vec<(u32, u32)>,
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
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        self.depth_texture = Some(depth_texture);
    }
}

impl shader::Pipeline for ShapesPipelineRenderer {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
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
            pipeline: ShapesPipeline::new(device, format, &uniform_bind_group_layout),
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
            vertices: self.text_vertices.clone(),
            ranges: self.text_ranges.clone(),
            camera: self.camera.as_camera().clone(),
            uniforms_state: Arc::new(Mutex::new(None)),
        }
    }
}

fn update(sim: &mut ShapesSimulation, message: Message) {
    match message {
        Message::Tick => {}
        Message::Event(event) => {
            let zoom_amount = 1.5;
            match event {
                iced::event::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    match key {
                        Key::Named(Named::ArrowLeft) => sim.camera.move_left(),
                        Key::Named(Named::ArrowRight) => sim.camera.move_right(),
                        Key::Named(Named::ArrowUp) => sim.camera.move_up(),
                        Key::Named(Named::ArrowDown) => sim.camera.move_down(),
                        Key::Character(ch) if ch == "+" || ch == "=" => {
                            sim.camera.dolly(-zoom_amount)
                        }
                        Key::Character(ch) if ch == "-" || ch == "_" => {
                            sim.camera.dolly(zoom_amount)
                        }
                        _ => {}
                    }
                }
                iced::event::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    let (x, y) = (position.x, position.y);
                    if sim.right_button_down {
                        if let Some((px, py)) = sim.drag_start {
                            sim.camera.rotate_yaw(-(x - px) * 0.005);
                            sim.camera.rotate_pitch(-(y - py) * 0.005);
                            sim.drag_start = Some((x, y));
                        } else {
                            sim.drag_start = Some((x, y));
                        }
                    }
                    sim.cursor_position = Some((x, y));
                }
                iced::event::Event::Mouse(iced::mouse::Event::ButtonPressed(
                    iced::mouse::Button::Right,
                )) => {
                    sim.right_button_down = true;
                    sim.drag_start = sim.cursor_position;
                }
                iced::event::Event::Mouse(iced::mouse::Event::ButtonReleased(
                    iced::mouse::Button::Right,
                )) => {
                    sim.right_button_down = false;
                    sim.drag_start = None;
                }
                iced::event::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                    let amount = match delta {
                        iced::mouse::ScrollDelta::Lines { y, .. } => y * zoom_amount,
                        iced::mouse::ScrollDelta::Pixels { y, .. } => y * zoom_amount / 10.0,
                    };
                    sim.camera.dolly(-amount);
                }
                _ => {}
            }
        }
    }
}

fn view(sim: &ShapesSimulation) -> Element<'_, Message> {
    let scene = shader(sim).width(Length::Fill).height(Length::Fill);
    let info = text(&sim.help_text).size(16);

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
        let uniforms = Uniforms::new(&self.camera, [0.0, 0.0, 0.0], 0.0);
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

        pipeline
            .pipeline
            .set_data(device, queue, self.vertices.clone(), self.ranges.clone());
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
        }
    }
}

fn create_text_mesh() -> TextMesh {
    let mut mesh = TextMesh::new();

    mesh.append(&build_text(
        Vector3::new(-6.0, 0.0, 1.5),
        Vector3::new(0.0, 1.0, 0.0),
        0.35,
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        [0.95, 0.55, 0.30],
    ));

    mesh.append(&build_text(
        Vector3::new(-6.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        0.35,
        "abcdefghijklmnopqrstuvwxyz",
        [0.30, 0.85, 0.45],
    ));

    mesh.append(&build_text(
        Vector3::new(-6.0, 0.0, -1.5),
        Vector3::new(0.0, 1.0, 0.0),
        0.35,
        "0123456789.,:()+-=/",
        [0.45, 0.60, 0.95],
    ));

    mesh.append(&build_text(
        Vector3::new(-6.0, 0.0, -3.0),
        Vector3::new(0.0, 1.0, 0.0),
        0.26,
        "The quick brown fox jumps over the lazy dog",
        [0.95, 0.95, 0.40],
    ));

    mesh.append(&build_axis_label(
        Vector3::new(3.0, 0.0, 0.0),
        0,
        0.35,
        [1.0, 0.3, 0.3],
    ));
    mesh.append(&build_axis_label(
        Vector3::new(0.0, 3.0, 0.0),
        1,
        0.35,
        [0.3, 1.0, 0.3],
    ));
    mesh.append(&build_axis_label(
        Vector3::new(0.0, 0.0, 3.0),
        2,
        0.35,
        [0.3, 0.5, 1.0],
    ));

    let mut shapes = Shapes::new();
    shapes.add_frame(
        [-6.0, 0.0, 0.0],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        2.0,
        "Axes",
    );
    shapes.add_orbital_elements(3.5, 35.0, 20.0, 55.0);

    append_shapes_to_mesh(&mut mesh, &shapes, 0.0);

    mesh
}

fn append_shapes_to_mesh(mesh: &mut TextMesh, shapes: &Shapes, earth_rotation_angle: f32) {
    let (shape_vertices, shape_ranges) = shapes.line_points(earth_rotation_angle);
    let offset = mesh.vertices.len() as u32;
    mesh.vertices.extend(shape_vertices.into_iter());
    for (start, len) in shape_ranges {
        mesh.ranges.push((start + offset, len));
    }
}

fn main() -> iced::Result {
    env_logger::init();

    iced::application(ShapesSimulation::new, update, view)
        .subscription(|_state: &ShapesSimulation| {
            iced::Subscription::batch([
                time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
                iced::event::listen().map(Message::Event),
            ])
        })
        .run()
}
