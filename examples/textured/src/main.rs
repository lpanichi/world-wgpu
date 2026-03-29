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
use nalgebra::{Rotation3, Unit};
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
                    self.rotate_camera_around_up(-delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                    self.rotate_camera_around_up(delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                    self.rotate_camera_vertically(-delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                    self.rotate_camera_vertically(delta_angle);
                }
                _ => (),
            }
        }
    }

    fn rotate_camera_around_up(&mut self, angle_rad: f32) {
        let camera = &mut self.program.camera;
        let axis = camera.up; // Unit<Vector3<f32>>
        let direction = camera.eye - camera.target;
        let rot = Rotation3::from_axis_angle(&axis, angle_rad);
        camera.eye = camera.target + rot * direction;
    }

    fn rotate_camera_vertically(&mut self, angle_rad: f32) {
        let camera = &mut self.program.camera;
        let right =
            Unit::new_normalize((camera.eye - camera.target).cross(&camera.up.into_inner()));
        let direction = camera.eye - camera.target;
        let rot = Rotation3::from_axis_angle(&right, angle_rad);
        camera.eye = camera.target + rot * direction;
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
