/// ECI/ECEF Frame Validation example.
///
/// Validates reference frame orientations:
/// - ECI frame: X toward vernal equinox, Y 90° east, Z toward north pole (fixed in inertial space)
/// - ECEF frame: X through Greenwich meridian, rotating with Earth
/// - At t=0, shows both frames diverging as Earth rotates
/// - Ground stations remain fixed in ECEF, rotate with Earth in ECI view
/// - Sun direction line stays fixed in inertial space
///
/// The simulation runs (not paused) so you can watch the ECEF frame rotate
/// relative to ECI while ground stations track with the Earth.
use chrono::{TimeZone, Utc};
use gui::astro::Astral;
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::FrameMode;
use gui::model::ground_station::GroundStation;
use gui::model::orbit::Orbit;
use gui::model::satellite::Satellite;
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

struct FrameValidationSimulation {
    program: ProgramSimulation,
    validation_info: String,
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
}

impl FrameValidationSimulation {
    fn new() -> Self {
        let sim_time = Utc.with_ymd_and_hms(2025, 3, 20, 0, 0, 0).unwrap();
        let (day, hour) = Astral::datetime_to_day_hour(&sim_time);

        let sun_dir = Astral::sun_inertial_position(day, hour);

        // Ground station at Greenwich for visual ECEF reference
        let mut greenwich = GroundStation::new("Greenwich", 51.48, 0.0);
        greenwich.show_cone = false;
        greenwich.cube_size = 300.0;

        // One in Tokyo for longitude reference
        let mut tokyo = GroundStation::new("Tokyo", 35.68, 139.69);
        tokyo.show_cone = false;
        tokyo.cube_size = 300.0;

        // A polar orbit satellite for reference — ECI-fixed trajectory
        let polar_orbit = Orbit::builder(8_000.0, Orbit::circular_period_seconds(8_000.0))
            .name("Polar orbit")
            .inclination(90.0)
            .raan(0.0)
            .show_orbit(true)
            .add_satellite(Satellite::builder("Polar-1").phase_offset(0.0).build())
            .build();

        let mut core_sim = System::builder()
            .add_ground_station(greenwich)
            .add_ground_station(tokyo)
            .add_orbit(polar_orbit)
            .build(sim_time);

        // Run at 500x speed to watch frames diverge quickly
        core_sim.simulation_speed = 500;

        let earth_radius = gui::model::system::EARTH_RADIUS_KM;

        // ECI frame (fixed in inertial space)
        core_sim.shapes.add_eci_frame(earth_radius * 2.5);

        // ECEF frame (rotates dynamically with Earth each render frame)
        core_sim.shapes.add_ecef_frame(earth_radius * 2.0);

        // Sun direction
        core_sim.shapes.add_sun_line(
            gui::model::FrameMode::Eci,
            [sun_dir[0] as f32, sun_dir[1] as f32, sun_dir[2] as f32],
            earth_radius * 3.0,
        );

        // Reference points
        core_sim.shapes.add_surface_point(0.0, 0.0, "(0°,0°)");
        core_sim.shapes.add_surface_point(90.0, 0.0, "North Pole");

        let camera_eye = Point3::new(10_000.0, -15_000.0, 12_000.0);
        let mut camera = Camera::new(camera_eye, [0.0, 0.0, 0.0].into(), 1600.0, 900.0);
        camera.fovy = 40.;

        let program = ProgramSimulation {
            system: core_sim,
            camera,
            satellite_mode: SatelliteRenderMode::Dot,
            frame_mode: FrameMode::Eci,
            ecef_reference_earth_angle: 0.0,
            paused: false,
            time_scale: 500.0,
            pick_radius_scale: 1.0,
            show_clouds: false,
        };

        let validation_info =
            "FRAME VALIDATION — Watch: ECI axes (long, labeled X/Y/Z) stay fixed, \
             ECEF axes (short, labeled X/Y/Z) rotate with Earth. \
             Ground stations rotate in ECI view. Press 'F' to toggle ECI/ECEF. Speed: 500x"
                .to_string();

        Self {
            program,
            validation_info,
            cursor_position: None,
            drag_start: None,
            right_button_down: false,
        }
    }
}

impl iced::widget::shader::Program<Message> for FrameValidationSimulation {
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

fn update(sim: &mut FrameValidationSimulation, message: Message) {
    match message {
        Message::Tick => {
            if !sim.program.paused {
                sim.program.tick();
            }
        }
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
                        Key::Character(ch) if ch == "f" || ch == "F" => {
                            sim.program.frame_mode = match sim.program.frame_mode {
                                FrameMode::Eci => FrameMode::Ecef,
                                FrameMode::Ecef => FrameMode::Eci,
                            };
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

fn view(sim: &FrameValidationSimulation) -> Element<'_, Message> {
    let scene = shader(sim).width(Length::Fill).height(Length::Fill);
    let frame_label = match sim.program.frame_mode {
        FrameMode::Eci => "Current: ECI",
        FrameMode::Ecef => "Current: ECEF",
    };
    let info = text(format!("{} | {}", sim.validation_info, frame_label)).size(13);

    container(column![info, scene].spacing(4))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(4)
        .into()
}

fn main() -> iced::Result {
    env_logger::init();

    iced::application(FrameValidationSimulation::new, update, view)
        .subscription(|_state: &FrameValidationSimulation| {
            iced::Subscription::batch([
                time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
                iced::event::listen().map(Message::Event),
            ])
        })
        .run()
}
