use chrono::Utc;
use env_logger::Env;

use gui::{
    gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode},
    model::{
        ground_station::GroundStation,
        orbit::Orbit,
        satellite::Satellite,
        simulation::{EARTH_RADIUS_KM, Simulation},
    },
};
use iced::{
    Element,
    Length::Fill,
    Theme,
    keyboard::{self, Key},
    time::{self, milliseconds},
    widget::{button, column, container, pane_grid, row, scrollable, shader, text, text_input},
};
use log::{debug, info};

mod program;
use crate::program::SelectedObject;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelMode {
    Builder,
    Manager,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BuilderPane {
    None,
    Orbit,
    Station,
    Satellite,
}

#[derive(Clone)]
enum Message {
    KeyboardEvent(keyboard::Event),
    Event(iced::event::Event),
    Tick,
    OnObjectSelected(SelectedObject, Option<f32>),
    PaneClicked(pane_grid::Pane),
    PaneDragged(pane_grid::DragEvent),
    PaneResized(pane_grid::ResizeEvent),
    ToggleFrame,
    SwitchMode(PanelMode),
    ShowBuilderPane(BuilderPane),
    CreateOrbit,
    CreateStation,
    CreateOrbitSatellite,
    DeleteOrbit(usize),
    DeleteStation(usize),
    DeleteSatellite(usize, usize),
    OrbitAltitudeInput(String),
    OrbitInclinationInput(String),
    OrbitRaanInput(String),
    OrbitArgPerigeeInput(String),
    StationNameInput(String),
    StationLatInput(String),
    StationLonInput(String),
    SatelliteNameInput(String),
    SatelliteOrbitIndexInput(String),
    TogglePause,
    IncreaseTimeScale,
    DecreaseTimeScale,
    ResetTimeScale,
    ResetTime,
}

#[derive(Clone, Copy, Debug)]
struct PaneState {
    id: usize,
}

impl PaneState {
    fn new(id: usize) -> Self {
        Self { id }
    }
}

struct Textured {
    program: program::Program,
    panes: pane_grid::State<PaneState>,
    focus: Option<pane_grid::Pane>,
    status_message: String,
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
    selected_object: SelectedObject,
    selected_hit_distance: Option<f32>,
    viewport_size: (f32, f32),

    panel_mode: PanelMode,
    builder_pane: BuilderPane,

    // Builder widget state
    orbit_altitude_input: String,
    orbit_inclination_input: String,
    orbit_raan_input: String,
    orbit_arg_perigee_input: String,
    station_name_input: String,
    station_lat_input: String,
    station_lon_input: String,
    satellite_name_input: String,
    satellite_orbit_index_input: String,
}

impl Textured {
    fn update(&mut self, message: Message) {
        match message {
            Message::KeyboardEvent(event) => self.handle_keyboard_event(event),
            Message::Event(event) => self.handle_event(event),
            Message::Tick => {
                if !self.program.paused {
                    self.program.tick();
                }
            }
            Message::OnObjectSelected(object, hit_distance) => {
                self.handle_object_selected(object, hit_distance)
            }
            Message::PaneClicked(pane) => self.focus = Some(pane),
            Message::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.drop(pane, target);
            }
            Message::PaneDragged(_) => {}
            Message::PaneResized(event) => {
                self.panes.resize(event.split, event.ratio);
            }

            Message::ToggleFrame => {
                let current_phase = self.program.earth_rotation_phase();
                match self.program.frame_mode {
                    crate::program::FrameMode::Eci => {
                        // Enter ECEF: sync reference to avoid jump.
                        self.program.ecef_reference_earth_angle = current_phase;
                        self.program.frame_mode = crate::program::FrameMode::Ecef;
                    }
                    crate::program::FrameMode::Ecef => {
                        // Leave ECEF: stop automatic earth-locked updates.
                        self.program.frame_mode = crate::program::FrameMode::Eci;
                    }
                }
                self.status_message = format!("Frame mode: {:?}", self.program.frame_mode);
            }
            Message::TogglePause => {
                self.program.toggle_pause();
                self.status_message = if self.program.paused {
                    "Paused".to_string()
                } else {
                    "Running".to_string()
                };
            }
            Message::IncreaseTimeScale => {
                let new_scale = if self.program.time_scale < 1.0 {
                    1.0
                } else {
                    self.program.time_scale * 2.0
                };
                self.program.set_time_scale(new_scale);
                self.status_message = format!("Simulation speed: {:.1}x", self.program.time_scale);
            }
            Message::DecreaseTimeScale => {
                self.program.set_time_scale(self.program.time_scale * 0.5);
                self.status_message = format!("Simulation speed: {:.1}x", self.program.time_scale);
            }
            Message::ResetTimeScale => {
                self.program.set_time_scale(1.0);
                self.status_message = "Simulation speed reset to 1x".to_string();
            }
            Message::ResetTime => {
                self.program.reset_time();
                self.status_message = "Time reset".to_string();
            }
            Message::SwitchMode(mode) => {
                self.panel_mode = mode;
                self.status_message = format!("Switched to {:?} mode", mode);
            }
            Message::ShowBuilderPane(builder_pane) => {
                self.builder_pane = builder_pane;
                self.status_message = format!("Builder pane: {:?}", builder_pane);
            }
            Message::OrbitAltitudeInput(value) => self.orbit_altitude_input = value,
            Message::OrbitInclinationInput(value) => self.orbit_inclination_input = value,
            Message::OrbitRaanInput(value) => self.orbit_raan_input = value,
            Message::OrbitArgPerigeeInput(value) => self.orbit_arg_perigee_input = value,
            Message::StationNameInput(value) => self.station_name_input = value,
            Message::StationLatInput(value) => self.station_lat_input = value,
            Message::StationLonInput(value) => self.station_lon_input = value,
            Message::SatelliteNameInput(value) => self.satellite_name_input = value,
            Message::SatelliteOrbitIndexInput(value) => self.satellite_orbit_index_input = value,
            Message::CreateOrbit => {
                let altitude = self.orbit_altitude_input.parse::<f32>().unwrap_or(500.0);
                let inclination = self.orbit_inclination_input.parse::<f32>().unwrap_or(20.0);
                let raan = self.orbit_raan_input.parse::<f32>().unwrap_or(30.0);
                let arg_perigee = self.orbit_arg_perigee_input.parse::<f32>().unwrap_or(0.0);

                let semi_major_axis = EARTH_RADIUS_KM + altitude;
                let period_seconds = Orbit::circular_period_seconds(semi_major_axis);

                self.modify_model(|model| {
                    model.orbits.push(
                        Orbit::builder(semi_major_axis, period_seconds)
                            .inclination(inclination)
                            .raan(raan)
                            .arg_perigee(arg_perigee)
                            .show_orbit(true)
                            .add_satellite(Satellite::builder("Sat-1").phase_offset(0.0).build())
                            .build(),
                    );
                });

                self.status_message = format!(
                    "Created orbit, total {}",
                    self.program.simulation.orbits.len()
                );
            }
            Message::CreateStation => {
                let name = if self.station_name_input.trim().is_empty() {
                    format!(
                        "Station {}",
                        self.program.simulation.ground_stations.len() + 1
                    )
                } else {
                    self.station_name_input.clone()
                };
                let lat = self.station_lat_input.parse::<f32>().unwrap_or(0.0);
                let lon = self.station_lon_input.parse::<f32>().unwrap_or(0.0);

                self.modify_model(|model| {
                    model
                        .ground_stations
                        .push(GroundStation::new(name.clone(), lat, lon));
                });

                self.status_message = format!("Created station '{}'", name);
            }
            Message::CreateOrbitSatellite => {
                let orbit_index = self
                    .satellite_orbit_index_input
                    .parse::<usize>()
                    .ok()
                    .filter(|idx| *idx < self.program.simulation.orbits.len());

                if let Some(idx) = orbit_index {
                    let sat_name = if self.satellite_name_input.trim().is_empty() {
                        format!("Sat-{}", idx + 1)
                    } else {
                        self.satellite_name_input.clone()
                    };
                    self.modify_model(|model| {
                        if let Some(orbit) = model.orbits.get_mut(idx) {
                            orbit.satellites.push(
                                Satellite::builder(sat_name.clone())
                                    .phase_offset(0.0)
                                    .build(),
                            );
                        }
                    });
                    self.status_message =
                        format!("Created satellite '{}' in orbit {}", sat_name, idx);
                } else {
                    self.status_message = "Invalid orbit index for satellite creation".to_string();
                }
            }
            Message::DeleteOrbit(index) => {
                self.modify_model(|model| {
                    if index < model.orbits.len() {
                        model.orbits.remove(index);
                    }
                });
            }
            Message::DeleteStation(index) => {
                self.modify_model(|model| {
                    if index < model.ground_stations.len() {
                        model.ground_stations.remove(index);
                    }
                });
            }
            Message::DeleteSatellite(orbit_index, sat_index) => {
                self.modify_model(|model| {
                    if let Some(orbit) = model.orbits.get_mut(orbit_index) {
                        if sat_index < orbit.satellites.len() {
                            orbit.satellites.remove(sat_index);
                        }
                    }
                });
            }
        }
    }

    fn modify_model(&mut self, mut f: impl FnMut(&mut Simulation)) {
        let mut simulation = self.program.simulation.clone();
        f(&mut simulation);
        self.program.simulation = simulation;
    }

    fn handle_object_selected(&mut self, object: SelectedObject, hit_distance: Option<f32>) {
        self.selected_object = object.clone();
        self.selected_hit_distance = hit_distance;
        self.status_message = match &object {
            SelectedObject::Earth => "Earth selected".to_string(),
            SelectedObject::Satellite(name) => format!("Satellite selected: {}", name),
            SelectedObject::GroundStation(name) => format!("Ground station selected: {}", name),
            SelectedObject::None => "No object selected".to_string(),
        };
        info!(
            "OnObjectSelected: {:?} at distance={:?}",
            object, hit_distance
        );
    }

    fn handle_keyboard_event(&mut self, event: keyboard::Event) {
        if let keyboard::Event::KeyPressed { key, .. } = event {
            let delta_angle = 5.0_f32.to_radians();
            match key {
                Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
                    self.program.camera.rotate_around_up(-delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                    self.program.camera.rotate_around_up(delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                    self.program.camera.rotate_vertically(-delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                    self.program.camera.rotate_vertically(delta_angle);
                }
                Key::Named(iced::keyboard::key::Named::Space) => {
                    self.program.satellite_mode = match self.program.satellite_mode {
                        SatelliteRenderMode::Cube => SatelliteRenderMode::Dot,
                        SatelliteRenderMode::Dot => SatelliteRenderMode::Cube,
                    };
                }
                Key::Character(ch) if ch == "+" || ch == "=" => {
                    self.update(Message::IncreaseTimeScale);
                }
                Key::Character(ch) if ch == "-" || ch == "_" => {
                    self.update(Message::DecreaseTimeScale);
                }
                _ => (),
            }
        }
    }

    fn handle_event(&mut self, event: iced::event::Event) {
        match event {
            iced::event::Event::Window(iced::window::Event::Resized(size)) => {
                self.viewport_size = (size.width as f32, size.height as f32);
                self.program
                    .camera
                    .change_aspect(size.width as f32, size.height as f32);
                info!(
                    "Window resized: width={} height={}, updated camera aspect={:.3}",
                    size.width, size.height, self.program.camera.aspect
                );
            }
            iced::event::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                let x = position.x;
                let y = position.y;
                if self.right_button_down {
                    if let Some((prev_x, prev_y)) = self.drag_start {
                        let dx = x - prev_x;
                        let dy = y - prev_y;
                        self.program.camera.rotate_around_up(-dx * 0.005);
                        self.program.camera.rotate_vertically(-dy * 0.005);
                        self.drag_start = Some((x, y));
                    } else {
                        self.drag_start = Some((x, y));
                    }
                }
                self.cursor_position = Some((x, y));
            }
            iced::event::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Right,
            )) => {
                self.right_button_down = true;
                self.drag_start = self.cursor_position;
            }
            iced::event::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Right,
            )) => {
                self.right_button_down = false;
                self.drag_start = None;
            }
            iced::event::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            )) => {
                // left button down starts potential selection; no drag behavior.
            }
            iced::event::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )) => {
                if let Some(cursor_pos) = self.cursor_position {
                    if let Some((x, y, w, h)) = self.shader_pane_region() {
                        if cursor_pos.0 >= x
                            && cursor_pos.0 <= x + w
                            && cursor_pos.1 >= y
                            && cursor_pos.1 <= y + h
                        {
                            let local_pos = (cursor_pos.0 - x, cursor_pos.1 - y);

                            if let Some((origin, direction, cursor_ndc)) =
                                self.program.world_ray_from_cursor(local_pos, (w, h))
                            {
                                let (selected, hit_distance) =
                                    self.program
                                        .pick_object(origin, direction, cursor_ndc, (w, h));
                                self.update(Message::OnObjectSelected(selected, hit_distance));
                            }
                        }
                    }
                }
            }
            iced::event::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                let step_km = 50.0;
                let amount = match delta {
                    iced::mouse::ScrollDelta::Lines { y, .. } => y.signum() * step_km,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => y.signum() * step_km,
                };
                self.program.camera.dolly(amount);
            }
            _ => (),
        }
    }

    fn shader_pane_region(&self) -> Option<(f32, f32, f32, f32)> {
        let total_size = self.viewport_size;
        if total_size.0 <= 0.0 || total_size.1 <= 0.0 {
            return None;
        }

        let bounds = iced::Size::new(total_size.0, total_size.1);
        let regions = self.panes.layout().pane_regions(4.0, 0.0, bounds);

        let shader_pane =
            self.panes.iter().find_map(
                |(pane, state)| {
                    if state.id == 0 { Some(*pane) } else { None }
                },
            )?;

        let region = regions.get(&shader_pane)?;

        Some((region.x, region.y, region.width, region.height))
    }

    fn control_panel(&self) -> Element<'_, Message> {
        let meta = &self.program.simulation;

        let mode_row = row![
            button("Builder Mode").on_press(Message::SwitchMode(PanelMode::Builder)),
            button("Manager Mode").on_press(Message::SwitchMode(PanelMode::Manager)),
        ]
        .spacing(8);

        let builder_toolbar = row![
            button("Station").on_press(Message::ShowBuilderPane(BuilderPane::Station)),
            button("Orbit").on_press(Message::ShowBuilderPane(BuilderPane::Orbit)),
            button("Satellite").on_press(Message::ShowBuilderPane(BuilderPane::Satellite)),
        ]
        .spacing(8);

        let builder_panel = match self.builder_pane {
            BuilderPane::Orbit => column![
                text("Orbit Builder").size(14),
                text_input("Altitude (km)", &self.orbit_altitude_input)
                    .on_input(Message::OrbitAltitudeInput),
                text_input("Inclination (deg)", &self.orbit_inclination_input)
                    .on_input(Message::OrbitInclinationInput),
                text_input("RAAN (deg)", &self.orbit_raan_input).on_input(Message::OrbitRaanInput),
                text_input("Arg Perigee (deg)", &self.orbit_arg_perigee_input)
                    .on_input(Message::OrbitArgPerigeeInput),
                button("Create Orbit").on_press(Message::CreateOrbit),
            ]
            .spacing(8),
            BuilderPane::Station => column![
                text("Station Builder").size(14),
                text_input("Name", &self.station_name_input).on_input(Message::StationNameInput),
                text_input("Latitude", &self.station_lat_input).on_input(Message::StationLatInput),
                text_input("Longitude", &self.station_lon_input).on_input(Message::StationLonInput),
                button("Create Station").on_press(Message::CreateStation),
            ]
            .spacing(8),
            BuilderPane::Satellite => column![
                text("Satellite Builder").size(14),
                text_input("Name", &self.satellite_name_input)
                    .on_input(Message::SatelliteNameInput),
                text_input("Orbit index", &self.satellite_orbit_index_input)
                    .on_input(Message::SatelliteOrbitIndexInput),
                button("Create Satellite").on_press(Message::CreateOrbitSatellite),
            ]
            .spacing(8),
            BuilderPane::None => column![text("Select a builder resource above")].spacing(8),
        };

        let manager_panel = {
            let mut panel = column![text("Resource Manager").size(16)].spacing(8);

            panel = panel.push(text("Orbits").size(14));
            for (i, orbit) in meta.orbits.iter().enumerate() {
                panel = panel.push(
                    row![
                        text(format!(
                            "Orbit {}: a={:.1}, inc={:.1}, sats={} ",
                            i,
                            orbit.semi_major_axis,
                            orbit.inclination_deg,
                            orbit.satellites.len()
                        )),
                        button("Delete").on_press(Message::DeleteOrbit(i)),
                    ]
                    .spacing(6),
                );
            }

            panel = panel.push(text("Stations").size(14));
            for (i, station) in meta.ground_stations.iter().enumerate() {
                panel = panel.push(
                    row![
                        text(format!(
                            "{} @ ({:.1},{:.1})",
                            station.name, station.latitude_deg, station.longitude_deg
                        )),
                        button("Delete").on_press(Message::DeleteStation(i)),
                    ]
                    .spacing(6),
                );
            }

            panel = panel.push(text("Satellites").size(14));
            for (orbit_index, orbit) in meta.orbits.iter().enumerate() {
                for (sat_index, satellite) in orbit.satellites.iter().enumerate() {
                    panel = panel.push(
                        row![
                            text(format!("orbit {} / {}", orbit_index, satellite.name)),
                            button("Delete")
                                .on_press(Message::DeleteSatellite(orbit_index, sat_index)),
                        ]
                        .spacing(6),
                    );
                }
            }

            panel
        };

        let mode_content = match self.panel_mode {
            PanelMode::Builder => column![builder_toolbar, builder_panel].spacing(10),
            PanelMode::Manager => manager_panel,
        };

        let content = column![
            text(format!("Selected object: {}", self.status_message)).size(16),
            text(format!(
                "Select-ray distance: {}",
                match self.selected_hit_distance {
                    Some(d) => format!("{:.3}", d),
                    None => "N/A".to_string(),
                }
            ))
            .size(14),
            text(format!(
                "Altitude above Earth: {:.3} km",
                self.program.camera.eye.coords.norm() - gui::model::simulation::EARTH_RADIUS_KM
            ))
            .size(14),
            text(format!("Orbit count: {}", meta.orbits.len())).size(14),
            text(format!("Station count: {}", meta.ground_stations.len())).size(14),
            text(format!("Sat render: {:?}", self.program.satellite_mode)).size(14),
            text(format!("Simulation speed: {:.1}x", self.program.time_scale)).size(14),
            text(format!(
                "Time: {:.2} (paused={})",
                self.program.elapsed_time(),
                self.program.paused
            ))
            .size(14),
            row![
                button("Pause/Resume").on_press(Message::TogglePause),
                button("Reset Time").on_press(Message::ResetTime),
                button("Toggle Frame").on_press(Message::ToggleFrame),
            ]
            .spacing(8),
            row![
                button("Slower").on_press(Message::DecreaseTimeScale),
                button("Faster").on_press(Message::IncreaseTimeScale),
                button("1x").on_press(Message::ResetTimeScale),
            ]
            .spacing(8),
            mode_row,
            mode_content,
        ]
        .spacing(10)
        .padding(10);

        scrollable(content).height(Fill).into()
    }

    fn view(&self) -> Element<'_, Message> {
        let pane_grid = pane_grid::PaneGrid::new(&self.panes, |_, pane_state, _| {
            let content: Element<'_, Message> = match pane_state.id {
                0 => shader(&self.program).width(Fill).height(Fill).into(),
                1 => self.control_panel(),
                _ => text("Unknown pane").into(),
            };

            pane_grid::Content::new(content)
        })
        .width(Fill)
        .height(Fill)
        .spacing(4)
        .on_click(Message::PaneClicked)
        .on_drag(Message::PaneDragged)
        .on_resize(4, Message::PaneResized);

        container(pane_grid).padding(4).into()
    }
}

