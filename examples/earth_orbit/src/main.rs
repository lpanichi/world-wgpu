/// Earth's annual revolution & seasons teaching example.
///
/// Teaches why seasons happen and how the Earth revolves around the Sun
/// through the year:
/// - The axial tilt (23.44°) — not distance — is what causes the seasons
/// - As the Earth revolves, the Sun direction sweeps the ecliptic ring once a
///   year; the ring is drawn tilted by the axial tilt relative to the equator
/// - The Sun's declination oscillates ±23.44°, moving the subsolar point
///   between the Tropics and tilting the day/night terminator
/// - The equinoxes and solstices are fixed points on that annual path
///
/// Interactive panel:
/// - Time: pause, speed (days per second), and a day-of-year scrubber
/// - Season presets: March equinox / June solstice / September equinox /
///   December solstice
/// - Latitude: pick a place to read its day length & noon sun elevation
/// - View: Sun-path ring, tropics/polar circles, subsolar marker, longitude
///   grid with local-time labels
/// - Readouts: solar declination, subsolar point, season in both hemispheres,
///   and why seasons are not caused by Earth's distance to the Sun
use chrono::{Duration, TimeZone, Utc};
use gui::astro::Astral;
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::FrameMode;
use gui::model::system::{EARTH_RADIUS_KM, System};
use gui::simulation::Simulation as ProgramSimulation;
use gui::viewer::{CameraControl, subscription};
use iced::mouse;
use iced::widget::{
    button, column, container, row, scrollable, shader, slider, text, toggler, tooltip,
};
use iced::{Background, Border, Color, Element, Length};
use nalgebra::Point3;

const SUBSOLAR_COLOR: [f32; 3] = [1.0, 0.4, 0.1];
const TROPIC_COLOR: [f32; 3] = [0.5, 0.85, 1.0];
const POLAR_COLOR: [f32; 3] = [0.35, 0.7, 0.95];

/// Initial camera pose (restored by "Reset view"). Frames the Earth and the
/// tilted Sun-path ring that shows the annual revolution.
const CAMERA_EYE: Point3<f32> = Point3::new(-26_000.0, -17_000.0, 12_000.0);
const CAMERA_FOVY: f32 = 45.0;

/// Radius of the Sun's annual path ring in km.
const SUN_PATH_RADIUS: f32 = EARTH_RADIUS_KM * 2.5;

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Event(iced::event::Event),
    Shot(gui::screenshot::ShotMessage),
    // Time controls
    TogglePause(bool),
    SpeedChanged(f32),
    DayChanged(f32),
    SeasonChanged(u32),
    // Latitude control
    LatitudeChanged(f32),
    // View controls
    ResetCamera,
    ToggleSunPath(bool),
    ToggleCircles(bool),
    ToggleSubsolar(bool),
    ToggleLongitudes(bool),
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

struct EarthOrbitSimulation {
    program: ProgramSimulation,
    /// Simulation speed in days per real second.
    days_per_sec: f32,
    /// Current day of year (1..365), kept in sync with the running clock.
    day_of_year: f32,
    /// Latitude (degrees) used for the day-length / sun-elevation readouts.
    latitude_deg: f32,
    // View toggles
    show_sun_path: bool,
    show_circles: bool,
    show_subsolar: bool,
    show_longitudes: bool,
    info_lines: Vec<InfoLine>,
    tick_count: u32,
    // Camera interaction
    control: CameraControl,
    shot: gui::screenshot::AutoShot,
}

/// Normalize hours to [0, 24).
fn wrap_hours(h: f64) -> f64 {
    h.rem_euclid(24.0)
}

/// Ecliptic longitude (degrees) of the Sun for the given day/hour — the angle
/// swept along the annual path. 0° = March equinox, 90° = June solstice,
/// 180° = September equinox, 270° = December solstice.
fn sun_ecliptic_lon_deg(day: u32, hour: f64) -> f64 {
    let s = Astral::sun_inertial_position(day, hour);
    let eps = 23.44_f64.to_radians();
    let sin_lambda = s[1] / eps.cos();
    sin_lambda.atan2(s[0]).to_degrees().rem_euclid(360.0)
}

