use geometry::tesselation::build_sphere;
use gui::gpu::pipelines::textured::{camera::Camera, vertex::into_textured_vertex};
use iced::{
    Alignment::Center,
    Element,
    Length::Fill,
    Theme,
    keyboard::{self, Key},
    widget::{button, center, column, row, shader},
};

mod program;

#[derive(Clone)]
enum Message {
    KeyboardEvent(keyboard::Event),
}

struct Textured {
    program: program::Program,
}

impl Textured {
    fn update(&mut self, message: Message) {
        match message {
            Message::KeyboardEvent(event) => self.handle_keyboard_event(event),
        }
    }

    fn handle_keyboard_event(&mut self, event: keyboard::Event) {
        if let keyboard::Event::KeyPressed { key, .. } = event {
            let delta_angle = 5.0_f32.to_radians();
            match key {
                Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
                    self.program.camera.rotate_around_up(-delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                    self.program.camera.rotate_around_up(delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                    self.program.camera.rotate_vertically(-delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                    self.program.camera.rotate_vertically(delta_angle);
                }
                _ => (),
            }
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
        .subscription(|_state: &Textured| iced::keyboard::listen().map(Message::KeyboardEvent))
        .theme(Theme::KanagawaDragon)
        .run()
}
