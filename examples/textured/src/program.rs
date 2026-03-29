use gui::gpu::pipelines::textured::{camera::Camera, pipeline::Pipeline, vertex::TextureVertex};
use iced::{Rectangle, mouse, wgpu, widget::shader};
use std::sync::Arc;

#[derive(Debug)]
pub struct Simulation {
    pub triangles: Arc<Vec<TextureVertex>>,
}

impl Simulation {
    pub fn new(triangles: Vec<TextureVertex>) -> Self {
        Self {
            triangles: Arc::new(triangles),
        }
    }
}

pub struct Program {
    pub simulation: Arc<Simulation>,
    pub camera: Camera,
}

impl Program {}

impl<Message> shader::Program<Message> for Program {
    type State = String;

    type Primitive = Primitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: iced::Rectangle,
    ) -> Self::Primitive {
        let mut camera = self.camera.clone();
        camera.change_aspect(bounds.width, bounds.height);

        Primitive {
            simulation: Arc::clone(&self.simulation),
            camera,
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    simulation: Arc<Simulation>,
    camera: Camera,
}

impl shader::Primitive for Primitive {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &iced::Rectangle,
        viewport: &shader::Viewport,
    ) {
        pipeline.prepare(
            device,
            queue,
            bounds,
            viewport,
            self.simulation.triangles.as_ref(),
            &self.camera,
        );
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        // Render primitive
        pipeline.render(
            encoder,
            target,
            self.simulation.triangles.as_ref(),
            clip_bounds,
        );
    }
}
