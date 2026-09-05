/// Sun-synchronous orbit demonstration example.
///
/// Explains and validates how sun-synchronous orbits (SSO) work:
/// - Earth's J2 oblateness makes retrograde orbits precess eastward
/// - At inclination ≈ 96–100° (altitude dependent) the nodal precession
///   exactly matches the Sun's apparent motion (~0.9856°/day)
/// - The orbit plane then keeps a constant angle to the Sun, so the
///   satellite crosses the equator at the same local solar time every day
///
/// Interactive panel:
/// - Orbit: altitude, inclination, local time of ascending node (LTAN, with
///   dawn-dusk / mid-morning / noon presets), sun-synchronous lock (auto-derives
///   inclination from altitude), satellite constellation size
/// - Season: restart the simulation at an equinox or solstice to see how the
///   terminator tilt changes while the orbit plane stays locked to the Sun
/// - Ground station: pick a station and watch the contact link appear when the
///   satellite rises above its horizon (polar stations see every pass)
/// - View: ECI/ECEF frame, camera reset, satellite FOV footprint, Keplerian
///   element visualization, and a longitude grid with local-time labels
/// - Time: pause, simulation speed
/// - Readouts: UTC clock, ground-station local solar time & sun elevation,
///   current LTAN/LTDN, nodal drift rate vs the required value, and the
///   Sun-to-orbit-plane angle (constant = sun-synchronous)
use chrono::{TimeZone, Utc};
use gui::astro::{Astral, constants};
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::FrameMode;
use gui::model::orbit::Orbit;
use gui::model::satellite::Satellite;
use gui::model::system::System;
use gui::simulation::Simulation as ProgramSimulation;
use gui::viewer::{CameraControl, subscription};
use iced::mouse;
use iced::widget::{
    button, column, container, radio, row, scrollable, shader, slider, text, toggler, tooltip,
};
use iced::{Background, Border, Color, Element, Length};
use nalgebra::{Point3, Rotation3, Vector3};

const LINK_LINE_COLOR: [f32; 3] = [0.3, 0.95, 0.4];

/// Ground stations available for the contact demo: (name, lat °, lon °).
const STATIONS: [(&str, f32, f32); 5] = [
    ("Toulouse", 43.6, 1.43),
    ("Kourou", 5.2, -52.8),
    ("Kiruna", 67.9, 21.1),
    ("Svalbard", 78.2, 15.4),
    ("Singapore", 1.35, 103.8),
];

/// Initial camera pose (restored by the "Reset view" button). Close enough
/// that the satellite model is visible riding the orbit track.
const CAMERA_EYE: Point3<f32> = Point3::new(-9_500.0, -13_000.0, 8_500.0);
const CAMERA_FOVY: f32 = 45.0;

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Event(iced::event::Event),
    Shot(gui::screenshot::ShotMessage),
    // Orbit controls
    AltitudeChanged(f32),
    InclinationChanged(f32),
    LtanChanged(f32),
    ToggleSsoLock(bool),
    ToggleJ2(bool),
    SatellitesChanged(f32),
    // Season controls
    SeasonChanged(u32, u32),
    // Ground-station controls
    StationChanged(usize),
    // View controls
    ToggleFrameMode,
    ResetCamera,
    ToggleFov(bool),
    ToggleFovFill(bool),
    FovAngleChanged(f32),
    ToggleOrbitPath(bool),
    ToggleKeplerian(bool),
    ToggleLongitudes(bool),
    // Time controls
    TogglePause(bool),
    SpeedChanged(f32),
}

/// One line of the "Live info" panel: a section header or a term/value row
/// whose term carries a pedagogical hover tooltip.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum InfoLine {
    Header(String),
    Row {
        term: String,
        value: String,
        tip: Option<String>,
    },
}

struct SunSynchronousSimulation {
    program: ProgramSimulation,
    // Orbit parameters
    altitude_km: f32,
    inclination_deg: f32,
    ltan_hours: f32,
    /// When on, inclination (or altitude) is auto-derived from the other so the
    /// orbit stays exactly sun-synchronous.
    sso_lock: bool,
    /// Number of evenly-spaced satellites on the orbit (constellation demo).
    sat_count: u32,
    /// Whether to render the on-scene Keplerian-element visualization.
    show_keplerian: bool,
    /// Whether to render the longitude grid with local-time labels.
    show_longitudes: bool,
    /// Index into `STATIONS` / `system.ground_stations` for the readouts.
    station_index: usize,
    /// Speed slider exponent: time_scale = 2^speed_exp.
    speed_exp: f32,
    /// Live panel readouts, updated on ticks (rendering text computed inside
    /// `view` with rapidly changing content blanks out in iced 0.14).
    info_lines: Vec<InfoLine>,
    tick_count: u32,
    // Camera interaction
    control: CameraControl,
    shot: gui::screenshot::AutoShot,
}

/// Normalize an angle in degrees to [-180, 180).
fn wrap_deg(deg: f64) -> f64 {
    let a = (deg + 180.0).rem_euclid(360.0) - 180.0;
    if a == 180.0 { -180.0 } else { a }
}

/// Normalize hours to [0, 24).
fn wrap_hours(h: f64) -> f64 {
    h.rem_euclid(24.0)
}

