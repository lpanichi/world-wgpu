/// Vernal Equinox validation example.
///
/// Validates that on the March equinox (~day 79-80):
/// - The Sun direction lies in the equatorial plane (declination ≈ 0°)
/// - The subsolar point latitude is near 0°
/// - The Earth-Sun line is perpendicular to the Earth's rotation axis
/// - ECI and ECEF frames are displayed for reference
/// - Key geographic points (0°,0° and poles) are marked on the surface
use chrono::{TimeZone, Utc};
use gui::astro::Astral;
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::shapes::lat_lon_to_ecef;
use gui::model::system::System;
use gui::model::FrameMode;
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

struct VernalEquinoxSimulation {
    program: ProgramSimulation,
    validation_info: String,
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
}

impl VernalEquinoxSimulation {
    fn new() -> Self {
        // March 20, 2025 12:00 UTC — approximate vernal equinox
        let vernal_time = Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap();
        let (day, hour) = Astral::datetime_to_day_hour(&vernal_time);

        let (subsolar_lat, subsolar_lon) = Astral::subsolar_point(day, hour);
        let declination = Astral::solar_declination_deg(day);
        let sun_dir = Astral::sun_inertial_position(day, hour);

        // Camera looks from the Sun direction
        let subsolar_ecef = lat_lon_to_ecef(subsolar_lat as f32, subsolar_lon as f32);
        let subsolar_direction =
            Vector3::new(subsolar_ecef[0], subsolar_ecef[1], subsolar_ecef[2]).normalize();
        let camera_distance = 30_000.0;
        let camera_eye = Point3::from(subsolar_direction * camera_distance);

        let mut core_sim = System::builder().build(vernal_time);
        core_sim.simulation_speed = 0;

        let earth_radius = gui::model::system::EARTH_RADIUS_KM;
        let axis_len = earth_radius * 2.0;

        // ECI frame at origin
        core_sim.shapes.add_eci_frame(axis_len);

        // ECEF frame (rotates dynamically)
        core_sim.shapes.add_ecef_frame(axis_len * 0.8);

        // Sun direction line
        core_sim.shapes.add_sun_line(
            gui::model::FrameMode::Eci,
            [sun_dir[0] as f32, sun_dir[1] as f32, sun_dir[2] as f32],
            earth_radius * 3.0,
        );

        // Key geographic points on the surface
        core_sim
            .shapes
            .add_surface_point(0.0, 0.0, "Equator/Greenwich (0°,0°)");
        core_sim.shapes.add_surface_point(0.0, 90.0, "Equator 90°E");
        core_sim
            .shapes
            .add_surface_point(0.0, -90.0, "Equator 90°W");
        core_sim
            .shapes
            .add_surface_point(0.0, 180.0, "Equator 180°");
        core_sim.shapes.add_surface_point(90.0, 0.0, "North Pole");
        core_sim.shapes.add_surface_point(-90.0, 0.0, "South Pole");

        // Subsolar point
        core_sim.shapes.add_surface_point(
            subsolar_lat as f32,
            subsolar_lon as f32,
            "Subsolar point",
        );
        core_sim.shapes.add_surface_line(
            subsolar_lat as f32,
            subsolar_lon as f32,
            earth_radius * 0.5,
            "Subsolar radial",
        );

        // Terminator longitudes at equator
        let (term_w, term_e) = Astral::terminator_longitudes(subsolar_lon);
        core_sim
            .shapes
            .add_surface_point(0.0, term_w as f32, "Terminator West");
        core_sim
            .shapes
            .add_surface_point(0.0, term_e as f32, "Terminator East");

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
            "VERNAL EQUINOX VALIDATION — Day {} ({}) | \
             Declination: {:.4}° (expect ≈0°) | \
             Subsolar: ({:.2}°, {:.2}°) | \
             Sun ECI: ({:.4}, {:.4}, {:.4}) — Z≈0 validates equatorial Sun",
            day,
            vernal_time.format("%Y-%m-%d %H:%M UTC"),
            declination,
            subsolar_lat,
            subsolar_lon,
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

impl iced::widget::shader::Program<Message> for VernalEquinoxSimulation {
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

fn update(sim: &mut VernalEquinoxSimulation, message: Message) {
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

fn view(sim: &VernalEquinoxSimulation) -> Element<'_, Message> {
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

    iced::application(VernalEquinoxSimulation::new, update, view)
        .subscription(|_state: &VernalEquinoxSimulation| {
            iced::Subscription::batch([
                time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
                iced::event::listen().map(Message::Event),
            ])
        })
        .run()
}