/// Hours of daylight at a latitude for a given solar declination (degrees).
/// Handles polar day (24 h) and polar night (0 h).
fn daylight_hours(lat_deg: f64, decl_deg: f64) -> f64 {
    let arg = -(lat_deg.to_radians().tan() * decl_deg.to_radians().tan());
    if arg > 1.0 {
        return 0.0; // polar night
    }
    if arg < -1.0 {
        return 24.0; // polar day
    }
    24.0 * arg.acos() / std::f64::consts::PI
}

/// Noon solar elevation (degrees) above the horizon at a latitude.
fn noon_elevation_deg(lat_deg: f64, decl_deg: f64) -> f64 {
    90.0 - (lat_deg - decl_deg).abs()
}

/// Season name in the northern hemisphere (southern is opposite).
fn season_north(ecliptic_lon_deg: f64) -> &'static str {
    match ecliptic_lon_deg.rem_euclid(360.0) {
        l if l < 90.0 => "Spring (S: Autumn)",
        l if l < 180.0 => "Summer (S: Winter)",
        l if l < 270.0 => "Autumn (S: Spring)",
        _ => "Winter (S: Summer)",
    }
}

impl EarthOrbitSimulation {
    fn new() -> Self {
        // March equinox 2025, solar noon UTC: subsolar point near the equator.
        let start_time = Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap();

        let mut core_sim = System::builder().build(start_time);
        let axis_len = EARTH_RADIUS_KM * 2.0;
        core_sim.shapes.add_eci_frame(axis_len);

        let mut camera = Camera::new(CAMERA_EYE, [0.0, 0.0, 0.0].into(), 1600.0, 900.0);
        camera.fovy = CAMERA_FOVY;

        let program = ProgramSimulation {
            system: core_sim,
            camera,
            satellite_mode: SatelliteRenderMode::Dot,
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
            days_per_sec: 5.0,
            day_of_year: 79.0,
            latitude_deg: 45.0,
            show_sun_path: true,
            show_circles: true,
            show_subsolar: true,
            show_longitudes: false,
            info_lines: Vec::new(),
            tick_count: 0,
            control: CameraControl::default(),
            shot: gui::screenshot::AutoShot::from_env(),
        };
        sim.set_days_per_sec(sim.days_per_sec);
        sim.refresh_dynamic_shapes();
        sim.refresh_info();
        sim
    }

    /// Set the simulation clock's rate in days per real second.
    fn set_days_per_sec(&mut self, days: f32) {
        self.days_per_sec = days.clamp(0.1, 20.0);
        // Each fixed 16 ms step advances `simulation_speed` ms of sim time and
        // the fixed-step loop runs ~60 steps per real second, so
        // speed = days/s * 86400 / 0.96 ≈ days/s * 90_000.
        let speed = (self.days_per_sec * 90_000.0).round().clamp(1.0, 5_000_000.0) as i32;
        self.program.time_scale = speed as f32;
        self.program.system.simulation_speed = speed;
    }

    /// Reset the clock to an arbitrary UTC instant (kept in the 2025 year).
    fn set_sim_time(&mut self, dt: chrono::DateTime<Utc>) {
        let system = &mut self.program.system;
        system.simulation_time = dt;
        system.start_time = dt;
        system.last_tick_time = Utc::now();
        system.accumulator = chrono::TimeDelta::zero();
        let (day, _) = Astral::datetime_to_day_hour(&dt);
        self.day_of_year = day as f32;
        self.refresh_dynamic_shapes();
        self.refresh_info();
    }

    /// Jump to a specific day of year (1..365), keeping the current time of day.
    fn set_day(&mut self, day: f32) {
        let day = day.round().clamp(1.0, 365.0) as u32;
        let (_, hour) = self.program.system.day_hour();
        let base = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let dt = base + Duration::days((day - 1) as i64) + Duration::seconds((hour * 3600.0) as i64);
        self.set_sim_time(dt);
    }

    /// Jump to a season's representative date (month/day).
    fn set_season(&mut self, month: u32, day: u32) {
        let dt = Utc.with_ymd_and_hms(2025, month, day, 12, 0, 0).unwrap();
        self.set_sim_time(dt);
    }

    /// Restore the initial camera pose.
    fn reset_camera(&mut self) {
        let mut camera = Camera::new(CAMERA_EYE, [0.0, 0.0, 0.0].into(), 1600.0, 900.0);
        camera.fovy = CAMERA_FOVY;
        camera.aspect = self.program.camera.aspect;
        self.program.camera = camera;
    }