/// Format fractional hours as `HH:MM`.
fn format_hour(hours: f64) -> String {
    let h = wrap_hours(hours);
    let hh = h.floor() as u32;
    let mm = ((h - h.floor()) * 60.0).round() as u32;
    let (hh, mm) = if mm == 60 { (hh + 1, 0) } else { (hh, mm) };
    format!("{hh:02}:{mm:02}")
}

impl SunSynchronousSimulation {
    fn new() -> Self {
        // Summer solstice, mid-morning: tilted terminator, good SSO lighting.
        let sim_time = Utc.with_ymd_and_hms(2025, 6, 21, 10, 0, 0).unwrap();

        let altitude_km = 700.0_f32;
        let inclination_deg =
            Astral::sun_synchronous_inclination(altitude_km as f64, 0.0).unwrap() as f32;
        let ltan_hours = 6.0_f32; // dawn-dusk orbit: plane contains the Sun

        let sma = gui::model::system::EARTH_RADIUS_KM + altitude_km;
        let period = Orbit::circular_period_seconds(sma);

        let orbit = Orbit::builder(sma, period)
            .name("SSO Sat")
            .inclination(inclination_deg)
            .raan(Self::raan_for_ltan(&sim_time, ltan_hours))
            .show_orbit(true)
            .with_j2(true)
            .build();

        let stations: Vec<gui::model::ground_station::GroundStation> = STATIONS
            .iter()
            .map(|(name, lat, lon)| {
                gui::model::ground_station::GroundStation::new(*name, *lat, *lon)
            })
            .collect();

        let mut core_sim = System::builder().add_orbit(orbit).build(sim_time);
        core_sim.ground_stations = stations;
        // Keep the default scene readable: the green contact geometry is
        // available through the panel toggles instead.
        for station in &mut core_sim.ground_stations {
            station.show_cone = false;
        }
        core_sim.orbits[0].show_fov = false;
        // Real satellites are sub-pixel at this scale; exaggerate so the model
        // is visible riding the orbit track.
        core_sim.satellite_scale_factor = 6.0 * gui::model::system::System::SATELLITE_SCALE_FACTOR;

        core_sim
            .shapes
            .add_eci_frame(gui::model::system::EARTH_RADIUS_KM * 2.5);
        for (name, lat, lon) in STATIONS {
            core_sim.shapes.add_surface_point(lat, lon, name);
        }

        let mut camera = Camera::new(CAMERA_EYE, [0.0, 0.0, 0.0].into(), 1600.0, 900.0);
        camera.fovy = CAMERA_FOVY;

        let program = ProgramSimulation {
            system: core_sim,
            camera,
            satellite_mode: SatelliteRenderMode::Model,
            frame_mode: FrameMode::Eci,
            ecef_reference_earth_angle: 0.0,
            paused: false,
            time_scale: 1.0,
            pick_radius_scale: 1.0,
            show_clouds: false,
            show_atmosphere: true,
            show_night_lights: true,
            show_bloom: false,
        };

        let mut sim = Self {
            program,
            altitude_km,
            inclination_deg,
            ltan_hours,
            sso_lock: true,
            sat_count: 1,
            station_index: 0,
            show_keplerian: true,
            show_longitudes: true,
            speed_exp: 8.0, // 2^8 = 256x: one day passes in ~5.6 min
            info_lines: Vec::new(),
            tick_count: 0,
            control: CameraControl::default(),
            shot: gui::screenshot::AutoShot::from_env(),
        };
        sim.rebuild_satellites();
        sim.program.set_time_scale(2.0_f32.powf(sim.speed_exp));
        sim.refresh_dynamic_lines();
        sim.refresh_info();
        sim
    }

    /// RAAN (degrees) that puts the ascending node at local solar time
    /// `ltan_hours` for the given UTC instant.
    ///
    /// Local solar time = UTC hour + longitude/15, and node longitude
    /// (east, degrees) = RAAN - GMST, hence RAAN = node_lon + GMST.
    fn raan_for_ltan(sim_time: &chrono::DateTime<Utc>, ltan_hours: f32) -> f32 {
        let (day, hour) = Astral::datetime_to_day_hour(sim_time);
        let gmst_deg = Astral::earth_rotation_angle(day, hour).to_degrees();
        let node_lon = wrap_deg((ltan_hours as f64 - hour) * 15.0);
        (node_lon + gmst_deg).rem_euclid(360.0) as f32
    }

    /// Apply the panel's orbit parameters to the rendered orbit.
    fn apply_orbit(&mut self) {
        if let Some(orbit) = self.program.system.orbits.first_mut() {
            orbit.semi_major_axis = gui::model::system::EARTH_RADIUS_KM + self.altitude_km;
            orbit.period_seconds = Orbit::circular_period_seconds(orbit.semi_major_axis);
            orbit.inclination_deg = self.inclination_deg;
            orbit.raan_deg =
                Self::raan_for_ltan(&self.program.system.simulation_time, self.ltan_hours);
        }
        self.refresh_dynamic_lines();
    }

    /// Rebuild the satellite constellation with `sat_count` evenly spaced
    /// phases so ground coverage is uniform.
    fn rebuild_satellites(&mut self) {
        if let Some(orbit) = self.program.system.orbits.first_mut() {
            let n = self.sat_count.max(1);
            orbit.satellites = (0..n)
                .map(|i| {
                    Satellite::builder(format!("SSO-{i}"))
                        .phase_offset(i as f32 * std::f32::consts::TAU / n as f32)
                        .build()
                })
                .collect();
        }
    }

