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
    Kpi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BuilderPane {
    None,
    Orbit,
    Station,
    Satellite,
    RectSurface,
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
    // Builder: Orbit
    CreateOrbit,
    OrbitAltitudeInput(String),
    OrbitInclinationInput(String),
    OrbitRaanInput(String),
    OrbitArgPerigeeInput(String),
    // Builder: Station
    CreateStation,
    StationNameInput(String),
    StationLatInput(String),
    StationLonInput(String),
    // Builder: Satellite
    CreateOrbitSatellite,
    SatelliteNameInput(String),
    SatelliteOrbitIndexInput(String),
    // Builder: Rectangular surface
    CreateRectSurface,
    RectMinLatInput(String),
    RectMaxLatInput(String),
    RectMinLonInput(String),
    RectMaxLonInput(String),
    // Manager: delete
    DeleteOrbit(usize),
    DeleteStation(usize),
    DeleteSatellite(usize, usize),
    // Manager: tune orbit
    ToggleOrbitVisible(usize),
    ToggleOrbitFov(usize),
    ToggleOrbitFovFill(usize),
    OrbitFovAngleInput(usize, String),
    OrbitInclinationEdit(usize, String),
    OrbitRaanEdit(usize, String),
    // Manager: tune station
    StationMinElevationInput(usize, String),
    ToggleStationCone(usize),
    // Simulation controls
    TogglePause,
    IncreaseTimeScale,
    DecreaseTimeScale,
    ResetTimeScale,
    ResetTime,
    TogglePrecession,
    // Camera
    FollowSatellite(Option<(usize, usize)>),
    // KPI
    KpiStationIndexInput(String),
    KpiOrbitIndexInput(String),
    KpiSatIndexInput(String),
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
    error_message: String,
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
    selected_object: SelectedObject,
    selected_hit_distance: Option<f32>,
    viewport_size: (f32, f32),

    panel_mode: PanelMode,
    builder_pane: BuilderPane,

    // Builder: orbit
    orbit_altitude_input: String,
    orbit_inclination_input: String,
    orbit_raan_input: String,
    orbit_arg_perigee_input: String,
    // Builder: station
    station_name_input: String,
    station_lat_input: String,
    station_lon_input: String,
    // Builder: satellite
    satellite_name_input: String,
    satellite_orbit_index_input: String,
    // Builder: rectangular surface
    rect_min_lat_input: String,
    rect_max_lat_input: String,
    rect_min_lon_input: String,
    rect_max_lon_input: String,

    // Camera follow
    follow_satellite: Option<(usize, usize)>,
    /// Camera offset from satellite position when in follow mode.
    follow_offset: nalgebra::Vector3<f32>,

    // KPI state
    kpi_station_index: String,
    kpi_orbit_index: String,
    kpi_sat_index: String,
    /// Ring buffer of recent distance samples for the KPI plot.
    kpi_distance_history: Vec<(f32, f32)>,

    // Manager: focused resource after click
    manager_focus: Option<SelectedObject>,
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
                // Update camera follow
                if let Some((orbit_idx, sat_idx)) = self.follow_satellite {
                    if let Some(orbit) = self.program.simulation.orbits.get(orbit_idx) {
                        if let Some(sat) = orbit.satellites.get(sat_idx) {
                            let elapsed = self.program.elapsed_time();
                            let pos = orbit.position(elapsed, sat);
                            let sat_pos = nalgebra::Point3::new(pos[0], pos[1], pos[2]);
                            self.program.camera.eye = sat_pos + self.follow_offset;
                            self.program.camera.target = sat_pos;
                        }
                    }
                }
                // Update KPI distance history
                if let (Ok(si), Ok(oi), Ok(sati)) = (
                    self.kpi_station_index.parse::<usize>(),
                    self.kpi_orbit_index.parse::<usize>(),
                    self.kpi_sat_index.parse::<usize>(),
                ) {
                    let elapsed = self.program.elapsed_time();
                    if let Some(dist) = self
                        .program
                        .simulation
                        .station_satellite_distance(si, oi, sati, elapsed)
                    {
                        self.kpi_distance_history.push((elapsed, dist));
                        // Keep last 500 samples
                        if self.kpi_distance_history.len() > 500 {
                            self.kpi_distance_history.remove(0);
                        }
                    }
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
                        self.program.ecef_reference_earth_angle = current_phase;
                        self.program.frame_mode = crate::program::FrameMode::Ecef;
                    }
                    crate::program::FrameMode::Ecef => {
                        self.program.frame_mode = crate::program::FrameMode::Eci;
                    }
                }
                self.status_message = format!("Frame: {:?}", self.program.frame_mode);
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
                self.status_message = format!("Speed: {:.1}x", self.program.time_scale);
            }
            Message::DecreaseTimeScale => {
                self.program.set_time_scale(self.program.time_scale * 0.5);
                self.status_message = format!("Speed: {:.1}x", self.program.time_scale);
            }
            Message::ResetTimeScale => {
                self.program.set_time_scale(1.0);
                self.status_message = "Speed: 1x".to_string();
            }
            Message::ResetTime => {
                self.program.reset_time();
                self.kpi_distance_history.clear();
                self.status_message = "Time reset".to_string();
            }
            Message::TogglePrecession => {
                self.program.simulation.precession_enabled =
                    !self.program.simulation.precession_enabled;
                self.status_message = format!(
                    "Precession: {}",
                    if self.program.simulation.precession_enabled {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
            }
            Message::SwitchMode(mode) => {
                self.panel_mode = mode;
                self.manager_focus = None;
            }
            Message::ShowBuilderPane(builder_pane) => {
                self.builder_pane = builder_pane;
                self.error_message.clear();
            }
            // Builder inputs
            Message::OrbitAltitudeInput(value) => self.orbit_altitude_input = value,
            Message::OrbitInclinationInput(value) => self.orbit_inclination_input = value,
            Message::OrbitRaanInput(value) => self.orbit_raan_input = value,
            Message::OrbitArgPerigeeInput(value) => self.orbit_arg_perigee_input = value,
            Message::StationNameInput(value) => self.station_name_input = value,
            Message::StationLatInput(value) => self.station_lat_input = value,
            Message::StationLonInput(value) => self.station_lon_input = value,
            Message::SatelliteNameInput(value) => self.satellite_name_input = value,
            Message::SatelliteOrbitIndexInput(value) => self.satellite_orbit_index_input = value,
            Message::RectMinLatInput(value) => self.rect_min_lat_input = value,
            Message::RectMaxLatInput(value) => self.rect_max_lat_input = value,
            Message::RectMinLonInput(value) => self.rect_min_lon_input = value,
            Message::RectMaxLonInput(value) => self.rect_max_lon_input = value,
            // Builder: create
            Message::CreateOrbit => {
                let altitude = match self.orbit_altitude_input.parse::<f32>() {
                    Ok(v) if v > 0.0 && v < 100_000.0 => v,
                    _ => {
                        self.error_message =
                            "Altitude must be a positive number (km), e.g. 500".to_string();
                        return;
                    }
                };
                let inclination = match self.orbit_inclination_input.parse::<f32>() {
                    Ok(v) if (-180.0..=180.0).contains(&v) => v,
                    _ => {
                        self.error_message =
                            "Inclination must be between -180° and 180°".to_string();
                        return;
                    }
                };
                let raan = match self.orbit_raan_input.parse::<f32>() {
                    Ok(v) => v,
                    _ => {
                        self.error_message = "RAAN must be a valid number (degrees)".to_string();
                        return;
                    }
                };
                let arg_perigee = match self.orbit_arg_perigee_input.parse::<f32>() {
                    Ok(v) => v,
                    _ => {
                        self.error_message =
                            "Argument of perigee must be a valid number (degrees)".to_string();
                        return;
                    }
                };

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
                self.error_message.clear();
                self.status_message = format!(
                    "Created orbit at {:.0} km, total {}",
                    altitude,
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
                let lat = match self.station_lat_input.parse::<f32>() {
                    Ok(v) if (-90.0..=90.0).contains(&v) => v,
                    _ => {
                        self.error_message = "Latitude must be between -90° and 90°".to_string();
                        return;
                    }
                };
                let lon = match self.station_lon_input.parse::<f32>() {
                    Ok(v) if (-180.0..=180.0).contains(&v) => v,
                    _ => {
                        self.error_message = "Longitude must be between -180° and 180°".to_string();
                        return;
                    }
                };

                self.modify_model(|model| {
                    model
                        .ground_stations
                        .push(GroundStation::new(name.clone(), lat, lon));
                });
                self.error_message.clear();
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
                        format!(
                            "Sat-{}",
                            self.program.simulation.orbits[idx].satellites.len() + 1
                        )
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
                    self.error_message.clear();
                    self.status_message =
                        format!("Created satellite '{}' in orbit {}", sat_name, idx);
                } else {
                    self.error_message = format!(
                        "Invalid orbit index. Must be 0..{}",
                        self.program.simulation.orbits.len().saturating_sub(1)
                    );
                }
            }
            Message::CreateRectSurface => {
                let min_lat = match self.rect_min_lat_input.parse::<f32>() {
                    Ok(v) if (-90.0..=90.0).contains(&v) => v,
                    _ => {
                        self.error_message =
                            "Min latitude must be between -90° and 90°".to_string();
                        return;
                    }
                };
                let max_lat = match self.rect_max_lat_input.parse::<f32>() {
                    Ok(v) if (-90.0..=90.0).contains(&v) && v > min_lat => v,
                    _ => {
                        self.error_message =
                            "Max latitude must be > min latitude and between -90° and 90°"
                                .to_string();
                        return;
                    }
                };
                let min_lon = match self.rect_min_lon_input.parse::<f32>() {
                    Ok(v) if (-180.0..=180.0).contains(&v) => v,
                    _ => {
                        self.error_message =
                            "Min longitude must be between -180° and 180°".to_string();
                        return;
                    }
                };
                let max_lon = match self.rect_max_lon_input.parse::<f32>() {
                    Ok(v) if (-180.0..=180.0).contains(&v) && v > min_lon => v,
                    _ => {
                        self.error_message =
                            "Max longitude must be > min longitude and between -180° and 180°"
                                .to_string();
                        return;
                    }
                };
                // Rectangle is drawn as a feature; we store it as a ground station pair for now.
                // Store it as metadata in the status message. The actual rendering happens via features.
                self.error_message.clear();
                self.status_message = format!(
                    "Rect surface: ({:.1},{:.1}) to ({:.1},{:.1})",
                    min_lat, min_lon, max_lat, max_lon
                );
                // The rectangle will be rendered in the feature lines.
                // We'll store the rect specs in the simulation.
                // For now, let's add rectangle support to the simulation's stored rects.
                self.program
                    .simulation
                    .rect_surfaces
                    .push((min_lat, max_lat, min_lon, max_lon));
            }
            // Manager: delete
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
            // Manager: tune orbit
            Message::ToggleOrbitVisible(idx) => {
                if let Some(orbit) = self.program.simulation.orbits.get_mut(idx) {
                    orbit.show_orbit = !orbit.show_orbit;
                }
            }
            Message::ToggleOrbitFov(idx) => {
                if let Some(orbit) = self.program.simulation.orbits.get_mut(idx) {
                    orbit.show_fov = !orbit.show_fov;
                }
            }
            Message::ToggleOrbitFovFill(idx) => {
                if let Some(orbit) = self.program.simulation.orbits.get_mut(idx) {
                    orbit.fill_fov = !orbit.fill_fov;
                }
            }
            Message::OrbitFovAngleInput(idx, value) => {
                if let Ok(angle) = value.parse::<f32>() {
                    if let Some(orbit) = self.program.simulation.orbits.get_mut(idx) {
                        orbit.fov_half_angle_deg = angle.clamp(0.1, 89.0);
                    }
                }
            }
            Message::OrbitInclinationEdit(idx, value) => {
                if let Ok(inc) = value.parse::<f32>() {
                    if let Some(orbit) = self.program.simulation.orbits.get_mut(idx) {
                        orbit.inclination_deg = inc;
                    }
                }
            }
            Message::OrbitRaanEdit(idx, value) => {
                if let Ok(raan) = value.parse::<f32>() {
                    if let Some(orbit) = self.program.simulation.orbits.get_mut(idx) {
                        orbit.raan_deg = raan;
                    }
                }
            }
            // Manager: tune station
            Message::StationMinElevationInput(idx, value) => {
                if let Ok(elev) = value.parse::<f32>() {
                    if let Some(station) = self.program.simulation.ground_stations.get_mut(idx) {
                        station.min_elevation_deg = elev.clamp(0.0, 90.0);
                    }
                }
            }
            Message::ToggleStationCone(idx) => {
                if let Some(station) = self.program.simulation.ground_stations.get_mut(idx) {
                    station.show_cone = !station.show_cone;
                }
            }
            // Camera follow
            Message::FollowSatellite(target) => {
                match target {
                    Some((o, s)) => {
                        // Initialize offset along radial direction from satellite
                        if let Some(orbit) = self.program.simulation.orbits.get(o) {
                            if let Some(sat) = orbit.satellites.get(s) {
                                let elapsed = self.program.elapsed_time();
                                let pos = orbit.position(elapsed, sat);
                                let radial =
                                    nalgebra::Vector3::new(pos[0], pos[1], pos[2]).normalize();
                                self.follow_offset = radial * 200.0;
                            }
                        }
                        self.follow_satellite = Some((o, s));
                        self.status_message = format!("Following orbit {} sat {}", o, s);
                    }
                    None => {
                        self.follow_satellite = None;
                        // Reset camera to point at Earth
                        self.program.camera.target = nalgebra::Point3::origin();
                        self.status_message = "Free camera".to_string();
                    }
                }
            }
            // KPI
            Message::KpiStationIndexInput(v) => {
                self.kpi_station_index = v;
                self.kpi_distance_history.clear();
            }
            Message::KpiOrbitIndexInput(v) => {
                self.kpi_orbit_index = v;
                self.kpi_distance_history.clear();
            }
            Message::KpiSatIndexInput(v) => {
                self.kpi_sat_index = v;
                self.kpi_distance_history.clear();
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
            SelectedObject::Satellite(name) => format!("Satellite: {}", name),
            SelectedObject::GroundStation(name) => format!("Station: {}", name),
            SelectedObject::None => "No selection".to_string(),
        };
        // If a resource is clicked, switch to manager mode focused on that resource
        match &object {
            SelectedObject::Satellite(_) | SelectedObject::GroundStation(_) => {
                self.panel_mode = PanelMode::Manager;
                self.manager_focus = Some(object.clone());
            }
            _ => {
                self.manager_focus = None;
            }
        }
        info!(
            "OnObjectSelected: {:?} at distance={:?}",
            object, hit_distance
        );
    }

    fn rotate_follow_offset_horizontal(&mut self, angle: f32) {
        let rot = nalgebra::Rotation3::from_axis_angle(&nalgebra::Vector3::z_axis(), angle);
        self.follow_offset = rot * self.follow_offset;
    }

    fn rotate_follow_offset_vertical(&mut self, angle: f32) {
        let right = self
            .follow_offset
            .cross(&nalgebra::Vector3::z_axis().into_inner());
        if right.norm_squared() < 1e-8 {
            return;
        }
        let axis = nalgebra::Unit::new_normalize(right);
        let rot = nalgebra::Rotation3::from_axis_angle(&axis, angle);
        let new_offset = rot * self.follow_offset;
        // Prevent flipping past poles
        let up_dot = new_offset
            .normalize()
            .dot(&nalgebra::Vector3::z_axis().into_inner());
        if up_dot.abs() > 0.98 {
            return;
        }
        self.follow_offset = new_offset;
    }

    fn handle_keyboard_event(&mut self, event: keyboard::Event) {
        if let keyboard::Event::KeyPressed { key, .. } = event {
            let delta_angle = 5.0_f32.to_radians();
            match key {
                Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
                    if self.follow_satellite.is_some() {
                        self.rotate_follow_offset_horizontal(-delta_angle);
                    } else {
                        self.program.camera.rotate_around_up(-delta_angle);
                    }
                }
                Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                    if self.follow_satellite.is_some() {
                        self.rotate_follow_offset_horizontal(delta_angle);
                    } else {
                        self.program.camera.rotate_around_up(delta_angle);
                    }
                }
                Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                    if self.follow_satellite.is_some() {
                        self.rotate_follow_offset_vertical(-delta_angle);
                    } else {
                        self.program.camera.rotate_vertically(-delta_angle);
                    }
                }
                Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                    if self.follow_satellite.is_some() {
                        self.rotate_follow_offset_vertical(delta_angle);
                    } else {
                        self.program.camera.rotate_vertically(delta_angle);
                    }
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
                        if self.follow_satellite.is_some() {
                            self.rotate_follow_offset_horizontal(-dx * 0.005);
                            self.rotate_follow_offset_vertical(-dy * 0.005);
                        } else {
                            self.program.camera.rotate_around_up(-dx * 0.005);
                            self.program.camera.rotate_vertically(-dy * 0.005);
                        }
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
                if self.follow_satellite.is_some() {
                    let dist = self.follow_offset.norm();
                    let new_dist = (dist - amount).max(10.0);
                    self.follow_offset = self.follow_offset.normalize() * new_dist;
                } else {
                    self.program.camera.dolly(amount);
                }
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

        // --- Header: status + sim date ---
        let header = column![
            text(&self.status_message).size(14),
            text(format!("Date: {}", meta.simulation_date_string())).size(13),
            text(format!(
                "Alt: {:.0} km  |  Speed: {:.0}x  |  Frame: {:?}",
                self.program.camera.eye.coords.norm() - EARTH_RADIUS_KM,
                self.program.time_scale,
                self.program.frame_mode,
            ))
            .size(12),
        ]
        .spacing(3);

        // Error message
        let error_display = if self.error_message.is_empty() {
            column![]
        } else {
            column![
                text(&self.error_message)
                    .size(13)
                    .color(iced::Color::from_rgb(1.0, 0.3, 0.3))
            ]
        };

        // --- Controls row ---
        let controls = column![
            row![
                button("⏯ Pause").on_press(Message::TogglePause).width(80),
                button("⏮ Reset").on_press(Message::ResetTime).width(80),
                button("🔄 Frame").on_press(Message::ToggleFrame).width(80),
            ]
            .spacing(4),
            row![
                button("◀ Slower")
                    .on_press(Message::DecreaseTimeScale)
                    .width(80),
                button("▶ Faster")
                    .on_press(Message::IncreaseTimeScale)
                    .width(80),
                button("1x").on_press(Message::ResetTimeScale).width(40),
            ]
            .spacing(4),
            row![
                button(if self.program.simulation.precession_enabled {
                    "Precession: ON"
                } else {
                    "Precession: OFF"
                })
                .on_press(Message::TogglePrecession),
                button(if self.follow_satellite.is_some() {
                    "Free Camera"
                } else {
                    "Follow..."
                })
                .on_press(Message::FollowSatellite(
                    if self.follow_satellite.is_some() {
                        None
                    } else {
                        Some((0, 0))
                    }
                )),
            ]
            .spacing(4),
        ]
        .spacing(4);

        // --- Mode tabs ---
        let mode_row = row![
            button("Builder")
                .on_press(Message::SwitchMode(PanelMode::Builder))
                .style(if self.panel_mode == PanelMode::Builder {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                }),
            button("Manager")
                .on_press(Message::SwitchMode(PanelMode::Manager))
                .style(if self.panel_mode == PanelMode::Manager {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                }),
            button("KPIs")
                .on_press(Message::SwitchMode(PanelMode::Kpi))
                .style(if self.panel_mode == PanelMode::Kpi {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                }),
        ]
        .spacing(4);

        // --- Mode content ---
        let mode_content: Element<'_, Message> = match self.panel_mode {
            PanelMode::Builder => self.builder_panel(),
            PanelMode::Manager => self.manager_panel(),
            PanelMode::Kpi => self.kpi_panel(),
        };

        let content = column![header, error_display, controls, mode_row, mode_content,]
            .spacing(8)
            .padding(10);

        scrollable(content).height(Fill).into()
    }

    fn builder_panel(&self) -> Element<'_, Message> {
        let builder_toolbar = row![
            button("Station").on_press(Message::ShowBuilderPane(BuilderPane::Station)),
            button("Orbit").on_press(Message::ShowBuilderPane(BuilderPane::Orbit)),
            button("Satellite").on_press(Message::ShowBuilderPane(BuilderPane::Satellite)),
            button("Rect Surface").on_press(Message::ShowBuilderPane(BuilderPane::RectSurface)),
        ]
        .spacing(4);

        let builder_form: Element<'_, Message> = match self.builder_pane {
            BuilderPane::Orbit => column![
                text("New Orbit").size(15),
                text("Altitude above Earth surface").size(11),
                text_input("Altitude (km)", &self.orbit_altitude_input)
                    .on_input(Message::OrbitAltitudeInput),
                text("Orbital inclination angle").size(11),
                text_input("Inclination (°)", &self.orbit_inclination_input)
                    .on_input(Message::OrbitInclinationInput),
                text("Right Ascension of Ascending Node").size(11),
                text_input("RAAN (°)", &self.orbit_raan_input).on_input(Message::OrbitRaanInput),
                text("Argument of Perigee").size(11),
                text_input("Arg. Perigee (°)", &self.orbit_arg_perigee_input)
                    .on_input(Message::OrbitArgPerigeeInput),
                button("Create Orbit").on_press(Message::CreateOrbit),
            ]
            .spacing(4)
            .into(),
            BuilderPane::Station => column![
                text("New Ground Station").size(15),
                text_input("Name", &self.station_name_input).on_input(Message::StationNameInput),
                text("Geographic latitude").size(11),
                text_input("Latitude (°, -90 to 90)", &self.station_lat_input)
                    .on_input(Message::StationLatInput),
                text("Geographic longitude").size(11),
                text_input("Longitude (°, -180 to 180)", &self.station_lon_input)
                    .on_input(Message::StationLonInput),
                button("Create Station").on_press(Message::CreateStation),
            ]
            .spacing(4)
            .into(),
            BuilderPane::Satellite => column![
                text("New Satellite").size(15),
                text_input("Satellite name", &self.satellite_name_input)
                    .on_input(Message::SatelliteNameInput),
                text(format!(
                    "Orbit index (0..{})",
                    self.program.simulation.orbits.len().saturating_sub(1)
                ))
                .size(11),
                text_input("Orbit index", &self.satellite_orbit_index_input)
                    .on_input(Message::SatelliteOrbitIndexInput),
                button("Create Satellite").on_press(Message::CreateOrbitSatellite),
            ]
            .spacing(4)
            .into(),
            BuilderPane::RectSurface => column![
                text("Rectangular Surface on Earth").size(15),
                text("Define a lat/lon bounding box").size(11),
                text_input("Min Latitude (°)", &self.rect_min_lat_input)
                    .on_input(Message::RectMinLatInput),
                text_input("Max Latitude (°)", &self.rect_max_lat_input)
                    .on_input(Message::RectMaxLatInput),
                text_input("Min Longitude (°)", &self.rect_min_lon_input)
                    .on_input(Message::RectMinLonInput),
                text_input("Max Longitude (°)", &self.rect_max_lon_input)
                    .on_input(Message::RectMaxLonInput),
                button("Create Surface").on_press(Message::CreateRectSurface),
            ]
            .spacing(4)
            .into(),
            BuilderPane::None => column![text("Select a resource type above").size(13)].into(),
        };

        column![builder_toolbar, builder_form].spacing(6).into()
    }

    fn manager_panel(&self) -> Element<'_, Message> {
        let meta = &self.program.simulation;
        let mut panel = column![text("Resource Manager").size(16)].spacing(6);

        // If a resource was clicked, show only that resource
        let show_all = self.manager_focus.is_none();

        if show_all {
            panel =
                panel.push(button("Show All").on_press(Message::SwitchMode(PanelMode::Manager)));
        } else {
            panel = panel.push(
                button("← Show All Resources").on_press(Message::SwitchMode(PanelMode::Manager)),
            );
        }

        // Orbits section
        let show_orbits =
            show_all || matches!(&self.manager_focus, Some(SelectedObject::Satellite(_)));
        if show_orbits {
            panel = panel.push(text("━━ Orbits ━━").size(13));
            for (i, orbit) in meta.orbits.iter().enumerate() {
                let orbit_header = row![
                    text(format!(
                        "#{} alt={:.0}km inc={:.1}° sats={}",
                        i,
                        orbit.semi_major_axis - EARTH_RADIUS_KM,
                        orbit.inclination_deg,
                        orbit.satellites.len()
                    ))
                    .size(12),
                    button("🗑").on_press(Message::DeleteOrbit(i)),
                ]
                .spacing(4);

                let orbit_controls = row![
                    button(if orbit.show_orbit {
                        "Orbit: ✓"
                    } else {
                        "Orbit: ✗"
                    })
                    .on_press(Message::ToggleOrbitVisible(i)),
                    button(if orbit.show_fov {
                        "FOV: ✓"
                    } else {
                        "FOV: ✗"
                    })
                    .on_press(Message::ToggleOrbitFov(i)),
                    button(if orbit.fill_fov {
                        "Fill: ✓"
                    } else {
                        "Fill: ✗"
                    })
                    .on_press(Message::ToggleOrbitFovFill(i)),
                ]
                .spacing(2);

                let fov_input = row![
                    text("FOV half-angle (°)").size(11),
                    text_input("deg", &format!("{:.1}", orbit.fov_half_angle_deg),)
                        .on_input(move |v| Message::OrbitFovAngleInput(i, v))
                        .width(60),
                ]
                .spacing(4);

                let orbit_params = row![
                    text("Inc (°)").size(11),
                    text_input("deg", &format!("{:.1}", orbit.inclination_deg))
                        .on_input(move |v| Message::OrbitInclinationEdit(i, v))
                        .width(55),
                    text("RAAN (°)").size(11),
                    text_input("deg", &format!("{:.1}", orbit.raan_deg))
                        .on_input(move |v| Message::OrbitRaanEdit(i, v))
                        .width(55),
                ]
                .spacing(4);

                panel = panel.push(
                    column![orbit_header, orbit_controls, fov_input, orbit_params].spacing(2),
                );

                for (si, sat) in orbit.satellites.iter().enumerate() {
                    panel = panel.push(
                        row![
                            text(format!("  └ {}", sat.name)).size(11),
                            button("🗑").on_press(Message::DeleteSatellite(i, si)),
                            button("📷").on_press(Message::FollowSatellite(Some((i, si)))),
                        ]
                        .spacing(4),
                    );
                }
            }
        }

        // Stations section
        let show_stations =
            show_all || matches!(&self.manager_focus, Some(SelectedObject::GroundStation(_)));
        if show_stations {
            panel = panel.push(text("━━ Stations ━━").size(13));
            for (i, station) in meta.ground_stations.iter().enumerate() {
                let station_row = row![
                    text(format!(
                        "{} ({:.1}°, {:.1}°)",
                        station.name, station.latitude_deg, station.longitude_deg
                    ))
                    .size(12),
                    button("🗑").on_press(Message::DeleteStation(i)),
                ]
                .spacing(4);

                let station_controls = row![
                    button(if station.show_cone {
                        "Cone: ✓"
                    } else {
                        "Cone: ✗"
                    })
                    .on_press(Message::ToggleStationCone(i)),
                    text("Min elev (°)").size(11),
                    text_input("deg", &format!("{:.1}", station.min_elevation_deg),)
                        .on_input(move |v| Message::StationMinElevationInput(i, v))
                        .width(60),
                ]
                .spacing(4);

                panel = panel.push(column![station_row, station_controls].spacing(2));
            }
        }

        panel.into()
    }

    fn kpi_panel(&self) -> Element<'_, Message> {
        let mut panel = column![text("KPI Dashboard").size(16)].spacing(6);

        panel = panel.push(text("Station-Satellite Distance Plot").size(14));
        panel = panel.push(
            row![
                text("Station #").size(11),
                text_input("0", &self.kpi_station_index)
                    .on_input(Message::KpiStationIndexInput)
                    .width(40),
                text("Orbit #").size(11),
                text_input("0", &self.kpi_orbit_index)
                    .on_input(Message::KpiOrbitIndexInput)
                    .width(40),
                text("Sat #").size(11),
                text_input("0", &self.kpi_sat_index)
                    .on_input(Message::KpiSatIndexInput)
                    .width(40),
            ]
            .spacing(4),
        );

        // Text-based "plot" of recent distance values
        if self.kpi_distance_history.is_empty() {
            panel = panel.push(text("No data yet. Configure indices and wait...").size(11));
        } else {
            let last = self.kpi_distance_history.last().unwrap();
            panel = panel.push(
                text(format!(
                    "Current distance: {:.1} km (t={:.1}s)",
                    last.1, last.0
                ))
                .size(13),
            );

            // Simple ASCII sparkline of last 60 samples
            let samples: Vec<f32> = self
                .kpi_distance_history
                .iter()
                .rev()
                .take(60)
                .rev()
                .map(|(_, d)| *d)
                .collect();

            if samples.len() >= 2 {
                let min_d = samples.iter().cloned().fold(f32::INFINITY, f32::min);
                let max_d = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let range = (max_d - min_d).max(1.0);

                let bars = "▁▂▃▄▅▆▇█";
                let bar_chars: Vec<char> = bars.chars().collect();
                let sparkline: String = samples
                    .iter()
                    .map(|d| {
                        let normalized = ((d - min_d) / range * 7.0).round() as usize;
                        bar_chars[normalized.min(7)]
                    })
                    .collect();

                panel = panel
                    .push(text(format!("Min: {:.0} km  Max: {:.0} km", min_d, max_d)).size(11));
                panel = panel.push(text(sparkline).size(16));
            }
        }

        // Summary stats
        let meta = &self.program.simulation;
        panel = panel.push(text("━━ Summary ━━").size(13));
        panel = panel.push(
            text(format!(
                "Orbits: {}  Stations: {}  Satellites: {}",
                meta.orbits.len(),
                meta.ground_stations.len(),
                meta.satellite_count(),
            ))
            .size(12),
        );

        panel.into()
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
            status_message: "Ready".to_string(),
            error_message: String::new(),
            panel_mode: PanelMode::Builder,
            builder_pane: BuilderPane::None,
            orbit_altitude_input: "500".to_string(),
            orbit_inclination_input: "20".to_string(),
            orbit_raan_input: "30".to_string(),
            orbit_arg_perigee_input: "0".to_string(),
            station_name_input: String::new(),
            station_lat_input: "0".to_string(),
            station_lon_input: "0".to_string(),
            satellite_name_input: String::new(),
            satellite_orbit_index_input: "0".to_string(),
            rect_min_lat_input: "-10".to_string(),
            rect_max_lat_input: "10".to_string(),
            rect_min_lon_input: "-10".to_string(),
            rect_max_lon_input: "10".to_string(),
            cursor_position: None,
            drag_start: None,
            right_button_down: false,
            selected_object: SelectedObject::None,
            selected_hit_distance: None,
            viewport_size: (200.0, 200.0),
            follow_satellite: None,
            follow_offset: nalgebra::Vector3::new(0.0, 0.0, 200.0),
            kpi_station_index: "0".to_string(),
            kpi_orbit_index: "0".to_string(),
            kpi_sat_index: "0".to_string(),
            kpi_distance_history: Vec::new(),
            manager_focus: None,
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
