use chrono::Utc;
use env_logger::Env;

use gui::{
    gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode},
    model::{
        ground_station::GroundStation,
        orbit::Orbit,
        satellite::Satellite,
        system::{EARTH_RADIUS_KM, System},
    },
    ui::{
        screens::main_screen::{
            self, BuilderForm, SidebarTab, error_banner, kpi_panel, orbit_builder_panel,
            orbit_manager_item, rect_surface_builder_panel, satellite_builder_panel,
            satellite_manager_item, sim_controls_panel, station_builder_panel,
            station_manager_item, status_bar, tab_bar,
        },
        theme::{colors, spacing, typography},
    },
};
use iced::{
    Element,
    Length::Fill,
    Theme,
    keyboard::{self, Key},
    time::{self, milliseconds},
    widget::{column, container, pane_grid, shader, text},
};
use log::{debug, info};

use gui::simulation::{FrameMode, SelectedObject, Simulation};

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
    SwitchMode(SidebarTab),
    ShowBuilderPane(BuilderForm),
    // Builder: Orbit
    CreateOrbit,
    OrbitAltitudeInput(String),
    OrbitInclinationInput(String),
    OrbitRaanInput(String),
    OrbitArgPerigeeInput(String),
    OrbitNameInput(String),
    ToggleOrbitAdvanced,
    // Builder: Station
    CreateStation,
    StationNameInput(String),
    StationLatInput(String),
    StationLonInput(String),
    // Builder: Satellite
    CreateOrbitSatellite,
    SatelliteNameInput(String),
    SatelliteOrbitSelected(usize),
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
    program: Simulation,
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

    panel_mode: SidebarTab,
    builder_pane: BuilderForm,

    // Builder: orbit
    orbit_altitude_input: String,
    orbit_inclination_input: String,
    orbit_raan_input: String,
    orbit_arg_perigee_input: String,
    orbit_name_input: String,
    orbit_advanced_expanded: bool,
    // Builder: station
    station_name_input: String,
    station_lat_input: String,
    station_lon_input: String,
    // Builder: satellite
    satellite_name_input: String,
    satellite_orbit_selection: Option<usize>,
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
                    if let Some(orbit) = self.program.system.orbits.get(orbit_idx) {
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
                        .system
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
                    FrameMode::Eci => {
                        self.program.ecef_reference_earth_angle = current_phase;
                        self.program.frame_mode = FrameMode::Ecef;
                    }
                    FrameMode::Ecef => {
                        self.program.frame_mode = FrameMode::Eci;
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
                self.program.system.precession_enabled = !self.program.system.precession_enabled;
                self.status_message = format!(
                    "Precession: {}",
                    if self.program.system.precession_enabled {
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
            Message::ShowBuilderPane(builder_form) => {
                self.builder_pane = builder_form;
                self.error_message.clear();
            }
            // Builder inputs
            Message::OrbitAltitudeInput(value) => self.orbit_altitude_input = value,
            Message::OrbitInclinationInput(value) => self.orbit_inclination_input = value,
            Message::OrbitRaanInput(value) => self.orbit_raan_input = value,
            Message::OrbitArgPerigeeInput(value) => self.orbit_arg_perigee_input = value,
            Message::OrbitNameInput(value) => self.orbit_name_input = value,
            Message::ToggleOrbitAdvanced => {
                self.orbit_advanced_expanded = !self.orbit_advanced_expanded
            }
            Message::StationNameInput(value) => self.station_name_input = value,
            Message::StationLatInput(value) => self.station_lat_input = value,
            Message::StationLonInput(value) => self.station_lon_input = value,
            Message::SatelliteNameInput(value) => self.satellite_name_input = value,
            Message::SatelliteOrbitSelected(value) => self.satellite_orbit_selection = Some(value),
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
                let orbit_name = if self.orbit_name_input.trim().is_empty() {
                    format!("Orbit {}", self.program.system.orbits.len() + 1)
                } else {
                    self.orbit_name_input.trim().to_string()
                };

                self.modify_model(|model| {
                    model.orbits.push(
                        Orbit::builder(semi_major_axis, period_seconds)
                            .name(orbit_name.clone())
                            .inclination(inclination)
                            .raan(raan)
                            .arg_perigee(arg_perigee)
                            .show_orbit(true)
                            .add_satellite(Satellite::builder("Sat-1").phase_offset(0.0).build())
                            .build(),
                    );
                });
                self.satellite_orbit_selection = Some(self.program.system.orbits.len() - 1);
                self.error_message.clear();
                self.status_message = format!(
                    "Created orbit '{}' at {:.0} km, total {}",
                    orbit_name,
                    altitude,
                    self.program.system.orbits.len(),
                );
            }
            Message::CreateStation => {
                let name = if self.station_name_input.trim().is_empty() {
                    format!("Station {}", self.program.system.ground_stations.len() + 1)
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
                if self.program.system.orbits.is_empty() {
                    self.error_message = "No orbits available. Create an orbit first.".to_string();
                    return;
                }

                let orbit_index = self
                    .satellite_orbit_selection
                    .filter(|idx| *idx < self.program.system.orbits.len());

                if let Some(idx) = orbit_index {
                    let sat_name = if self.satellite_name_input.trim().is_empty() {
                        format!(
                            "Sat-{}",
                            self.program.system.orbits[idx].satellites.len() + 1
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
                        self.program.system.orbits.len().saturating_sub(1)
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
                    .system
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

                let orbit_count = self.program.system.orbits.len();
                self.satellite_orbit_selection = match self.satellite_orbit_selection {
                    None => {
                        if orbit_count > 0 {
                            Some(0)
                        } else {
                            None
                        }
                    }
                    Some(_) if orbit_count == 0 => None,
                    Some(selected) if selected > index => Some(selected - 1),
                    Some(selected) if selected >= orbit_count => Some(orbit_count - 1),
                    Some(selected) => Some(selected),
                };
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
                if let Some(orbit) = self.program.system.orbits.get_mut(idx) {
                    orbit.show_orbit = !orbit.show_orbit;
                }
            }
            Message::ToggleOrbitFov(idx) => {
                if let Some(orbit) = self.program.system.orbits.get_mut(idx) {
                    orbit.show_fov = !orbit.show_fov;
                }
            }
            Message::ToggleOrbitFovFill(idx) => {
                if let Some(orbit) = self.program.system.orbits.get_mut(idx) {
                    orbit.fill_fov = !orbit.fill_fov;
                }
            }
            Message::OrbitFovAngleInput(idx, value) => {
                if let Ok(angle) = value.parse::<f32>() {
                    if let Some(orbit) = self.program.system.orbits.get_mut(idx) {
                        orbit.fov_half_angle_deg = angle.clamp(0.1, 89.0);
                    }
                }
            }
            Message::OrbitInclinationEdit(idx, value) => {
                if let Ok(inc) = value.parse::<f32>() {
                    if let Some(orbit) = self.program.system.orbits.get_mut(idx) {
                        orbit.inclination_deg = inc;
                    }
                }
            }
            Message::OrbitRaanEdit(idx, value) => {
                if let Ok(raan) = value.parse::<f32>() {
                    if let Some(orbit) = self.program.system.orbits.get_mut(idx) {
                        orbit.raan_deg = raan;
                    }
                }
            }
            // Manager: tune station
            Message::StationMinElevationInput(idx, value) => {
                if let Ok(elev) = value.parse::<f32>() {
                    if let Some(station) = self.program.system.ground_stations.get_mut(idx) {
                        station.min_elevation_deg = elev.clamp(0.0, 90.0);
                    }
                }
            }
            Message::ToggleStationCone(idx) => {
                if let Some(station) = self.program.system.ground_stations.get_mut(idx) {
                    station.show_cone = !station.show_cone;
                }
            }
            // Camera follow
            Message::FollowSatellite(target) => {
                match target {
                    Some((o, s)) => {
                        // Initialize offset along radial direction from satellite
                        if let Some(orbit) = self.program.system.orbits.get(o) {
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

    fn modify_model(&mut self, mut f: impl FnMut(&mut System)) {
        let mut system = self.program.system.clone();
        f(&mut system);
        self.program.system = system;
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
                self.panel_mode = SidebarTab::Manager;
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

    fn build_status_bar(&self) -> Element<'_, Message> {
        let meta = &self.program.system;
        let status_left = &self.status_message;
        let status_center = format!("Date: {}", meta.simulation_date_string());
        let status_right = format!(
            "Alt: {:.0} km  |  Speed: {:.0}x  |  Frame: {:?}",
            self.program.camera.eye.coords.norm() - EARTH_RADIUS_KM,
            self.program.time_scale,
            self.program.frame_mode,
        );
        status_bar(status_left, &status_center, &status_right)
    }

    fn build_sidebar(&self) -> Element<'_, Message> {
        // --- Sim controls ---
        let sim_controls = sim_controls_panel(
            self.program.paused,
            self.program.time_scale,
            self.program.system.precession_enabled,
            self.follow_satellite.is_some(),
            Message::TogglePause,
            Message::ResetTime,
            Message::ToggleFrame,
            Message::DecreaseTimeScale,
            Message::IncreaseTimeScale,
            Message::ResetTimeScale,
            Message::TogglePrecession,
            Message::FollowSatellite(if self.follow_satellite.is_some() {
                None
            } else {
                Some((0, 0))
            }),
        );

        // --- Tab bar ---
        let tabs = tab_bar(self.panel_mode, Message::SwitchMode);

        // --- Error ---
        let error = error_banner::<Message>(&self.error_message);

        // --- Mode content ---
        let mode_content: Element<'_, Message> = match self.panel_mode {
            SidebarTab::Builder => self.builder_panel(),
            SidebarTab::Manager => self.manager_panel(),
            SidebarTab::Kpi => self.kpi_view(),
        };

        // --- Compose sidebar ---
        let mut col = iced::widget::Column::new().spacing(spacing::SECTION_GAP);
        col = col.push(sim_controls);
        col = col.push(tabs);
        col = col.push(error);
        col = col.push(mode_content);

        let scrollable_content = iced::widget::scrollable(
            container(col)
                .padding(spacing::SIDEBAR_PADDING)
                .width(iced::Length::Fill),
        )
        .height(iced::Length::Fill);

        container(scrollable_content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(colors::BG_ELEVATED)),
                border: iced::Border {
                    color: colors::BORDER,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            })
            .into()
    }

    fn builder_panel(&self) -> Element<'_, Message> {
        let toolbar = main_screen::builder_toolbar(self.builder_pane, Message::ShowBuilderPane);

        let form: Element<'_, Message> = match self.builder_pane {
            BuilderForm::Orbit => orbit_builder_panel(
                &self.orbit_name_input,
                &self.orbit_altitude_input,
                &self.orbit_inclination_input,
                &self.orbit_raan_input,
                &self.orbit_arg_perigee_input,
                Message::OrbitNameInput,
                Message::OrbitAltitudeInput,
                Message::OrbitInclinationInput,
                Message::OrbitRaanInput,
                Message::OrbitArgPerigeeInput,
                Message::CreateOrbit,
                self.orbit_advanced_expanded,
                Message::ToggleOrbitAdvanced,
            ),
            BuilderForm::Station => station_builder_panel(
                &self.station_name_input,
                &self.station_lat_input,
                &self.station_lon_input,
                Message::StationNameInput,
                Message::StationLatInput,
                Message::StationLonInput,
                Message::CreateStation,
            ),
            BuilderForm::Satellite => satellite_builder_panel(
                &self.satellite_name_input,
                self.satellite_orbit_selection,
                self.program
                    .system
                    .orbits
                    .iter()
                    .enumerate()
                    .map(|(index, orbit)| {
                        let label = if orbit.name.trim().is_empty() {
                            format!("Orbit {}", index + 1)
                        } else {
                            orbit.name.clone()
                        };
                        (index, label)
                    })
                    .collect(),
                Message::SatelliteNameInput,
                Message::SatelliteOrbitSelected,
                Message::CreateOrbitSatellite,
            ),
            BuilderForm::RectSurface => rect_surface_builder_panel(
                &self.rect_min_lat_input,
                &self.rect_max_lat_input,
                &self.rect_min_lon_input,
                &self.rect_max_lon_input,
                Message::RectMinLatInput,
                Message::RectMaxLatInput,
                Message::RectMinLonInput,
                Message::RectMaxLonInput,
                Message::CreateRectSurface,
            ),
            BuilderForm::None => column![
                text("Select a resource type above")
                    .size(typography::SIZE_SM)
                    .color(colors::TEXT_SECONDARY)
            ]
            .into(),
        };

        column![toolbar, form].spacing(spacing::SECTION_GAP).into()
    }

    fn manager_panel(&self) -> Element<'_, Message> {
        let meta = &self.program.system;
        let mut items: Vec<Element<'_, Message>> = Vec::new();

        // Orbits
        for (i, orbit) in meta.orbits.iter().enumerate() {
            items.push(orbit_manager_item(
                i,
                (orbit.semi_major_axis - EARTH_RADIUS_KM) as f32,
                orbit.inclination_deg as f32,
                orbit.satellites.len(),
                orbit.show_orbit,
                orbit.show_fov,
                orbit.fill_fov,
                orbit.fov_half_angle_deg as f32,
                Message::DeleteOrbit(i),
                Message::ToggleOrbitVisible(i),
                Message::ToggleOrbitFov(i),
                Message::ToggleOrbitFovFill(i),
                move |v| Message::OrbitFovAngleInput(i, v),
                move |v| Message::OrbitInclinationEdit(i, v),
                move |v| Message::OrbitRaanEdit(i, v),
                orbit.raan_deg as f32,
            ));

            // Satellite sub-items
            for (si, sat) in orbit.satellites.iter().enumerate() {
                items.push(satellite_manager_item(
                    &sat.name,
                    Message::DeleteSatellite(i, si),
                    Message::FollowSatellite(Some((i, si))),
                ));
            }
        }

        // Stations
        for (i, station) in meta.ground_stations.iter().enumerate() {
            items.push(station_manager_item(
                &station.name,
                station.latitude_deg as f32,
                station.longitude_deg as f32,
                station.show_cone,
                station.min_elevation_deg as f32,
                Message::DeleteStation(i),
                Message::ToggleStationCone(i),
                move |v| Message::StationMinElevationInput(i, v),
            ));
        }

        if items.is_empty() {
            column![
                text("No resources. Use the Builder to create orbits and stations.")
                    .size(typography::SIZE_SM)
                    .color(colors::TEXT_SECONDARY)
            ]
            .into()
        } else {
            let mut col = column![].spacing(spacing::CONTROL_GAP);
            for item in items {
                col = col.push(item);
            }
            col.into()
        }
    }

    fn kpi_view(&self) -> Element<'_, Message> {
        let meta = &self.program.system;

        // Compute current distance and sparkline
        let current_distance = if let (Ok(si), Ok(oi), Ok(sati)) = (
            self.kpi_station_index.parse::<usize>(),
            self.kpi_orbit_index.parse::<usize>(),
            self.kpi_sat_index.parse::<usize>(),
        ) {
            let elapsed = self.program.elapsed_time();
            self.program
                .system
                .station_satellite_distance(si, oi, sati, elapsed)
        } else {
            None
        };

        // Sparkline
        let (sparkline, min_dist, max_dist) = if self.kpi_distance_history.len() >= 2 {
            let samples: Vec<f32> = self
                .kpi_distance_history
                .iter()
                .rev()
                .take(60)
                .rev()
                .map(|(_, d)| *d)
                .collect();
            let min_d = samples.iter().cloned().fold(f32::INFINITY, f32::min);
            let max_d = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let range = (max_d - min_d).max(1.0);
            let bars = "▁▂▃▄▅▆▇█";
            let bar_chars: Vec<char> = bars.chars().collect();
            let spark: String = samples
                .iter()
                .map(|d| {
                    let normalized = ((d - min_d) / range * 7.0).round() as usize;
                    bar_chars[normalized.min(7)]
                })
                .collect();
            (Some(spark), Some(min_d), Some(max_d))
        } else {
            (None, None, None)
        };

        kpi_panel(
            &self.kpi_station_index,
            &self.kpi_orbit_index,
            &self.kpi_sat_index,
            current_distance,
            sparkline.as_deref(),
            min_dist,
            max_dist,
            meta.orbits.len(),
            meta.ground_stations.len(),
            meta.satellite_count(),
            Message::KpiStationIndexInput,
            Message::KpiOrbitIndexInput,
            Message::KpiSatIndexInput,
        )
    }

    fn view(&self) -> Element<'_, Message> {
        let pane_grid = pane_grid::PaneGrid::new(&self.panes, |_, pane_state, _| {
            let content: Element<'_, Message> = match pane_state.id {
                0 => shader(&self.program).width(Fill).height(Fill).into(),
                1 => column![self.build_status_bar(), self.build_sidebar()]
                    .spacing(0)
                    .height(Fill)
                    .into(),
                _ => text("Unknown pane").into(),
            };

            pane_grid::Content::new(content)
        })
        .width(Fill)
        .height(Fill)
        .spacing(spacing::XXXS)
        .on_click(Message::PaneClicked)
        .on_drag(Message::PaneDragged)
        .on_resize(4, Message::PaneResized);

        container(pane_grid).padding(spacing::XXXS).into()
    }
}

impl Default for Textured {
    fn default() -> Self {
        let earth_radius = EARTH_RADIUS_KM;
        let orbit1_a = earth_radius + 500.0;
        let orbit2_a = earth_radius + 800.0;

        let mut system = System::builder()
            .add_orbit(
                Orbit::builder(orbit1_a, Orbit::circular_period_seconds(orbit1_a))
                    .name("Orbit 1")
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
                    .name("Orbit 2")
                    .inclination(45.0)
                    .raan(80.0)
                    .arg_perigee(30.0)
                    .show_orbit(true)
                    .add_satellite(Satellite::builder("Sat-3").phase_offset(2.0).build())
                    .build(),
            )
            .add_ground_station(GroundStation::new("Paris Station", 48.8566, 2.3522))
            .build(Utc::now());
        system.simulation_speed = 120;

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
            program: Simulation {
                system,
                camera,
                satellite_mode: SatelliteRenderMode::Dot,
                frame_mode: FrameMode::Eci,
                ecef_reference_earth_angle: 0.0,
                paused: false,
                time_scale: 120.0,
                pick_radius_scale: 2.0,
                show_clouds: true,
            },
            panes,
            focus: Some(root_pane),
            status_message: "Ready".to_string(),
            error_message: String::new(),
            panel_mode: SidebarTab::Builder,
            builder_pane: BuilderForm::Orbit,
            orbit_altitude_input: "500".to_string(),
            orbit_inclination_input: "20".to_string(),
            orbit_raan_input: "30".to_string(),
            orbit_arg_perigee_input: "0".to_string(),
            orbit_name_input: String::new(),
            orbit_advanced_expanded: false,
            station_name_input: String::new(),
            station_lat_input: "0".to_string(),
            station_lon_input: "0".to_string(),
            satellite_name_input: String::new(),
            satellite_orbit_selection: Some(0),
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
    // Enable debug logs only for this example crate ("simulation").
    // Override with RUST_LOG when needed, e.g. RUST_LOG=info or RUST_LOG=simulation=debug
    env_logger::Builder::from_env(Env::default().default_filter_or("simulation=debug")).init();

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