    /// Restart the simulation clock at an equinox/solstice instant, keeping
    /// the orbit and LTAN so the terminator-tilt change becomes visible.
    fn set_season(&mut self, month: u32, day: u32) {
        let sim_time = Utc.with_ymd_and_hms(2025, month, day, 10, 0, 0).unwrap();
        let system = &mut self.program.system;
        system.simulation_time = sim_time;
        system.start_time = sim_time;
        system.last_tick_time = Utc::now();
        system.accumulator = chrono::TimeDelta::zero();
        self.apply_orbit();
        self.refresh_info();
    }

    /// Restore the initial camera pose.
    fn reset_camera(&mut self) {
        let mut camera = Camera::new(CAMERA_EYE, [0.0, 0.0, 0.0].into(), 1600.0, 900.0);
        camera.fovy = CAMERA_FOVY;
        camera.aspect = self.program.camera.aspect;
        self.program.camera = camera;
    }

    /// Station/satellite geometry in the ECI frame for the selected station
    /// and the first satellite: `(station_eci, sat_eci, elevation_deg, range_km)`.
    fn station_geometry(&self) -> Option<(Vector3<f32>, Vector3<f32>, f32, f32)> {
        let system = &self.program.system;
        let station = system.ground_stations.get(self.station_index)?;
        let orbit = system.orbits.first()?;
        let sat = orbit.satellites.first()?;

        let cart = station.cartesian();
        let rot = Rotation3::from_axis_angle(&Vector3::z_axis(), -system.earth_rotation() as f32);
        let station_eci = rot * Vector3::new(cart[0], cart[1], cart[2]);

        let sat_pos = orbit.position(system.elapsed_seconds(), sat);
        let sat_eci = Vector3::new(sat_pos[0], sat_pos[1], sat_pos[2]);

        let up = station_eci.normalize();
        let elevation = ((sat_eci - station_eci).normalize().dot(&up))
            .clamp(-1.0, 1.0)
            .asin()
            .to_degrees();
        let range = (sat_eci - station_eci).norm();
        Some((station_eci, sat_eci, elevation, range))
    }

    /// Rebuild the per-frame Sun, node and contact-link lines (ECI frame).
    fn refresh_dynamic_lines(&mut self) {
        let (day, hour) = self.program.system.day_hour();
        let sun_dir = Astral::sun_inertial_position(day, hour);
        let contact = self.station_geometry();

        let shapes = &mut self.program.system.shapes;
        shapes.lines.retain(|l| {
            l.label != "Sun direction" && l.label != "Station link"
        });
        shapes.add_sun_line(
            FrameMode::Eci,
            [sun_dir[0] as f32, sun_dir[1] as f32, sun_dir[2] as f32],
            gui::model::system::EARTH_RADIUS_KM * 2.0,
        );
        if let Some((station_eci, sat_eci, elevation, _)) = contact
            && elevation > 0.0
        {
            shapes.add_colored_line(
                FrameMode::Eci,
                station_eci.into(),
                sat_eci.into(),
                LINK_LINE_COLOR,
                "Station link",
            );
        }

        self.refresh_keplerian();
        self.refresh_longitudes();
    }

    /// Rebuild the on-scene Keplerian-element visualization from the current
    /// orbit (tracking the J2-drifted RAAN) unless the toggle is off.
    fn refresh_keplerian(&mut self) {
        let system = &mut self.program.system;
        system.shapes.orbital_elements.clear();
        if !self.show_keplerian {
            return;
        }
        if let Some(orbit) = system.orbits.first() {
            let raan_eff = orbit.raan_deg_at(system.elapsed_seconds());
            system.shapes.add_orbital_elements(
                orbit.semi_major_axis,
                orbit.inclination_deg,
                raan_eff,
                orbit.arg_perigee_deg,
            );
        }
    }

    /// Rebuild the longitude grid (meridians every 15°) with a local-time label
    /// on each, unless the toggle is off. Local solar time = UTC hour + lon/15.
    fn refresh_longitudes(&mut self) {
        const LONGITUDE_COLOR: [f32; 3] = [0.45, 0.5, 0.6];
        let system = &mut self.program.system;
        system.shapes.longitude_lines.clear();
        if !self.show_longitudes {
            return;
        }
        let (_, hour) = system.day_hour();
        for lon in (-180..180).step_by(15) {
            let lon = lon as f32;
            let local = wrap_hours(hour + lon as f64 / 15.0);
            system
                .shapes
                .add_longitude_line(lon, format_hour(local), LONGITUDE_COLOR);
        }
    }

    fn control_panel(&self) -> Element<'_, Message> {
        let title = text("Sun-Synchronous Orbit")
            .size(18.0)
            .color(Color::from_rgb(0.92, 0.92, 0.95));

        // --- Time controls ---
        let time_title = section_title("Time");
        let pause = toggler(self.program.paused)
            .label("Pause")
            .on_toggle(Message::TogglePause)
            .size(18.0)
            .text_size(13.0)
            .style(toggler_style);
        let speed = param_slider(
            &format!(
                "Speed: {}x (1 day ≈ {:.0} s)",
                2.0_f32.powf(self.speed_exp),
                86400.0 / 2.0_f32.powf(self.speed_exp)
            ),
            0.0..=14.0,
            self.speed_exp,
            Message::SpeedChanged,
        );

        let clock = dyn_text(
            format!(
                "UTC: {}",
                self.program.system.simulation_date_string()
            ),
            13.0,
            Color::from_rgb(0.75, 0.78, 0.85),
        );

