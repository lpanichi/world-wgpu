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
use gui::model::shapes::lat_lon_to_ecef;
use gui::model::system::System;
use gui::simulation::{FrameMode, Simulation as ProgramSimulation};
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

struct EarthTextureSimulation {
    program: ProgramSimulation,
    validation_info: String,
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
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
        };

        let validation_info = format!(
            "EARTH TEXTURE VALIDATION — Ground stations mark known cities. \
             Verify visually: markers should align with geographic features on the texture. \
             (0°,0°) = Gulf of Guinea | Paris(48.9°N,2.4°E) | NYC(40.7°N,74°W) | \
             Sydney(33.9°S,151.2°E) | Tokyo(35.7°N,139.7°E)"
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
            iced::Subscription::batch([
                time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
                iced::event::listen().map(Message::Event),
            ])
        })
        .run()
}