    /// Rebuild everything that depends on the current date: the Sun direction
    /// line, subsolar marker, Sun-path ring, and the optional overlays.
    fn refresh_dynamic_shapes(&mut self) {
        let (day, hour) = self.program.system.day_hour();
        self.day_of_year = day as f32;

        let sun_dir = Astral::sun_inertial_position(day, hour);
        let sun_dir_f32 = [
            sun_dir[0] as f32,
            sun_dir[1] as f32,
            sun_dir[2] as f32,
        ];
        let (subsolar_lat, subsolar_lon) = Astral::subsolar_point(day, hour);
        let today_lon = sun_ecliptic_lon_deg(day, hour);

        let shapes = &mut self.program.system.shapes;

        // Dynamic lines: keep the Sun line and subsolar radial, drop everything
        // else that was labelled for this example.
        shapes.lines.retain(|l| {
            l.label != "Sun direction" && l.label != "Subsolar radial"
        });
        shapes.add_sun_line(FrameMode::Eci, sun_dir_f32, SUN_PATH_RADIUS * 0.92);
        shapes.add_colored_surface_line(
            subsolar_lat as f32,
            subsolar_lon as f32,
            EARTH_RADIUS_KM * 0.6,
            SUBSOLAR_COLOR,
            "Subsolar radial",
        );

        // Points: subsolar marker + optional tropics / polar circles.
        shapes.points.clear();
        if self.show_subsolar {
            shapes.add_colored_surface_point(
                subsolar_lat as f32,
                subsolar_lon as f32,
                SUBSOLAR_COLOR,
                0.0,
                "Subsolar point",
            );
        }
        if self.show_circles {
            for (lat, color) in [
                (23.44, TROPIC_COLOR),
                (-23.44, TROPIC_COLOR),
                (66.56, POLAR_COLOR),
                (-66.56, POLAR_COLOR),
            ] {
                for lon in (-180..=180).step_by(30) {
                    shapes.add_colored_surface_point(lat, lon as f32, color, 0.0, "");
                }
            }
        }

        // Sun-path ring (the annual revolution), rebuilt so the today marker moves.
        shapes.sun_paths.clear();
        if self.show_sun_path {
            shapes.add_sun_path(
                SUN_PATH_RADIUS,
                today_lon as f32,
                [0.45, 0.5, 0.62],
                [1.0, 0.7, 0.2],
                [1.0, 0.4, 0.1],
            );
        }

        // Longitude grid with local-time labels.
        shapes.longitude_lines.clear();
        if self.show_longitudes {
            const LONGITUDE_COLOR: [f32; 3] = [0.45, 0.5, 0.6];
            for lon in (-180..180).step_by(15) {
                let lon = lon as f32;
                let local = wrap_hours(hour + lon as f64 / 15.0);
                let hh = local.floor() as u32;
                let mm = ((local - local.floor()) * 60.0).round() as u32;
                shapes.add_longitude_line(lon, format!("{hh:02}:{mm:02}"), LONGITUDE_COLOR);
            }
        }
    }

    fn control_panel(&self) -> Element<'_, Message> {
        let title = text("Earth's Year & the Seasons")
            .size(18.0)
            .color(Color::from_rgb(0.92, 0.92, 0.95));

