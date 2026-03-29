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
    pub start_time: std::time::Instant,
}

impl Program {
    pub fn satellite_position(&self) -> [f32; 3] {
        const SPEED: f32 = 0.8;
        const RADIUS: f32 = 2.7;

        let elapsed = self.start_time.elapsed().as_secs_f32();
        let angle = (elapsed * SPEED) % (std::f32::consts::TAU);

        [RADIUS * angle.cos(), 0.0, RADIUS * angle.sin()]
    }
}

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
            satellite_position: self.satellite_position(),
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    simulation: Arc<Simulation>,
    camera: Camera,
    satellite_position: [f32; 3],
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
            self.satellite_position,
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
