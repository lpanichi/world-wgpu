use geometry::tesselation::build_sphere;
use gui::gpu::pipelines::textured::{camera::Camera, vertex::into_textured_vertex};
use iced::{
    Alignment::Center,
    Element,
    Length::Fill,
    Theme,
    widget::{button, center, column, row, shader},
};
mod program;

#[derive(Clone)]
enum Message {
    MoveCamera([f32; 3]),
}

struct Textured {
    program: program::Program,
}

impl Textured {
    fn update(&mut self, message: Message) {
        match message {
            Message::MoveCamera(position) => self.program.move_camera(&position),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let shader = shader(&self.program).width(Fill).height(Fill);
        let controls = row![button("hello")];

        center(column![shader, controls].align_x(Center)).into()
    }
}

impl Default for Textured {
    fn default() -> Self {
        let sphere = build_sphere();
        let triangles = into_textured_vertex(sphere);
        let camera = Camera::new([0., 0., -3.].into(), [0., 0., 0.].into(), 200., 200.);

        Self {
            program: program::Program {
                triangles: triangles,
                camera: camera,
            },
        }
    }
}

fn main() -> iced::Result {
    iced::application(Textured::default, Textured::update, Textured::view)
        .theme(Theme::KanagawaDragon)
        .run()
}
