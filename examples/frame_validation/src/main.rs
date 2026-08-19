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
use gui::viewer::{CameraControl, subscription};
use iced::keyboard;
use iced::mouse;
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
    control: CameraControl,
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

        // Add directions to reference stars/points manually to ensure they match ECI axes X and Z
        // Add characteristic named stars
        let named_stars = gui::gpu::pipelines::planet::star_catalog::get_named_stars();
        for (name, dir) in named_stars.iter() {
            if ["Sirius", "Polaris", "Betelgeuse", "Rigel", "Vega", "Canopus"].contains(&name.as_str()) {
                core_sim.shapes.add_colored_star_line(
                    gui::model::FrameMode::Eci,
                    *dir,
                    earth_radius * 3.5,
                    [1.0, 0.9, 0.4],
                    name,
                );
            }
        }

        core_sim.shapes.add_colored_star_line(
            gui::model::FrameMode::Eci,
            [1.0, 0.0, 0.0],
            earth_radius * 2.7,
            [0.5, 0.8, 1.0],
            "Vernal Equinox (+X ECI)",
        );
        core_sim.shapes.add_colored_star_line(
            gui::model::FrameMode::Eci,
            [0.0, 0.0, 1.0],
            earth_radius * 2.7,
            [0.5, 0.8, 1.0],
            "North Celestial Pole (+Z ECI)",
        );


        // ECEF frame (rotates dynamically with Earth each render frame)
        core_sim.shapes.add_ecef_frame(earth_radius * 2.0);

        // Sub-points to validate ECEF base vectors
        core_sim.shapes.add_colored_surface_line(0.0, 0.0, earth_radius * 1.5, [0.5, 0.8, 1.0], "Prime Meridian (+X ECEF)");
        core_sim.shapes.add_colored_surface_line(0.0, 90.0, earth_radius * 1.5, [0.5, 0.8, 1.0], "90° East (+Y ECEF)");
        core_sim.shapes.add_colored_surface_line(90.0, 0.0, earth_radius * 1.5, [0.5, 0.8, 1.0], "North Pole (+Z ECEF)");


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
            show_atmosphere: false,
            show_night_lights: false,
            show_bloom: false,
        };

        let validation_info =
            "FRAME VALIDATION — Watch: ECI axes (long, labeled X/Y/Z) stay fixed, \
             ECEF axes (short, labeled X/Y/Z) rotate with Earth. \
             Ground stations rotate in ECI view. Press 'F' to toggle ECI/ECEF. Speed: 500x"
                .to_string();

        Self {
            program,
            validation_info,
            control: CameraControl::default(),
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
            let is_frame_toggle = matches!(
                &event,
                iced::event::Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Character(ch),
                    ..
                }) if ch == "f" || ch == "F"
            );

            if !sim.control.handle_event(&event, &mut sim.program.camera) && is_frame_toggle {
                sim.program.frame_mode = match sim.program.frame_mode {
                    FrameMode::Eci => FrameMode::Ecef,
                    FrameMode::Ecef => FrameMode::Eci,
                };
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
            subscription(|_| Message::Tick, Message::Event)
        })
        .run()
}
