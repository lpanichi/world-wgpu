use chrono::{TimeZone, Utc};
use gui::astro::Astral;
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::{
    ground_station::GroundStation, orbit::Orbit, satellite::Satellite, system::System,
};
use gui::simulation::{FrameMode, Simulation as ProgramSimulation};
use iced::mouse;
use iced::time;
use iced::widget::{container, shader};
use iced::{Element, Length};

#[derive(Debug, Clone, Copy)]
enum Message {
    Tick,
}

struct VernalEquinoxSimulation {
    program: ProgramSimulation,
}

impl VernalEquinoxSimulation {
    fn new() -> Self {
        let vernal_time = Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap();

        let (subsolar_lat, subsolar_lon) = Astral::subsolar_point(79, 12.0);

        let mut core_sim = System::builder()
            .add_orbit(
                Orbit::builder(
                    gui::model::system::EARTH_RADIUS_KM + 900.0,
                    Orbit::circular_period_seconds(gui::model::system::EARTH_RADIUS_KM + 900.0),
                )
                .name("Vernal Orbit")
                .inclination(98.0)
                .raan(0.0)
                .arg_perigee(0.0)
                .show_orbit(true)
                .add_satellite(Satellite::builder("VernalSat").phase_offset(0.0).build())
                .build(),
            )
            .add_ground_station(GroundStation::new(
                "Subsolar Station",
                subsolar_lat as f32,
                subsolar_lon as f32,
            ))
            .build(vernal_time);

        core_sim.simulation_speed = 120;

        let camera_distance = gui::model::system::EARTH_RADIUS_KM + 10_000.0;
        let camera = Camera::new(
            [
                -camera_distance * 0.7,
                -camera_distance * 0.7,
                camera_distance * 0.35,
            ]
            .into(),
            [0.0, 0.0, 0.0].into(),
            1600.0,
            900.0,
        );

        let mut program = ProgramSimulation {
            system: core_sim,
            camera,
            satellite_mode: SatelliteRenderMode::Dot,
            frame_mode: FrameMode::Eci,
            ecef_reference_earth_angle: 0.0,
            paused: false,
            time_scale: 120.0,
            pick_radius_scale: 1.0,
        };

        program.set_time_scale(120.0);

        Self { program }
    }

    fn tick(&mut self) {
        if !self.program.paused {
            self.program.tick();
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

fn update(simulation: &mut VernalEquinoxSimulation, message: Message) {
    match message {
        Message::Tick => simulation.tick(),
    }
}

fn view(simulation: &VernalEquinoxSimulation) -> Element<'_, Message> {
    let content = shader(simulation).width(Length::Fill).height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn main() -> iced::Result {
    env_logger::init();

    iced::application(VernalEquinoxSimulation::new, update, view)
        .subscription(|_state: &VernalEquinoxSimulation| {
            iced::Subscription::batch([
                time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
            ])
        })
        .run()
}
