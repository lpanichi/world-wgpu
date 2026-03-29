use gui::gpu::pipelines::planet::{camera::Camera, pipeline::Pipeline, satellite::SatelliteRenderMode};
use gui::model::Simulation;
use iced::{Rectangle, mouse, wgpu, widget::shader};
use std::sync::Arc;

pub struct Program {
    pub model: Arc<Simulation>,
    pub camera: Camera,
    pub start_time: std::time::Instant,
    pub satellite_mode: SatelliteRenderMode,
}

impl Program {
    pub fn elapsed_time(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
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
            model: Arc::clone(&self.model),
            camera,
            elapsed: self.elapsed_time(),
            satellite_mode: self.satellite_mode,
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    model: Arc<Simulation>,
    camera: Camera,
    elapsed: f32,
    satellite_mode: SatelliteRenderMode,
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
            &self.model,
            &self.camera,
            self.elapsed,
            self.satellite_mode,
        );
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        pipeline.render(encoder, target, clip_bounds);
    }
}
