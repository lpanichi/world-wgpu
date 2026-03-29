use gui::gpu::pipelines::textured::{camera::Camera, pipeline::Pipeline, vertex::TextureVertex};
use iced::{Rectangle, mouse, wgpu, widget::shader};

pub struct Program {
    pub triangles: Vec<TextureVertex>,
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
            triangles: self.triangles.clone(),
            camera: camera,
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    triangles: Vec<TextureVertex>,
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
            &self.triangles,
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
        pipeline.render(encoder, target, &self.triangles, clip_bounds);
    }
}
