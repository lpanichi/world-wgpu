/// Earth Texture Placement validation example.
///
/// Validates that the Earth texture is mapped correctly:
/// - (0°,0°) should be in the Gulf of Guinea off the coast of West Africa
/// - (48.86°N, 2.35°E) is Paris, France
/// - (40.71°N, -74.01°W) is New York City
/// - (-33.87°S, 151.21°E) is Sydney, Australia
/// - (35.68°N, 139.69°E) is Tokyo, Japan
/// - Ground stations at these known locations should visually match the texture
///
/// Also validates that ECEF frame aligns: the ECEF X-axis should point
/// through the Greenwich meridian (0° longitude).
use chrono::{TimeZone, Utc};
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::ground_station::GroundStation;
use gui::model::geo::lat_lon_to_ecef;
use gui::model::system::System;
use gui::model::FrameMode;
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
}

struct EarthTextureSimulation {
    program: ProgramSimulation,
    validation_info: String,
    control: CameraControl,
}

impl EarthTextureSimulation {
    fn new() -> Self {
        let sim_time = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        // Known cities as ground stations for visual texture verification
        let cities = vec![
            ("Gulf of Guinea (0,0)", 0.0_f32, 0.0_f32),
            ("Paris", 48.86, 2.35),
            ("New York", 40.71, -74.01),
            ("Sydney", -33.87, 151.21),
            ("Tokyo", 35.68, 139.69),
            ("Cape Town", -33.93, 18.42),
            ("São Paulo", -23.55, -46.63),
            ("North Pole", 90.0, 0.0),
            ("South Pole", -90.0, 0.0),
        ];

        let mut builder = System::builder();
        for (name, lat, lon) in &cities {
            let mut station = GroundStation::new(*name, *lat, *lon);
            station.show_cone = false;
            station.cube_size = 200.0;
            builder = builder.add_ground_station(station);
        }

        let mut core_sim = builder.build(sim_time);
        core_sim.simulation_speed = 0;

        let earth_radius = gui::model::system::EARTH_RADIUS_KM;

        // ECEF frame for reference — in ECEF mode, X goes through Greenwich
        core_sim.shapes.add_ecef_frame(earth_radius * 1.5);

        // Mark each city with a surface point
        for (name, lat, lon) in &cities {
            core_sim.shapes.add_surface_point(*lat, *lon, *name);
        }

        // Equator ring
        for lon in (-180..=180).step_by(10) {
            core_sim.shapes.add_surface_point(0.0, lon as f32, "");
        }

        // Prime meridian line
        for lat in (-90..=90).step_by(10) {
            core_sim.shapes.add_surface_point(lat as f32, 0.0, "");
        }

        // Camera looking at Gulf of Guinea
        let gulf_pos = lat_lon_to_ecef(0.0, 0.0);
        let gulf_dir = Vector3::new(gulf_pos[0], gulf_pos[1], gulf_pos[2]).normalize();
        let camera_eye = Point3::from(gulf_dir * 20_000.0);

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

        let validation_info = "EARTH TEXTURE VALIDATION — Ground stations mark known cities. \
             Verify visually: markers should align with geographic features on the texture. \
             (0°,0°) = Gulf of Guinea | Paris(48.9°N,2.4°E) | NYC(40.7°N,74°W) | \
             Sydney(33.9°S,151.2°E) | Tokyo(35.7°N,139.7°E)"
            .to_string();

        Self {
            program,
            validation_info,
            control: CameraControl::default(),
        }
    }
}

impl iced::widget::shader::Program<Message> for EarthTextureSimulation {
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

fn update(sim: &mut EarthTextureSimulation, message: Message) {
    match message {
        Message::Tick => {}
        Message::Event(event) => {
            sim.control.handle_event(&event, &mut sim.program.camera);
        }
    }
}

fn view(sim: &EarthTextureSimulation) -> Element<'_, Message> {
    let scene = shader(sim).width(Length::Fill).height(Length::Fill);
    let info = text(&sim.validation_info).size(13);

    container(column![info, scene].spacing(4))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(4)
        .into()
}

fn main() -> iced::Result {
    env_logger::init();

    iced::application(EarthTextureSimulation::new, update, view)
        .subscription(|_state: &EarthTextureSimulation| {
            subscription(|_| Message::Tick, Message::Event)
        })
        .run()
}
