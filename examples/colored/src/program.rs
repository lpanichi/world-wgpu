use gui::gpu::pipelines::star::{pipeline::Pipeline, vertex::ColorVertex};
use iced::{Rectangle, mouse, wgpu, widget::shader};

pub struct Program {
    pub triangles: Vec<ColorVertex>,
}

impl Program {
    pub fn new(triangles: Vec<ColorVertex>) -> Self {
        Self { triangles }
    }
}

impl<Message> shader::Program<Message> for Program {
    type State = String;

    type Primitive = Primitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: iced::Rectangle,
    ) -> Self::Primitive {
        Primitive {
            triangles: self.triangles.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    triangles: Vec<ColorVertex>,
}

impl shader::Primitive for Primitive {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        _bounds: &iced::Rectangle,
        _viewport: &shader::Viewport,
    ) {
        pipeline.prepare(device, queue, &self.triangles);
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        _clip_bounds: &Rectangle<u32>,
    ) {
        // Render primitive
        pipeline.render(encoder, target, &self.triangles);
    }
}