        // --- Season presets ---
        // The orbit plane stays locked to the Sun all year, but the Earth's
        // axial tilt (solar declination) changes: watch the terminator swing
        // between the equinox and solstice dates while the Sun line keeps its
        // constant angle to the orbit plane.
        let season_title = section_title("Season (restart date)");
        let seasons = row![
            panel_button("Mar equinox", Message::SeasonChanged(3, 20)),
            panel_button("Jun solstice", Message::SeasonChanged(6, 21)),
            panel_button("Sep equinox", Message::SeasonChanged(9, 22)),
            panel_button("Dec solstice", Message::SeasonChanged(12, 21)),
        ]
        .spacing(4.0)
        .wrap();

        // --- Orbit controls ---
        let orbit_title = section_title("Orbit");
        let sso_state = if self.sso_lock {
            "ON (locked to Sun)"
        } else {
            "OFF (free inclination)"
        };
        let lock = toggler(self.sso_lock)
            .label(format!("Sun-synchronous: {sso_state}"))
            .on_toggle(Message::ToggleSsoLock)
            .size(18.0)
            .text_size(13.0)
            .style(toggler_style);

        let altitude = param_slider(
            &format!("Altitude: {:.0} km", self.altitude_km),
            400.0..=1200.0,
            self.altitude_km,
            Message::AltitudeChanged,
        );
        let inclination = param_slider(
            &format!("Inclination: {:.2}°", self.inclination_deg),
            70.0..=110.0,
            self.inclination_deg,
            Message::InclinationChanged,
        );
        let ltan = with_tip(
            param_slider(
                &format!("LTAN (initial): {}", format_hour(self.ltan_hours as f64)),
                0.0..=24.0,
                self.ltan_hours,
                Message::LtanChanged,
            ),
            "Local Time of Ascending Node - the local solar time at which the satellite \
             crosses the equator heading north. For a sun-synchronous orbit it stays \
             constant every day. 06:00 puts the orbit plane through the dawn/dusk \
             terminator (plane contains the Sun); 12:00 makes every pass near local \
             noon. Changing it re-phases the RAAN at the current simulation time.",
        );
        // Classic mission choices: the plane containing the Sun (dawn-dusk,
        // local midnight/moon at the nodes), a mid-morning imaging pass
        // (Sentinel-like), or a local-noon orbit.
        let ltan_presets = row![
            panel_button("06:00 dawn-dusk", Message::LtanChanged(6.0)),
            panel_button("10:30 imaging", Message::LtanChanged(10.5)),
            panel_button("12:00 noon", Message::LtanChanged(12.0)),
        ]
        .spacing(4.0)
        .wrap();

        let j2 = toggler(self.program.system.orbits[0].with_j2)
            .label("J2 perturbation (node precession)")
            .on_toggle(Message::ToggleJ2)
            .size(18.0)
            .text_size(13.0)
            .style(toggler_style);

        // --- Constellation ---
        let constellation_title = section_title("Constellation");
        let sats = param_slider(
            &format!("Satellites: {}", self.sat_count),
            1.0..=8.0,
            self.sat_count as f32,
            Message::SatellitesChanged,
        );

        // --- Ground station ---
        let station_title = section_title("Ground station (contact demo)");
        let station_pick = column(
            self.program
                .system
                .ground_stations
                .iter()
                .enumerate()
                .map(|(i, st)| {
                    radio(
                        format!(
                            "{}  ({:.1}°, {:.1}°)",
                            st.name, st.latitude_deg, st.longitude_deg
                        ),
                        i,
                        Some(self.station_index),
                        Message::StationChanged,
                    )
                    .size(14.0)
                    .text_size(12.0)
                    .spacing(6.0)
                    .style(radio_style)
                    .into()
                })
                .collect::<Vec<Element<'_, Message>>>(),
        )
        .spacing(2.0);

        // --- View ---
        let view_title = section_title("View");
        let frame_label = match self.program.frame_mode {
            FrameMode::Eci => "Frame: ECI (inertial)",
            FrameMode::Ecef => "Frame: ECEF (Earth-fixed)",
        };
        let frame = panel_button(&frame_label, Message::ToggleFrameMode);
        let reset_view = panel_button("Reset camera", Message::ResetCamera);
        let frame_row = row![frame, reset_view].spacing(6.0).wrap();

        let orbit = &self.program.system.orbits[0];
        let orbit_path = toggler(orbit.show_orbit)
            .label("Orbit path")
            .on_toggle(Message::ToggleOrbitPath)
            .size(16.0)
            .text_size(13.0)
            .style(toggler_style);
        let fov = toggler(orbit.show_fov)
            .label("Satellite FOV footprint")
            .on_toggle(Message::ToggleFov)
            .size(16.0)
            .text_size(13.0)
            .style(toggler_style);
        let fov_fill = toggler(orbit.fill_fov)
            .label("Fill FOV footprint")
            .on_toggle(Message::ToggleFovFill)
            .size(16.0)
            .text_size(13.0)
            .style(toggler_style);
        let fov_angle = param_slider(
            &format!("FOV half-angle: {:.0}°", orbit.fov_half_angle_deg),
            5.0..=60.0,
            orbit.fov_half_angle_deg,
            Message::FovAngleChanged,
        );
        let keplerian = toggler(self.show_keplerian)
            .label("Keplerian elements")
            .on_toggle(Message::ToggleKeplerian)
            .size(16.0)
            .text_size(13.0)
            .style(toggler_style);
        let longitudes = toggler(self.show_longitudes)
            .label("Longitude grid + local time")
            .on_toggle(Message::ToggleLongitudes)
            .size(16.0)
            .text_size(13.0)
            .style(toggler_style);

