/// Moon Phases validation example.
///
/// Validates:
/// - Moon position relative to Earth and Sun
/// - Moon phase angle computation (0°=new moon, 180°=full moon)
/// - Earth-Moon and Earth-Sun lines for visual verification
/// - Shows the Moon at known full-moon date (Sun and Moon ~opposite directions)
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
use nalgebra::{Point3, Vector3};

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Event(iced::event::Event),
}

struct MoonPhasesSimulation {
    program: ProgramSimulation,
    validation_info: String,
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
}

impl MoonPhasesSimulation {
    fn new() -> Self {
        // March 14, 2025 — known full moon date
        let full_moon_time = Utc.with_ymd_and_hms(2025, 3, 14, 6, 0, 0).unwrap();
        let (day, hour) = Astral::datetime_to_day_hour(&full_moon_time);

        let sun_dir = Astral::sun_inertial_position(day, hour);
        let moon_pos = Astral::moon_inertial_position(day, hour);
        let phase_angle = Astral::moon_phase_angle(day, hour);

        let moon_dir =
            Vector3::new(moon_pos[0] as f32, moon_pos[1] as f32, moon_pos[2] as f32).normalize();
        let moon_dist_km =
            Vector3::new(moon_pos[0] as f32, moon_pos[1] as f32, moon_pos[2] as f32).norm();

        // Camera from above to see both Sun and Moon directions
        let camera_eye = Point3::new(0.0, 0.0, 80_000.0);

        let mut core_sim = System::builder().build(full_moon_time);
        core_sim.simulation_speed = 0;

        let earth_radius = gui::model::system::EARTH_RADIUS_KM;

        // ECI frame
        core_sim.shapes.add_eci_frame(earth_radius * 2.0);

        // Sun direction line
        core_sim.shapes.add_sun_line(
            gui::model::FrameMode::Eci,
            [sun_dir[0] as f32, sun_dir[1] as f32, sun_dir[2] as f32],
            earth_radius * 4.0,
        );

        // Earth-Moon line
        let moon_line_len = earth_radius * 4.0;
        core_sim.shapes.add_line(
            gui::model::FrameMode::Eci,
            [0.0, 0.0, 0.0],
            [
                moon_dir.x * moon_line_len,
                moon_dir.y * moon_line_len,
                moon_dir.z * moon_line_len,
            ],
            "Earth→Moon",
        );

        // North pole and equator markers
        core_sim.shapes.add_surface_point(90.0, 0.0, "North Pole");
        core_sim.shapes.add_surface_point(0.0, 0.0, "(0°,0°)");

        let mut camera = Camera::new(camera_eye, [0.0, 0.0, 0.0].into(), 1600.0, 900.0);
        camera.fovy = 40.;

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
            "MOON PHASE VALIDATION — {} | \
             Phase angle: {:.1}° (expect ≈180° for full moon) | \
             Moon dist: {:.0} km | \
             Moon ECI: ({:.0}, {:.0}, {:.0}) | \
             Sun ECI: ({:.4}, {:.4}, {:.4})",
            full_moon_time.format("%Y-%m-%d %H:%M UTC"),
            phase_angle,
            moon_dist_km,
            moon_pos[0],
            moon_pos[1],
            moon_pos[2],
            sun_dir[0],
            sun_dir[1],
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

impl iced::widget::shader::Program<Message> for MoonPhasesSimulation {
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

fn update(sim: &mut MoonPhasesSimulation, message: Message) {
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

fn view(sim: &MoonPhasesSimulation) -> Element<'_, Message> {
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

    iced::application(MoonPhasesSimulation::new, update, view)
        .subscription(|_state: &MoonPhasesSimulation| {
            iced::Subscription::batch([
                time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
                iced::event::listen().map(Message::Event),
            ])
        })
        .run()
}
