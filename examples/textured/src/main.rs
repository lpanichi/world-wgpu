use gui::gpu::pipelines::planet::camera::Camera;
use gui::gpu::pipelines::planet::satellite::SatelliteRenderMode;
use gui::model::{GroundStation, Orbit, Satellite, Simulation};
use iced::{
    Alignment::Center,
    Element,
    Length::Fill,
    Theme,
    keyboard::{self, Key},
    time::{self, milliseconds},
    widget::{button, center, column, row, shader},
};

mod program;

#[derive(Clone)]
enum Message {
    KeyboardEvent(keyboard::Event),
    Event(iced::event::Event),
    Tick,
}

struct Textured {
    program: program::Program,
}

impl Textured {
    fn update(&mut self, message: Message) {
        match message {
            Message::KeyboardEvent(event) => self.handle_keyboard_event(event),
            Message::Event(event) => self.handle_event(event),
            Message::Tick => {
                // Nothing needed; redraw is triggered by the timer tick
            }
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
                Key::Named(iced::keyboard::key::Named::Space) => {
                    self.program.satellite_mode = match self.program.satellite_mode {
                        SatelliteRenderMode::Cube => SatelliteRenderMode::Dot,
                        SatelliteRenderMode::Dot => SatelliteRenderMode::Cube,
                    };
                }
                _ => (),
            }
        }
    }

    fn handle_event(&mut self, event: iced::event::Event) {
        if let iced::event::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) = event {
            let amount = match delta {
                iced::mouse::ScrollDelta::Lines { y, .. } => y * 0.5,
                iced::mouse::ScrollDelta::Pixels { y, .. } => y * 0.01,
            };
            self.program.camera.dolly(amount);
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let shader = shader(&self.program).width(Fill).height(Fill);
        let controls = row![iced::widget::text(format!(
            "Satellites: {:?} (space to toggle)",
            self.program.satellite_mode
        ))];

        center(column![shader, controls].align_x(Center)).into()
    }
}

impl Default for Textured {
    fn default() -> Self {
        let model = Simulation::builder()
            .add_orbit(
                Orbit::builder(6.0, 20.0)
                    .inclination(20.0)
                    .raan(30.0)
                    .arg_perigee(0.0)
                    .show_orbit(true)
                    .add_satellite(Satellite::builder("Sat-1").phase_offset(0.0).build())
                    .add_satellite(Satellite::builder("Sat-2").phase_offset(2.0).build())
                    .build(),
            )
            .add_orbit(
                Orbit::builder(8.0, 30.0)
                    .inclination(45.0)
                    .raan(80.0)
                    .arg_perigee(30.0)
                    .show_orbit(true)
                    .add_satellite(Satellite::builder("Sat-3").phase_offset(2.0).build())
                    .build(),
            )
            .add_ground_station(GroundStation::new("Station A", 30.0, 10.0))
            .add_ground_station(GroundStation::new("Station B", -20.0, 100.0))
            .build();

        let camera = Camera::new([0., 6., -15.].into(), [0., 0., 0.].into(), 200., 200.);

        Self {
            program: program::Program {
                model: std::sync::Arc::new(model),
                camera,
                start_time: std::time::Instant::now(),
                satellite_mode: SatelliteRenderMode::Dot,
            },
        }
    }
}

fn main() -> iced::Result {
    iced::application(Textured::default, Textured::update, Textured::view)
        .subscription(|_state: &Textured| {
            iced::Subscription::batch([
                iced::keyboard::listen().map(Message::KeyboardEvent),
                iced::event::listen().map(Message::Event),
                time::every(milliseconds(16)).map(|_| Message::Tick),
            ])
        })
        .theme(Theme::KanagawaDragon)
        .run()
}