        // --- Live readouts ---
        let info_title = section_title("Live info (hover terms for help)");
        let info = SunSynchronousSimulation::info_view(&self.info_lines);

        let explain = text(
            "A retrograde orbit (i ≈ 96–100°) gets an eastward nodal push \
             from Earth's J2 oblateness. At the right altitude/inclination \
             this precession matches the Sun's apparent 0.9856°/day motion, \
             so the orbit plane keeps a constant angle to the Sun and every \
             equator crossing happens at the same local solar time. \
             Enable the Keplerian elements toggle to watch the orbit plane \
             stay fixed relative to the orange Sun line — then disable J2 or \
             move inclination off the SSO value and see them drift apart.\n\n\
             Try: pick Svalbard — it sees every pass (green contact link) \
             because SSO planes hug the terminator near the poles; Toulouse \
             only gets short contacts. Add satellites to fill coverage gaps. \
             Switch seasons: the terminator tilts, yet the Sun/node geometry \
             barely changes — that is the point of a sun-synchronous orbit.",
        )
        .size(11.0)
        .color(Color::from_rgb(0.5, 0.52, 0.58));

        let content = column![
            title,
            time_title,
            pause,
            speed,
            clock,
            season_title,
            seasons,
            orbit_title,
            lock,
            altitude,
            inclination,
            ltan,
            ltan_presets,
            j2,
            constellation_title,
            sats,
            station_title,
            station_pick,
            view_title,
            frame_row,
            orbit_path,
            fov,
            fov_fill,
            fov_angle,
            keplerian,
            longitudes,
            info_title,
            info,
            explain,
        ]
        .spacing(10.0)
        .padding(12.0)
        .width(Length::Fill);

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.11, 0.11, 0.14))),
                border: Border {
                    color: Color::from_rgb(0.22, 0.22, 0.26),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            })
            .into()
    }

    /// Recompute the "Live info" readouts from the current simulation state.
    fn refresh_info(&mut self) {
        self.info_lines = self.info_lines_text();
    }

    /// Render the "Live info" rows: aligned term/value columns where hoverable
    /// terms carry their pedagogical tooltip.
    fn info_view(lines: &[InfoLine]) -> Element<'static, Message> {
        let mut col = column![].spacing(3.0);
        for line in lines {
            match line {
                InfoLine::Header(title) => {
                    col = col.push(
                        text(title.clone())
                            .size(12.5)
                            .color(Color::from_rgb(0.55, 0.8, 0.95)),
                    );
                }
                InfoLine::Row { term, value, tip } => {
                    let term_widget = text(term.clone())
                        .size(11.5)
                        .color(Color::from_rgb(0.72, 0.75, 0.82));
                    let term_widget: Element<'_, Message> = match tip {
                        Some(tip) => tooltip(
                            term_widget,
                            tip_box(tip.as_str()),
                            tooltip::Position::Bottom,
                        )
                        .gap(3.0)
                        .into(),
                        None => term_widget.into(),
                    };
                    col = col.push(
                        row![
                            container(term_widget).width(Length::Fixed(112.0)),
                            text(value.clone()).size(11.5).color(PANEL_TEXT),
                        ]
                        .spacing(8.0),
                    );
                }
            }
        }
        col.into()
    }

    /// All live quantities shown in the panel, as tooltip-annotated rows.
    fn info_lines_text(&self) -> Vec<InfoLine> {
        const TIP_ORBIT: &str = "Semi-major axis (altitude + Earth radius), orbital period, and \
            inclination of the rendered orbit.";
        const TIP_RAAN: &str = "Right Ascension of the Ascending Node - inertial longitude of the \
            orbit's northbound equator crossing. J2 makes it drift about 1 deg/day east for \
            sun-synchronous orbits.";
        const TIP_DRIFT: &str = "Rate at which Earth's J2 oblateness pushes the orbit plane: \
            eastward for retrograde orbits, westward for prograde ones. A sun-synchronous orbit \
            needs +0.9856 deg/day to follow the Sun's apparent motion.";
        const TIP_DRIFT_OFF: &str = "J2 is disabled: the orbit plane no longer precesses and \
            stays fixed in inertial space while the Sun still appears to move ~1 deg/day.";
        const TIP_NODE_LON: &str = "Geographic longitude of the ascending node right now. The \
            node is fixed in the inertial frame, so Earth rotation sweeps this longitude \
            westward through the day.";
        const TIP_LTAN: &str = "Local Time of Ascending Node - the local solar time at which the \
            satellite crosses the equator heading north (06:00 = dawn-dusk orbit). It stays \
            constant from day to day exactly when the orbit is sun-synchronous.";
        const TIP_SAT_LOCAL: &str = "Local solar time at the satellite's current sub-satellite \
            point (UTC hour + longitude/15). A sun-synchronous orbit passes any longitude at \
            the same local time on every orbit, so this readout repeats each day.";
        const TIP_LTDN: &str = "Local Time of Descending Node - the southbound equator crossing, \
            always 12 h after LTAN.";
        const TIP_BETA: &str = "Angle between the Sun direction and the orbit plane (beta angle). \
            It is constant exactly when the orbit is sun-synchronous; watch it stay put while \
            the seasons change.";
        const TIP_SUBSOLAR: &str = "Point on Earth where the Sun is at zenith right now. Its \
            latitude is the solar declination.";
        const TIP_DECL: &str = "Solar declination - latitude of the subsolar point, swinging \
            between +23.4 deg and -23.4 deg across the seasons. It tilts the day/night terminator.";
        const TIP_SOLAR_TIME: &str = "Local solar time at the station's longitude: \
            UTC hour + longitude/15 h.";
        const TIP_SUN_ELEV: &str = "Height of the Sun above the station's horizon; \
            negative means night.";
        const TIP_CONTACT: &str = "View of the satellite from the selected station: elevation \
            above the horizon and slant range. A contact needs the satellite higher than the \
            station's minimum elevation (mask).";

        let row = |term: &str, value: String, tip: Option<&str>| InfoLine::Row {
            term: term.to_string(),
            value,
            tip: tip.map(str::to_string),
        };

        let system = &self.program.system;
        let (day, hour) = system.day_hour();
        let elapsed = system.elapsed_seconds();
        let orbit = &system.orbits[0];
        let station = &system.ground_stations[self.station_index];

        // Effective RAAN and node geometry
        let raan_eff = orbit.raan_deg_at(elapsed);
        let gmst_deg = Astral::earth_rotation_angle(day, hour).to_degrees();
        let node_lon = wrap_deg(raan_eff as f64 - gmst_deg);
        let ltan = wrap_hours(hour + node_lon / 15.0);
        let ltdn = wrap_hours(ltan + 12.0);

        // Nodal drift vs the sun-synchronous requirement
        let (drift, drift_tip) = match orbit.raan_drift_rate_rad_per_s() {
            Some(rate) => {
                let deg_day = rate * 86400.0 * 180.0 / std::f64::consts::PI;
                (
                    format!(
                        "{:+.4}°/day (need {:+.4})",
                        deg_day,
                        constants::OMEGA_SUNSYNC_DEG_PER_DAY
                    ),
                    TIP_DRIFT,
                )
            }
            None => ("J2 off — plane fixed".to_string(), TIP_DRIFT_OFF),
        };

        // Angle between the Sun direction and the orbit plane (constant for SSO)
        let sun = Astral::sun_inertial_position(day, hour);
        let inc = orbit.inclination_deg.to_radians() as f64;
        let phi = raan_eff.to_radians() as f64;
        let normal = [inc.sin() * phi.sin(), -inc.sin() * phi.cos(), inc.cos()];
        let dot = (sun[0] * normal[0] + sun[1] * normal[1] + sun[2] * normal[2]).clamp(-1.0, 1.0);
        let sun_plane_angle = (std::f64::consts::FRAC_PI_2 - dot.acos()).to_degrees();

        // Local solar time at the satellite's current sub-satellite point.
        let sat_local_time = match orbit.satellites.first() {
            Some(sat) => {
                let sat_eci = Vector3::from(orbit.position(elapsed, sat));
                let rot =
                    Rotation3::from_axis_angle(&Vector3::z_axis(), system.earth_rotation() as f32);
                let sat_ecef = rot * sat_eci;
                let sat_lon = wrap_deg((sat_ecef.y.atan2(sat_ecef.x)).to_degrees() as f64 - 180.0);
                format_hour(wrap_hours(hour + sat_lon / 15.0))
            }
            None => "n/a".to_string(),
        };

        // Ground station local time, sun elevation, and satellite contact
        let station_solar_time = wrap_hours(hour + station.longitude_deg as f64 / 15.0);
        let astro = Astral::create(station.latitude_deg as f64, station.longitude_deg as f64);
        let (_, elevation) = astro.sun_position(day, hour);
        let (subsolar_lat, subsolar_lon) = Astral::subsolar_point(day, hour);
        let sun_elev = elevation.to_degrees();

        let contact = match self.station_geometry() {
            Some((_, _, elev, range)) => {
                let status = if elev >= station.min_elevation_deg {
                    "IN CONTACT"
                } else if elev > 0.0 {
                    "above horizon"
                } else {
                    "below horizon"
                };
                format!("{status} · {elev:+.1}° · {range:.0} km")
            }
            None => "n/a".to_string(),
        };

        vec![
            InfoLine::Header("Orbit & plane".to_string()),
            row(
                "a · T · i",
                format!(
                    "{:.0} km · {:.1} min · {:.2}°",
                    orbit.semi_major_axis,
                    orbit.period_seconds / 60.0,
                    orbit.inclination_deg
                ),
                Some(TIP_ORBIT),
            ),
            row(
                "RAAN now",
                format!("{:.2}°", raan_eff.rem_euclid(360.0)),
                Some(TIP_RAAN),
            ),
            row("Nodal drift", drift, Some(drift_tip)),
            row("Node lon", format!("{:+.1}°", node_lon), Some(TIP_NODE_LON)),
            row("LTAN", format_hour(ltan), Some(TIP_LTAN)),
            row("LTDN", format_hour(ltdn), Some(TIP_LTDN)),
            row("Sat local time", sat_local_time, Some(TIP_SAT_LOCAL)),
            row(
                "Sun/plane angle",
                format!("{:.2}°", sun_plane_angle),
                Some(TIP_BETA),
            ),
            InfoLine::Header("Sun & season".to_string()),
            row(
                "Subsolar point",
                format!("({:.1}°, {:.1}°)", subsolar_lat, subsolar_lon),
                Some(TIP_SUBSOLAR),
            ),
            row(
                "Declination",
                format!("{:.1}°", Astral::solar_declination_deg(day)),
                Some(TIP_DECL),
            ),
            InfoLine::Header(format!("Station: {}", station.name)),
            row(
                "Local solar time",
                format_hour(station_solar_time),
                Some(TIP_SOLAR_TIME),
            ),
            row(
                "Sun elevation",
                format!(
                    "{sun_elev:+.1}° {}",
                    if sun_elev > 0.0 { "(day)" } else { "(night)" }
                ),
                Some(TIP_SUN_ELEV),
            ),
            row(
                "Satellite",
                format!("{} · min elev {:.0}°", contact, station.min_elevation_deg),
                Some(TIP_CONTACT),
            ),
        ]
    }
}

