/// Orbital Elements validation example.
///
/// Validates Keplerian orbital mechanics visualization:
/// - Ascending node line (intersection of orbital plane with equatorial plane)
/// - Orbital plane circle vs equatorial plane circle
/// - Inclination arc between the two planes
/// - Argument of perigee direction within the orbital plane
/// - Satellite position on the orbit at t=0
///
/// Shows a 45° inclined orbit with RAAN=30° and argp=60° for clear visualization.
/// Color code:
///   Equatorial plane circle — reference plane (i=0°)
///   Inclined orbit circle — the actual orbital plane
///   Node line — intersection of orbital and equatorial planes (Ω direction)
///   Perigee line — direction of closest approach within orbital plane (ω from node)
///   Inclination arc — angle between planes at ascending node
use chrono::{TimeZone, Utc};
use gui::astro::Astral;
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::orbit::Orbit;
use gui::model::satellite::Satellite;
use gui::model::system::System;
use gui::simulation::{FrameMode, Simulation as ProgramSimulation};
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

struct OrbitElementsSimulation {
    program: ProgramSimulation,
    validation_info: String,
    detail_info: String,
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
}

impl OrbitElementsSimulation {
    fn new() -> Self {
        let sim_time = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        let sma = 10_000.0_f32; // km
        let inc = 45.0_f32;
        let raan = 30.0_f32;
        let argp = 60.0_f32;
        let period = Orbit::circular_period_seconds(sma);

        let orbit = Orbit::builder(sma, period)
            .name("Validation Orbit")
            .inclination(inc)
            .raan(raan)
            .arg_perigee(argp)
            .show_orbit(true)
            .add_satellite(
                Satellite::builder("Sat-1")
                    .phase_offset(0.0)
                    .build(),
            )
            .build();

        // Second orbit: equatorial for contrast
        let equatorial_orbit = Orbit::builder(8_000.0, Orbit::circular_period_seconds(8_000.0))
            .name("Equatorial ref")
            .inclination(0.0)
            .raan(0.0)
            .arg_perigee(0.0)
            .show_orbit(true)
            .add_satellite(
                Satellite::builder("Eq-Sat")
                    .phase_offset(0.0)
                    .build(),
            )
            .build();

        // Sun-synchronous orbit for SSO validation
        let sso_inc = Astral::sun_synchronous_inclination(700.0, 0.0).unwrap_or(98.0);
        let sso_sma = 6371.0 + 700.0;
        let sso_orbit = Orbit::builder(sso_sma as f32, Orbit::circular_period_seconds(sso_sma as f32))
            .name("SSO (700km)")
            .inclination(sso_inc as f32)
            .raan(0.0)
            .arg_perigee(0.0)
            .show_orbit(true)
            .add_satellite(
                Satellite::builder("SSO-Sat")
                    .phase_offset(0.0)
                    .build(),
            )
            .build();

        let mut core_sim = System::builder()
            .add_orbit(orbit)
            .add_orbit(equatorial_orbit)
            .add_orbit(sso_orbit)
            .build(sim_time);
        core_sim.simulation_speed = 0;

        let earth_radius = gui::model::system::EARTH_RADIUS_KM;

        // ECI frame
        core_sim.shapes.add_eci_frame(earth_radius * 2.5);

        // Orbital elements visualization for the main orbit
        core_sim.shapes.add_orbital_elements(sma, inc, raan, argp);

        // Markers
        core_sim.shapes.add_surface_point(0.0, 0.0, "(0°,0°)");
        core_sim.shapes.add_surface_point(90.0, 0.0, "North Pole");

        // Ascending node direction marker on equator
        core_sim.shapes.add_surface_point(0.0, raan, "Ω (asc. node)");

        // Camera from above-side
        let camera_eye = Point3::new(15_000.0, -20_000.0, 15_000.0);

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
            "ORBITAL ELEMENTS — a={sma}km, i={inc}°, Ω={raan}°, ω={argp}° | \
             SSO(700km): i={sso_inc:.2}° (expect ≈98°)"
        );

        let detail_info = format!(
            "Elements breakdown: \
             [Ω = {raan}°] RAAN — longitude of ascending node (node line direction in equatorial plane) | \
             [i = {inc}°] Inclination — tilt of orbital plane from equator (arc at node) | \
             [ω = {argp}°] Arg. of perigee — angle from ascending node to perigee along orbit"
        );

        Self {
            program,
            validation_info,
            detail_info,
            cursor_position: None,
            drag_start: None,
            right_button_down: false,
        }
    }
}

impl iced::widget::shader::Program<Message> for OrbitElementsSimulation {
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

fn update(sim: &mut OrbitElementsSimulation, message: Message) {
    match message {
        Message::Tick => {}
        Message::Event(event) => {
            let rotate_angle = 5.0_f32.to_radians();
            let zoom_amount = 500.0;
            match event {
                iced::event::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    match key {
                        Key::Named(Named::ArrowLeft) => sim.program.camera.rotate_around_up(-rotate_angle),
                        Key::Named(Named::ArrowRight) => sim.program.camera.rotate_around_up(rotate_angle),
                        Key::Named(Named::ArrowUp) => sim.program.camera.rotate_vertically(-rotate_angle),
                        Key::Named(Named::ArrowDown) => sim.program.camera.rotate_vertically(rotate_angle),
                        Key::Character(ch) if ch == "+" || ch == "=" => sim.program.camera.dolly(-zoom_amount),
                        Key::Character(ch) if ch == "-" || ch == "_" => sim.program.camera.dolly(zoom_amount),
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
                iced::event::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right)) => {
                    sim.right_button_down = true;
                    sim.drag_start = sim.cursor_position;
                }
                iced::event::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Right)) => {
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

fn view(sim: &OrbitElementsSimulation) -> Element<'_, Message> {
    let scene = shader(sim).width(Length::Fill).height(Length::Fill);
    let info = text(&sim.validation_info).size(13);
    let detail = text(&sim.detail_info).size(11);

    container(column![info, detail, scene].spacing(4))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(4)
        .into()
}

fn main() -> iced::Result {
    env_logger::init();

    iced::application(OrbitElementsSimulation::new, update, view)
        .subscription(|_state: &OrbitElementsSimulation| {
            iced::Subscription::batch([
                time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
                iced::event::listen().map(Message::Event),
            ])
        })
        .run()
}
