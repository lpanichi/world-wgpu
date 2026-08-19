/// Winter Solstice validation example.
///
/// Validates that on the December solstice (~day 355):
/// - Solar declination is ≈ -23.44° (maximum)
/// - The subsolar point is at latitude ≈ -23.44° (Tropic of Capricorn)
/// - The Sun direction has a significant -Z component in ECI
/// - Earth-Sun line tilts southward from the equatorial plane
use chrono::{TimeZone, Utc};
use gui::astro::Astral;
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::FrameMode;
use gui::model::system::System;
use gui::simulation::Simulation as ProgramSimulation;
use gui::viewer::{CameraControl, subscription};
use iced::mouse;
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
    control: CameraControl,
}

impl SolsticeSimulation {
    fn new() -> Self {
        // December 21, 2025 12:00 UTC — approximate winter solstice
        let solstice_time = Utc.with_ymd_and_hms(2025, 12, 21, 12, 0, 0).unwrap();
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
            "Subsolar (Tropic of Capricorn)",
        );
        core_sim.shapes.add_surface_line(
            subsolar_lat as f32,
            subsolar_lon as f32,
            earth_radius * 0.5,
            "Subsolar radial",
        );

        // Tropic of Capricorn line (≈23.44°N) — mark several points along it
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
            show_atmosphere: false,
            show_night_lights: false,
            show_bloom: false,
        };

        let validation_info = format!(
            "SUMMER SOLSTICE VALIDATION — Day {} ({}) | \
             Declination: {:.4}° (expect ≈-23.44°) | \
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
            control: CameraControl::default(),
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
            sim.control.handle_event(&event, &mut sim.program.camera);
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
            subscription(|_| Message::Tick, Message::Event)
        })
        .run()
}