/// Panel text color shared by all styled widgets.
const PANEL_TEXT: Color = Color::from_rgb(0.85, 0.85, 0.9);

/// Text whose content may change over time.
fn dyn_text<'a>(content: String, size: f32, color: Color) -> Element<'a, Message> {
    text(content)
        .size(size)
        .color(color)
        .shaping(iced::widget::text::Shaping::Basic)
        .into()
}

/// Dark rounded tooltip body.
fn tip_box<'a>(tip: impl Into<String>) -> Element<'a, Message> {
    container(
        text(tip.into())
            .size(11.0)
            .color(PANEL_TEXT)
            .width(Length::Fixed(260.0)),
    )
    .padding(8.0)
    .style(|_theme| container::Style {
        background: Some(Background::Color(Color::from_rgb(0.13, 0.14, 0.18))),
        border: Border {
            color: Color::from_rgb(0.42, 0.45, 0.52),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}

/// Wrap any widget with a hover tooltip.
fn with_tip<'a>(content: Element<'a, Message>, tip: impl Into<String>) -> Element<'a, Message> {
    tooltip(content, tip_box(tip), tooltip::Position::Bottom)
        .gap(4.0)
        .into()
}

/// Toggler style based on the theme default but with readable light label text
/// (the default theme renders labels near-black on the dark panel).
fn toggler_style(
    theme: &iced::Theme,
    status: iced::widget::toggler::Status,
) -> iced::widget::toggler::Style {
    let mut style = iced::widget::toggler::default(theme, status);
    style.text_color = Some(PANEL_TEXT);
    style
}

/// Radio style based on the theme default but with readable light label text.
fn radio_style(
    theme: &iced::Theme,
    status: iced::widget::radio::Status,
) -> iced::widget::radio::Style {
    let mut style = iced::widget::radio::default(theme, status);
    style.text_color = Some(PANEL_TEXT);
    style
}

/// Section header inside the control panel.
fn section_title(label: &'static str) -> Element<'static, Message> {
    text(label.to_string())
        .size(15.0)
        .color(Color::from_rgb(0.85, 0.85, 0.9))
        .into()
}

/// Helper to create a labeled slider row.
fn param_slider<'a>(
    label: &str,
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    let lbl = dyn_text(
        label.to_string(),
        13.0,
        Color::from_rgb(0.62, 0.62, 0.68),
    );
    let sl = slider(range, value, on_change);
    column![lbl, sl].spacing(4.0).into()
}

/// Compact preset button styled to match the dark panel.
fn panel_button<'a>(label: &str, on_press: Message) -> Element<'a, Message> {
    button(
        text(label.to_string())
            .size(12.0)
            .color(Color::from_rgb(0.85, 0.85, 0.9)),
    )
    .padding([4.0, 8.0])
    .style(|_theme, status| {
        let (bg, border_color) = match status {
            button::Status::Pressed => (
                Color::from_rgb(0.22, 0.24, 0.30),
                Color::from_rgb(0.35, 0.38, 0.45),
            ),
            button::Status::Hovered => (
                Color::from_rgb(0.20, 0.21, 0.26),
                Color::from_rgb(0.35, 0.38, 0.45),
            ),
            _ => (
                Color::from_rgb(0.16, 0.17, 0.21),
                Color::from_rgb(0.28, 0.30, 0.36),
            ),
        };
        button::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 3.0.into(),
            },
            text_color: Color::from_rgb(0.85, 0.85, 0.9),
            ..button::Style::default()
        }
    })
    .on_press(on_press)
    .into()
}