        // --- Time controls ---
        let time_title = section_title("Time (Earth's annual revolution)");
        let pause = toggler(self.program.paused)
            .label("Pause")
            .on_toggle(Message::TogglePause)
            .size(18.0)
            .text_size(13.0)
            .style(toggler_style);
        let speed = param_slider(
            &format!("Speed: {:.1} days/s", self.days_per_sec),
            0.1..=20.0,
            self.days_per_sec,
            Message::SpeedChanged,
        );
        let day_slider = param_slider(
            &format!("Day of year: {:.0}", self.day_of_year),
            1.0..=365.0,
            self.day_of_year,
            Message::DayChanged,
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
        let season_title = section_title("Season presets");
        let seasons = row![
            panel_button("Mar equinox", Message::SeasonChanged(0)),
            panel_button("Jun solstice", Message::SeasonChanged(1)),
            panel_button("Sep equinox", Message::SeasonChanged(2)),
            panel_button("Dec solstice", Message::SeasonChanged(3)),
        ]
        .spacing(4.0)
        .wrap();

        // --- Latitude ---
        let latitude_title = section_title("Observer latitude");
        let latitude = with_tip(
            param_slider(
                &format!("Latitude: {:+.1}°", self.latitude_deg),
                -90.0..=90.0,
                self.latitude_deg,
                Message::LatitudeChanged,
            ),
            "Choose a latitude to read its noon sun elevation and day length \
             through the year. At the equator day length stays ~12 h; at high \
             latitudes the seasons swing between polar day and polar night.",
        );

        // --- View ---
        let view_title = section_title("View");
        let sun_path = toggler(self.show_sun_path)
            .label("Sun-path ring (annual orbit)")
            .on_toggle(Message::ToggleSunPath)
            .size(16.0)
            .text_size(13.0)
            .style(toggler_style);
        let circles = toggler(self.show_circles)
            .label("Tropics + polar circles")
            .on_toggle(Message::ToggleCircles)
            .size(16.0)
            .text_size(13.0)
            .style(toggler_style);
        let subsolar = toggler(self.show_subsolar)
            .label("Subsolar point")
            .on_toggle(Message::ToggleSubsolar)
            .size(16.0)
            .text_size(13.0)
            .style(toggler_style);
        let longitudes = toggler(self.show_longitudes)
            .label("Longitude grid + local time")
            .on_toggle(Message::ToggleLongitudes)
            .size(16.0)
            .text_size(13.0)
            .style(toggler_style);
        let reset_view = panel_button("Reset camera", Message::ResetCamera);

        // --- Live readouts ---
        let info_title = section_title("Live info (hover terms for help)");
        let info = EarthOrbitSimulation::info_view(&self.info_lines);

        let explain = text(
            "Seasons are caused by Earth's 23.44° axial tilt, not by the \
             distance to the Sun. As Earth orbits, the tilted axis keeps \
             pointing the same way in space, so the northern hemisphere leans \
             toward the Sun in June (summer solstice) and away in December \
             (winter solstice). The Sun's subsolar point slides between the \
             Tropics, dragging the day/night terminator with it and changing \
             day length everywhere.\n\n\
             Try: scrub the day-of-year slider (or pick a season preset) and \
             watch the orange Sun line walk around the tilted ring. At the \
             equinoxes the subsolar point is on the equator (day ≈ night \
             everywhere); at the solstices it reaches the Tropics. Move the \
             observer latitude toward the poles to see day length swing \
             between 0 h and 24 h.",
        )
        .size(11.0)
        .color(Color::from_rgb(0.5, 0.52, 0.58));

        let content = column![
            title,
            time_title,
            pause,
            speed,
            day_slider,
            clock,
            season_title,
            seasons,
            latitude_title,
            latitude,
            view_title,
            sun_path,
            circles,
            subsolar,
            longitudes,
            reset_view,
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
                            container(term_widget).width(Length::Fixed(120.0)),
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
        const TIP_DECL: &str = "Solar declination - the latitude of the Sun's subsolar point. \
            It oscillates between +23.44° (Tropic of Cancer, June) and -23.44° \
            (Tropic of Capricorn, December) over the year.";
        const TIP_SUBSOLAR: &str = "The point on Earth where the Sun is exactly overhead right \
            now. Its latitude is the solar declination; its longitude tracks the time of day.";
        const TIP_ECL_LON: &str = "The Sun's angle along its annual path (ecliptic longitude). \
            0° = March equinox, 90° = June solstice, 180° = September equinox, \
            270° = December solstice.";
        const TIP_SEASON: &str = "Season in the northern hemisphere (southern hemisphere is \
            opposite). Seasons flip with latitude because of the axial tilt.";
        const TIP_TILT: &str = "Earth's axial tilt (obliquity). The axis keeps pointing the \
            same way in space all year, which is what makes the Sun's height and day length \
            change with the seasons.";
        const TIP_DAYLEN: &str = "Hours of daylight at the chosen latitude for this date. \
            Depends only on latitude and solar declination: 12 h at the equator, up to 24 h \
            (polar day) or 0 h (polar night) near the poles.";
        const TIP_NOON: &str = "Maximum height of the Sun above the horizon at solar noon for \
            this latitude and date. At the equator on an equinox it is 90° (Sun straight \
            overhead).";
        const TIP_DIST: &str = "Earth is actually closest to the Sun in early January \
            (perihelion) — northern winter — which proves distance is not what drives the \
            seasons. The tilt is.";
        const TIP_VERNAL: &str = "Vernal (March) equinox: the Sun crosses the equator going \
            north. Day and night are nearly equal everywhere. This is where the year begins \
            on the annual ring (ecliptic longitude 0°).";
        const TIP_SOLSTICE: &str = "Solstices: the Sun reaches its highest (June, Tropic of \
            Cancer) or lowest (December, Tropic of Capricorn) declination. The longest and \
            shortest days of the year.";

        let system = &self.program.system;
        let (day, hour) = system.day_hour();
        let decl = Astral::solar_declination_deg(day);
        let (subsolar_lat, subsolar_lon) = Astral::subsolar_point(day, hour);
        let ecl_lon = sun_ecliptic_lon_deg(day, hour);
        let season = season_north(ecl_lon);

        let lat = self.latitude_deg as f64;
        let daylight = daylight_hours(lat, decl);
        let noon = noon_elevation_deg(lat, decl);
        let daylight_note = if daylight >= 23.99 {
            " (polar day)"
        } else if daylight <= 0.01 {
            " (polar night)"
        } else {
            ""
        };

        let row = |term: &str, value: String, tip: Option<&str>| InfoLine::Row {
            term: term.to_string(),
            value,
            tip: tip.map(str::to_string),
        };

        vec![
            InfoLine::Header("Sun & the seasons".to_string()),
            row(
                "Date",
                format!(
                    "{} (day {day})",
                    system.simulation_time.format("%b %d, %Y")
                ),
                None,
            ),
            row(
                "Solar declination",
                format!("{:+.2}°", decl),
                Some(TIP_DECL),
            ),
            row(
                "Subsolar point",
                format!("({:.1}°, {:.1}°)", subsolar_lat, subsolar_lon),
                Some(TIP_SUBSOLAR),
            ),
            row(
                "Sun on annual path",
                format!("{:.1}°", ecl_lon),
                Some(TIP_ECL_LON),
            ),
            row("Season (N)", season.to_string(), Some(TIP_SEASON)),
            InfoLine::Header(format!("At latitude {lat:+.0}°")),
            row(
                "Day length",
                format!("{daylight:.1} h{daylight_note}"),
                Some(TIP_DAYLEN),
            ),
            row(
                "Noon sun elevation",
                format!("{noon:+.1}°"),
                Some(TIP_NOON),
            ),
            InfoLine::Header("Why seasons?".to_string()),
            row("Axial tilt", "23.44°".to_string(), Some(TIP_TILT)),
            row(
                "Earth-Sun distance",
                "Nearest in January (perihelion)".to_string(),
                Some(TIP_DIST),
            ),
            row("March equinox", "Day ≈ night everywhere".to_string(), Some(TIP_VERNAL)),
            row(
                "June / December solstices",
                "Longest / shortest days".to_string(),
                Some(TIP_SOLSTICE),
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

/// Toggler style based on the theme default but with readable light label text.
fn toggler_style(
    theme: &iced::Theme,
    status: iced::widget::toggler::Status,
) -> iced::widget::toggler::Style {
    let mut style = iced::widget::toggler::default(theme, status);
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

impl iced::widget::shader::Program<Message> for EarthOrbitSimulation {
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

fn update(sim: &mut EarthOrbitSimulation, message: Message) -> iced::Task<Message> {
    match message {
        Message::Tick => {
            if !sim.program.paused {
                sim.program.tick();
            }
            sim.refresh_dynamic_shapes();
            sim.tick_count += 1;
            if sim.tick_count.is_multiple_of(30) {
                sim.refresh_info();
            }
            if let Some(task) = sim.shot.on_frame() {
                return task.map(Message::Shot);
            }
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
            sim.set_days_per_sec(v);
        }
        Message::DayChanged(v) => {
            sim.set_day(v);
        }
        Message::SeasonChanged(i) => {
            match i {
                0 => sim.set_season(3, 20),
                1 => sim.set_season(6, 21),
                2 => sim.set_season(9, 22),
                _ => sim.set_season(12, 21),
            }
        }
        Message::LatitudeChanged(v) => {
            sim.latitude_deg = v;
            sim.refresh_info();
        }
        Message::ResetCamera => {
            sim.reset_camera();
        }
        Message::ToggleSunPath(v) => {
            sim.show_sun_path = v;
            sim.refresh_dynamic_shapes();
        }
        Message::ToggleCircles(v) => {
            sim.show_circles = v;
            sim.refresh_dynamic_shapes();
        }
        Message::ToggleSubsolar(v) => {
            sim.show_subsolar = v;
            sim.refresh_dynamic_shapes();
        }
        Message::ToggleLongitudes(v) => {
            sim.show_longitudes = v;
            sim.refresh_dynamic_shapes();
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

fn view(sim: &EarthOrbitSimulation) -> Element<'_, Message> {
    let scene = shader(sim).width(Length::Fill).height(Length::Fill);
    let panel = sim.control_panel();

    let panel_col = container(panel)
        .width(Length::Fixed(340.0))
        .height(Length::Fill);

    row![panel_col, scene]
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn main() -> iced::Result {
    env_logger::init();

    let mut app = iced::application(EarthOrbitSimulation::new, update, view);
    if let Some(size) = gui::screenshot::window_size() {
        app = app.window_size(size);
    }
    app.subscription(|_state: &EarthOrbitSimulation| {
        subscription(|_| Message::Tick, Message::Event)
    })
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_lines_are_populated() {
        let sim = EarthOrbitSimulation::new();
        let lines = sim.info_lines_text();
        println!("INFO LINES:\n{lines:#?}");
        let headers = lines
            .iter()
            .filter(|l| matches!(l, InfoLine::Header(_)))
            .count();
        let rows = lines
            .iter()
            .filter(|l| matches!(l, InfoLine::Row { .. }))
            .count();
        assert!(headers >= 3);
        assert!(rows >= 10);
    }

    #[test]
    fn equinox_subsolar_near_equator() {
        let mut sim = EarthOrbitSimulation::new();
        sim.set_season(3, 20); // March equinox
        let (day, hour) = sim.program.system.day_hour();
        let (lat, _) = Astral::subsolar_point(day, hour);
        assert!(
            lat.abs() < 3.0,
            "March equinox subsolar latitude = {lat:.2}°, expected ≈0°"
        );
        let decl = Astral::solar_declination_deg(day);
        assert!(decl.abs() < 3.0, "March equinox declination = {decl:.2}°");
        let daylight_eq = daylight_hours(0.0, decl);
        assert!(
            (daylight_eq - 12.0).abs() < 0.3,
            "Equator equinox day length = {daylight_eq:.2} h, expected ≈12 h"
        );
    }

    #[test]
    fn solstice_declination_hits_tropics() {
        let mut sim = EarthOrbitSimulation::new();
        let (decl_jun, decl_dec);
        sim.set_season(6, 21);
        decl_jun = Astral::solar_declination_deg(sim.program.system.day_hour().0);
        sim.set_season(12, 21);
        decl_dec = Astral::solar_declination_deg(sim.program.system.day_hour().0);
        assert!(
            (decl_jun - 23.44).abs() < 3.0,
            "June solstice declination = {decl_jun:.2}°, expected ≈+23.44°"
        );
        assert!(
            (decl_dec + 23.44).abs() < 3.0,
            "December solstice declination = {decl_dec:.2}°, expected ≈-23.44°"
        );
    }

    #[test]
    fn day_length_swings_with_latitude_and_season() {
        // Polar latitude: polar day at summer solstice, polar night at winter.
        assert_eq!(daylight_hours(80.0, 23.44), 24.0);
        assert_eq!(daylight_hours(80.0, -23.44), 0.0);
        // Equator stays near 12 h all year.
        let (eq_jun, eq_dec);
        eq_jun = daylight_hours(0.0, 23.44);
        eq_dec = daylight_hours(0.0, -23.44);
        assert!((eq_jun - 12.0).abs() < 0.1);
        assert!((eq_dec - 12.0).abs() < 0.1);
        // Noon elevation checks.
        assert!((noon_elevation_deg(45.0, 23.44) - 68.44).abs() < 1e-6);
        assert!((noon_elevation_deg(0.0, 0.0) - 90.0).abs() < 1e-6);
    }
}