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
use gui::viewer::{CameraControl, subscription};
use iced::mouse;
use iced::widget::{column, container, shader, text};
use iced::{Element, Length};
use nalgebra::{Point3, Vector3};

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Event(iced::event::Event),
    Shot(gui::screenshot::ShotMessage),
}

struct MoonPhasesSimulation {
    program: ProgramSimulation,
    validation_info: String,
    control: CameraControl,
    shot: gui::screenshot::AutoShot,
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

        // Camera from above to see both Sun and Moon directions. Close enough
        // that Earth stays legible while both direction lines (and their
        // labels) still fit inside the frame.
        let camera_eye = Point3::new(0.0, 0.0, 34_000.0);

        let mut core_sim = System::builder().build(full_moon_time);
        core_sim.simulation_speed = 0;

        let earth_radius = gui::model::system::EARTH_RADIUS_KM;

        // ECI frame
        core_sim.shapes.add_eci_frame(earth_radius * 1.2);

        // Sun direction line
        core_sim.shapes.add_sun_line(
            gui::model::FrameMode::Eci,
            [sun_dir[0] as f32, sun_dir[1] as f32, sun_dir[2] as f32],
            earth_radius * 1.6,
        );

        // Earth-Moon line
        let moon_line_len = earth_radius * 1.6;
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
            show_atmosphere: false,
            show_night_lights: false,
            show_bloom: false,
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
            control: CameraControl::default(),
            shot: gui::screenshot::AutoShot::from_env(),
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

fn update(sim: &mut MoonPhasesSimulation, message: Message) -> iced::Task<Message> {
    match message {
        Message::Tick => {
            if let Some(task) = sim.shot.on_frame() {
                return task.map(Message::Shot);
            }
        }
        Message::Event(event) => {
            sim.control.handle_event(&event, &mut sim.program.camera);
        }
        Message::Shot(msg) => {
            if let Some(task) = sim.shot.handle(msg) {
                return task.map(Message::Shot);
            }
        }
    }
    iced::Task::none()
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

    let mut app = iced::application(MoonPhasesSimulation::new, update, view);
    if let Some(size) = gui::screenshot::window_size() {
        app = app.window_size(size);
    }
    app.subscription(|_state: &MoonPhasesSimulation| {
        subscription(|_| Message::Tick, Message::Event)
    })
    .run()
}