impl iced::widget::shader::Program<Message> for SunSynchronousSimulation {
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

fn update(sim: &mut SunSynchronousSimulation, message: Message) -> iced::Task<Message> {
    match message {
        Message::Tick => {
            if !sim.program.paused {
                sim.program.tick();
            }
            sim.refresh_dynamic_lines();
            sim.tick_count += 1;
            if sim.tick_count.is_multiple_of(30) {
                sim.refresh_info();
            }
            if let Some(task) = sim.shot.on_frame() {
                return task.map(Message::Shot);
            }
        }
        Message::AltitudeChanged(v) => {
            sim.altitude_km = v;
            if sim.sso_lock
                && let Some(inc) = Astral::sun_synchronous_inclination(v as f64, 0.0)
            {
                sim.inclination_deg = inc as f32;
            }
            sim.apply_orbit();
        }
        Message::InclinationChanged(v) => {
            sim.inclination_deg = v;
            if sim.sso_lock
                && let Some(alt) = Astral::sun_synchronous_altitude(v as f64, 0.0)
            {
                sim.altitude_km = alt as f32;
            }
            sim.apply_orbit();
        }
        Message::LtanChanged(v) => {
            sim.ltan_hours = v;
            sim.apply_orbit();
        }
        Message::ToggleSsoLock(v) => {
            sim.sso_lock = v;
            if v && let Some(inc) = Astral::sun_synchronous_inclination(sim.altitude_km as f64, 0.0)
            {
                sim.inclination_deg = inc as f32;
            }
            sim.apply_orbit();
        }
        Message::ToggleJ2(v) => {
            sim.program.system.orbits[0].with_j2 = v;
            sim.refresh_dynamic_lines();
            sim.tick_count += 1;
            if sim.tick_count.is_multiple_of(30) {
                sim.refresh_info();
            }
        }
        Message::SatellitesChanged(v) => {
            sim.sat_count = v.round().clamp(1.0, 8.0) as u32;
            sim.rebuild_satellites();
            sim.refresh_dynamic_lines();
            sim.refresh_info();
        }
        Message::SeasonChanged(month, day) => {
            sim.set_season(month, day);
        }
        Message::StationChanged(i) => {
            let max = sim.program.system.ground_stations.len().saturating_sub(1);
            sim.station_index = i.min(max);
            sim.refresh_dynamic_lines();
            sim.refresh_info();
        }
        Message::ToggleFrameMode => {
            sim.program.frame_mode = match sim.program.frame_mode {
                FrameMode::Eci => FrameMode::Ecef,
                FrameMode::Ecef => FrameMode::Eci,
            };
        }
        Message::ResetCamera => {
            sim.reset_camera();
        }
        Message::ToggleFov(v) => {
            sim.program.system.orbits[0].show_fov = v;
        }
        Message::ToggleFovFill(v) => {
            sim.program.system.orbits[0].fill_fov = v;
        }
        Message::FovAngleChanged(v) => {
            sim.program.system.orbits[0].fov_half_angle_deg = v;
        }
        Message::ToggleOrbitPath(v) => {
            sim.program.system.orbits[0].show_orbit = v;
        }
        Message::ToggleKeplerian(v) => {
            sim.show_keplerian = v;
            sim.refresh_keplerian();
        }
        Message::ToggleLongitudes(v) => {
            sim.show_longitudes = v;
            sim.refresh_longitudes();
        }
        Message::TogglePause(v) => {
            sim.program.paused = v;
            if !v {
                sim.program.system.last_tick_time = Utc::now();
                sim.program.system.accumulator = chrono::TimeDelta::zero();
            }
            sim.refresh_info();
        }
        Message::SpeedChanged(v) => {
            sim.speed_exp = v;
            sim.program.set_time_scale(2.0_f32.powf(v));
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

fn view(sim: &SunSynchronousSimulation) -> Element<'_, Message> {
    let scene = shader(sim).width(Length::Fill).height(Length::Fill);
    let panel = sim.control_panel();

    let panel_col = container(panel)
        .width(Length::Fixed(320.0))
        .height(Length::Fill);

    row![panel_col, scene]
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn main() -> iced::Result {
    env_logger::init();

    let mut app = iced::application(SunSynchronousSimulation::new, update, view);
    if let Some(size) = gui::screenshot::window_size() {
        app = app.window_size(size);
    }
    app.subscription(|_state: &SunSynchronousSimulation| {
        subscription(|_| Message::Tick, Message::Event)
    })
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_lines_are_populated() {
        let sim = SunSynchronousSimulation::new();
        let lines = sim.info_lines_text();
        println!("INFO LINES:\n{lines:#?}");
        // Three section headers plus at least a dozen term/value rows.
        let headers = lines
            .iter()
            .filter(|l| matches!(l, InfoLine::Header(_)))
            .count();
        let rows = lines
            .iter()
            .filter(|l| matches!(l, InfoLine::Row { .. }))
            .count();
        assert_eq!(headers, 3);
        assert!(rows >= 12);
        // Every row carries a pedagogical tooltip.
        assert!(lines.iter().all(|l| match l {
            InfoLine::Header(_) => true,
            InfoLine::Row { tip, .. } => tip.is_some(),
        }));
    }

    /// The drawn orbit track must pass through the satellite: both are sampled
    /// around the same elapsed time, so the J2-drifted plane is identical even
    /// after hours of simulated precession.
    #[test]
    fn satellite_rides_orbit_track_after_precession() {
        let sim = SunSynchronousSimulation::new();
        let system = &sim.program.system;
        let orbit = &system.orbits[0];
        let elapsed = 86_400.0_f32; // one simulated day of nodal drift

        let sat = &orbit.satellites[0];
        let sat_pos = orbit.position(elapsed, sat);
        let p = Vector3::new(sat_pos[0], sat_pos[1], sat_pos[2]);
        let r = p.norm();

        let ring = orbit.generate_orbit_positions_at(elapsed, 512);
        let min_dist = ring
            .iter()
            .map(|q| {
                let q = Vector3::new(q[0], q[1], q[2]);
                (p - q).norm()
            })
            .fold(f32::INFINITY, f32::min);

        // Ring discretization spacing at 512 samples:
        let step = std::f32::consts::TAU * r / 512.0;
        assert!(
            min_dist <= step,
            "satellite is {min_dist} km off the drawn track (step {step})"
        );
    }
}
