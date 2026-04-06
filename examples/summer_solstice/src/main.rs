/// Summer Solstice validation example.
///
/// Validates that on the June solstice (~day 172):
/// - Solar declination is ≈ +23.44° (maximum)
/// - The subsolar point is at latitude ≈ +23.44° (Tropic of Cancer)
/// - The Sun direction has a significant +Z component in ECI
/// - Earth-Sun line tilts northward from the equatorial plane
use chrono::{TimeZone, Utc};
use gui::astro::Astral;
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::FrameMode;
use gui::model::system::System;
use gui::simulation::Simulation as ProgramSimulation;
use iced::keyboard::{self, Key, key::Named};
use iced::mouse;
use iced::time;
use iced::widget::{column, container, shader, text};
use iced::{Element, Length};
use nalgebra::Point3;

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Event(iced::event::Event),
}

struct SolsticeSimulation {
    program: ProgramSimulation,
    validation_info: String,
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
}

impl SolsticeSimulation {
    fn new() -> Self {
        // June 21, 2025 12:00 UTC — approximate summer solstice
        let solstice_time = Utc.with_ymd_and_hms(2025, 6, 21, 12, 0, 0).unwrap();
        let (day, hour) = Astral::datetime_to_day_hour(&solstice_time);

        let (subsolar_lat, subsolar_lon) = Astral::subsolar_point(day, hour);
        let declination = Astral::solar_declination_deg(day);
        let sun_dir = Astral::sun_inertial_position(day, hour);

        // Camera from above-side to see the tilt
        let camera_eye = Point3::new(0.0, -25_000.0, 15_000.0);

        let mut core_sim = System::builder().build(solstice_time);
        core_sim.simulation_speed = 0;

        let earth_radius = gui::model::system::EARTH_RADIUS_KM;
        let axis_len = earth_radius * 2.0;

        // ECI frame
        core_sim.shapes.add_eci_frame(axis_len);

        // Sun direction
        core_sim.shapes.add_sun_line(
            gui::model::FrameMode::Eci,
            [sun_dir[0] as f32, sun_dir[1] as f32, sun_dir[2] as f32],
            earth_radius * 3.0,
        );

        // Subsolar point on surface
        core_sim.shapes.add_surface_point(
            subsolar_lat as f32,
            subsolar_lon as f32,
            "Subsolar (Tropic of Cancer)",
        );
        core_sim.shapes.add_surface_line(
            subsolar_lat as f32,
            subsolar_lon as f32,
            earth_radius * 0.5,
            "Subsolar radial",
        );

        // Tropic of Cancer line (≈23.44°N) — mark several points along it
        for lon in (-180..=180).step_by(30) {
            core_sim.shapes.add_surface_point(23.44, lon as f32, "");
        }

        // Arctic circle (≈66.56°N)
        for lon in (-180..=180).step_by(30) {
            core_sim.shapes.add_surface_point(66.56, lon as f32, "");
        }

        // Equator reference
        core_sim
            .shapes
            .add_surface_point(0.0, 0.0, "Equator (0°,0°)");
        core_sim.shapes.add_surface_point(90.0, 0.0, "North Pole");

        let mut camera = Camera::new(camera_eye, [0.0, 0.0, 0.0].into(), 1600.0, 900.0);
        camera.fovy = 30.;

        let program = ProgramSimulation {
            system: core_sim,
            camera,
            satellite_mode: SatelliteRenderMode::Dot,
            frame_mode: FrameMode::Eci,
            ecef_reference_earth_angle: 0.0,
            paused: true,
            time_scale: 0.0,
            pick_radius_scale: 1.0,
            show_clouds: false,
        };

        let validation_info = format!(
            "SUMMER SOLSTICE VALIDATION — Day {} ({}) | \
             Declination: {:.4}° (expect ≈+23.44°) | \
             Subsolar: ({:.2}°, {:.2}°) | \
             Sun ECI Z: {:.4} (positive = north tilt)",
            day,
            solstice_time.format("%Y-%m-%d %H:%M UTC"),
            declination,
            subsolar_lat,
            subsolar_lon,
            sun_dir[2],
        );

        Self {
            program,
            validation_info,
            cursor_position: None,
            drag_start: None,
            right_button_down: false,
        }
    }
}

impl iced::widget::shader::Program<Message> for SolsticeSimulation {
    type State = <ProgramSimulation as iced::widget::shader::Program<Message>>::State;
    type Primitive = <ProgramSimulation as iced::widget::shader::Program<Message>>::Primitive;

    fn draw(
        &self,
        state: &Self::State,
        cursor: mouse::Cursor,
        bounds: iced::Rectangle,
    ) -> Self::Primitive {
        <ProgramSimulation as iced::widget::shader::Program<Message>>::draw(
            &self.program,
            state,
            cursor,
            bounds,
        )
    }
}

fn update(sim: &mut SolsticeSimulation, message: Message) {
    match message {
        Message::Tick => {}
        Message::Event(event) => {
            let rotate_angle = 5.0_f32.to_radians();
            let zoom_amount = 500.0;
            match event {
                iced::event::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    match key {
                        Key::Named(Named::ArrowLeft) => {
                            sim.program.camera.rotate_around_up(-rotate_angle)
                        }
                        Key::Named(Named::ArrowRight) => {
                            sim.program.camera.rotate_around_up(rotate_angle)
                        }
                        Key::Named(Named::ArrowUp) => {
                            sim.program.camera.rotate_vertically(-rotate_angle)
                        }
                        Key::Named(Named::ArrowDown) => {
                            sim.program.camera.rotate_vertically(rotate_angle)
                        }
                        Key::Character(ch) if ch == "+" || ch == "=" => {
                            sim.program.camera.dolly(-zoom_amount)
                        }
                        Key::Character(ch) if ch == "-" || ch == "_" => {
                            sim.program.camera.dolly(zoom_amount)
                        }
                        _ => {}
                    }
                }
                iced::event::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    let (x, y) = (position.x, position.y);
                    if sim.right_button_down {
                        if let Some((px, py)) = sim.drag_start {
                            sim.program.camera.rotate_around_up(-(x - px) * 0.005);
                            sim.program.camera.rotate_vertically(-(y - py) * 0.005);
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
                        iced::mouse::ScrollDelta::Pixels { y, .. } => y * zoom_amount / 100.0,
                    };
                    sim.program.camera.dolly(amount);
                }
                _ => {}
            }
        }
    }
}

fn view(sim: &SolsticeSimulation) -> Element<'_, Message> {
    let scene = shader(sim).width(Length::Fill).height(Length::Fill);
    let info = text(&sim.validation_info).size(14);

    container(column![info, scene].spacing(4))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(4)
        .into()
}

fn main() -> iced::Result {
    env_logger::init();

    iced::application(SolsticeSimulation::new, update, view)
        .subscription(|_state: &SolsticeSimulation| {
            iced::Subscription::batch([
                time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
                iced::event::listen().map(Message::Event),
            ])
        })
        .run()
}