impl Default for Textured {
    fn default() -> Self {
        let earth_radius = EARTH_RADIUS_KM;
        let orbit1_a = earth_radius + 500.0;
        let orbit2_a = earth_radius + 800.0;

        let mut simulation = Simulation::builder()
            .add_orbit(
                Orbit::builder(orbit1_a, Orbit::circular_period_seconds(orbit1_a))
                    .inclination(20.0)
                    .raan(30.0)
                    .arg_perigee(0.0)
                    .show_orbit(true)
                    .add_satellite(Satellite::builder("Sat-1").phase_offset(0.0).build())
                    .add_satellite(Satellite::builder("Sat-2").phase_offset(2.0).build())
                    .build(),
            )
            .add_orbit(
                Orbit::builder(orbit2_a, Orbit::circular_period_seconds(orbit2_a))
                    .inclination(45.0)
                    .raan(80.0)
                    .arg_perigee(30.0)
                    .show_orbit(true)
                    .add_satellite(Satellite::builder("Sat-3").phase_offset(2.0).build())
                    .build(),
            )
            // Single station in Paris (lat, lon): 48.8566° N, 2.3522° E
            .add_ground_station(GroundStation::new("Paris Station", 48.8566, 2.3522))
            .build(Utc::now());
        simulation.simulation_speed = 120;

        let camera_distance = earth_radius + 10_000.0;
        let camera = Camera::new(
            [
                -camera_distance * 0.7,
                -camera_distance * 0.7,
                camera_distance * 0.35,
            ]
            .into(),
            [0.0, 0.0, 0.0].into(),
            200.,
            200.,
        );

        let (mut panes, root_pane) = pane_grid::State::new(PaneState::new(0));
        let _ = panes.split(pane_grid::Axis::Vertical, root_pane, PaneState::new(1));

        Self {
            program: program::Program {
                simulation,
                camera,
                satellite_mode: SatelliteRenderMode::Dot,
                frame_mode: crate::program::FrameMode::Eci,
                ecef_reference_earth_angle: 0.0,
                paused: false,
                time_scale: 120.0,
                pick_radius_scale: 2.0,
            },
            panes,
            focus: Some(root_pane),
            status_message: "No selection".to_string(),
            panel_mode: PanelMode::Builder,
            builder_pane: BuilderPane::None,
            orbit_altitude_input: "500.0".to_string(),
            orbit_inclination_input: "20.0".to_string(),
            orbit_raan_input: "30.0".to_string(),
            orbit_arg_perigee_input: "0.0".to_string(),
            station_name_input: "Station".to_string(),
            station_lat_input: "0.0".to_string(),
            station_lon_input: "0.0".to_string(),
            satellite_name_input: "Sat".to_string(),
            satellite_orbit_index_input: "0".to_string(),
            cursor_position: None,
            drag_start: None,
            right_button_down: false,
            selected_object: SelectedObject::None,
            selected_hit_distance: None,
            viewport_size: (200.0, 200.0),
        }
    }
}

fn main() -> iced::Result {
    // Enable debug logs only for this example crate ("textured").
    // Override with RUST_LOG when needed, e.g. RUST_LOG=info or RUST_LOG=textured=debug
    env_logger::Builder::from_env(Env::default().default_filter_or("textured=debug")).init();

    debug!("logging initialized for module: {}", module_path!());

    iced::application(Textured::default, Textured::update, Textured::view)
        .subscription(|_state: &Textured| {
            iced::Subscription::batch([
                iced::keyboard::listen().map(Message::KeyboardEvent),
                iced::event::listen().map(Message::Event),
                time::every(milliseconds(16)).map(|_| Message::Tick),
            ])
        })
        .theme(Theme::KanagawaDragon)
        .run()
}
