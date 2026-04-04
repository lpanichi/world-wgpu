/// Orbital Elements validation example with interactive control panel.
///
/// Visualizes Keplerian orbital mechanics with sliders and toggles:
/// - RAAN (Ω): longitude of ascending node
/// - Inclination (i): tilt of orbital plane from equator
/// - Eccentricity (e): shape of the orbit
/// - Argument of perigee (ω): rotation within orbital plane
/// - Toggle show/hide for ascending node, orbital plane, inclination arc, perigee/apogee
use chrono::{TimeZone, Utc};
use gui::gpu::pipelines::planet::{camera::Camera, satellite::SatelliteRenderMode};
use gui::model::orbit::Orbit;
use gui::model::satellite::Satellite;
use gui::model::shapes::OrbitalElements;
use gui::model::system::System;
use gui::simulation::{FrameMode, Simulation as ProgramSimulation};
use iced::keyboard::{self, Key, key::Named};
use iced::mouse;
use iced::time;
use iced::widget::{column, container, row, scrollable, shader, slider, text, toggler};
use iced::{Background, Border, Color, Element, Length};
use nalgebra::Point3;

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Event(iced::event::Event),
    // Orbital parameter sliders
    RaanChanged(f32),
    InclinationChanged(f32),
    EccentricityChanged(f32),
    ArgPerigeeChanged(f32),
    // Toggle visibility
    ToggleAscendingNode(bool),
    ToggleOrbitalPlane(bool),
    ToggleInclinationArc(bool),
    TogglePerigeeApogee(bool),
}

struct OrbitElementsSimulation {
    program: ProgramSimulation,
    // Orbital parameters
    sma: f32,
    raan: f32,
    inclination: f32,
    eccentricity: f32,
    arg_perigee: f32,
    // Visibility toggles
    show_ascending_node: bool,
    show_orbital_plane: bool,
    show_inclination_arc: bool,
    show_perigee_apogee: bool,
    // Mouse interaction
    cursor_position: Option<(f32, f32)>,
    drag_start: Option<(f32, f32)>,
    right_button_down: bool,
}

impl OrbitElementsSimulation {
    fn new() -> Self {
        let sim_time = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        let sma = 10_000.0_f32;
        let inc = 45.0_f32;
        let raan = 30.0_f32;
        let argp = 60.0_f32;
        let period = Orbit::circular_period_seconds(sma);

        let orbit = Orbit::builder(sma, period)
            .name("Validation Orbit")
            .inclination(inc)
            .raan(raan)
            .arg_perigee(argp)
            .show_orbit(true)
            .add_satellite(Satellite::builder("Sat-1").phase_offset(0.0).build())
            .build();

        let mut core_sim = System::builder().add_orbit(orbit).build(sim_time);
        core_sim.simulation_speed = 0;

        let earth_radius = gui::model::system::EARTH_RADIUS_KM;

        // ECI frame
        core_sim.shapes.add_eci_frame(earth_radius * 2.5);

        // Orbital elements visualization
        core_sim.shapes.add_orbital_elements(sma, inc, raan, argp);

        // Reference markers
        core_sim.shapes.add_surface_point(0.0, 0.0, "(0,0)");

        // Camera from above-side
        let camera_eye = Point3::new(15_000.0, -20_000.0, 15_000.0);
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

        Self {
            program,
            sma,
            raan,
            inclination: inc,
            eccentricity: 0.0,
            arg_perigee: argp,
            show_ascending_node: true,
            show_orbital_plane: true,
            show_inclination_arc: true,
            show_perigee_apogee: true,
            cursor_position: None,
            drag_start: None,
            right_button_down: false,
        }
    }

    /// Rebuild the orbital elements shape from current parameter values.
    fn rebuild_orbital_elements(&mut self) {
        self.program.system.shapes.orbital_elements.clear();
        self.program
            .system
            .shapes
            .orbital_elements
            .push(OrbitalElements {
                semi_major_axis: self.sma,
                eccentricity: self.eccentricity,
                inclination_deg: self.inclination,
                raan_deg: self.raan,
                arg_perigee_deg: self.arg_perigee,
                show_ascending_node: self.show_ascending_node,
                show_orbital_plane: self.show_orbital_plane,
                show_inclination_arc: self.show_inclination_arc,
                show_perigee_apogee: self.show_perigee_apogee,
                ..OrbitalElements::default_colors()
            });
        self.program.system.shapes.invalidate();

        // Also rebuild the first orbit to match visualization
        if let Some(orbit) = self.program.system.orbits.first_mut() {
            orbit.inclination_deg = self.inclination;
            orbit.raan_deg = self.raan;
            orbit.arg_perigee_deg = self.arg_perigee;
        }
    }

    fn control_panel(&self) -> Element<'_, Message> {
        let title = text("Orbital Elements")
            .size(18.0)
            .color(Color::from_rgb(0.92, 0.92, 0.95));

        let raan_section = param_slider(
            &format!("RAAN (Ω): {:.1}°", self.raan),
            0.0..=360.0,
            self.raan,
            Message::RaanChanged,
        );

        let inc_section = param_slider(
            &format!("Inclination (i): {:.1}°", self.inclination),
            0.0..=180.0,
            self.inclination,
            Message::InclinationChanged,
        );

        let ecc_section = param_slider(
            &format!("Eccentricity (e): {:.3}", self.eccentricity),
            0.0..=0.9,
            self.eccentricity,
            Message::EccentricityChanged,
        );

        let argp_section = param_slider(
            &format!("Arg. Perigee (ω): {:.1}°", self.arg_perigee),
            0.0..=360.0,
            self.arg_perigee,
            Message::ArgPerigeeChanged,
        );

        let toggle_title = text("Visibility")
            .size(16.0)
            .color(Color::from_rgb(0.92, 0.92, 0.95));

        let t_node = toggler(self.show_ascending_node)
            .label("Ascending Node")
            .on_toggle(Message::ToggleAscendingNode)
            .size(18.0)
            .text_size(13.0);

        let t_plane = toggler(self.show_orbital_plane)
            .label("Orbital Plane")
            .on_toggle(Message::ToggleOrbitalPlane)
            .size(18.0)
            .text_size(13.0);

        let t_inc = toggler(self.show_inclination_arc)
            .label("Inclination Arc")
            .on_toggle(Message::ToggleInclinationArc)
            .size(18.0)
            .text_size(13.0);

        let t_pe = toggler(self.show_perigee_apogee)
            .label("Perigee / Apogee")
            .on_toggle(Message::TogglePerigeeApogee)
            .size(18.0)
            .text_size(13.0);

        let info = text(format!(
            "a = {:.0} km\nPeriod = {:.0} s",
            self.sma,
            Orbit::circular_period_seconds(self.sma),
        ))
        .size(12.0)
        .color(Color::from_rgb(0.62, 0.62, 0.68));

        let content = column![
            title,
            raan_section,
            inc_section,
            ecc_section,
            argp_section,
            toggle_title,
            t_node,
            t_plane,
            t_inc,
            t_pe,
            info,
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
}

/// Helper to create a labeled slider row.
fn param_slider<'a>(
    label: &str,
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    let lbl = text(label.to_string())
        .size(13.0)
        .color(Color::from_rgb(0.62, 0.62, 0.68));
    let sl = slider(range, value, on_change).step(0.1);
    column![lbl, sl].spacing(4.0).into()
}

impl iced::widget::shader::Program<Message> for OrbitElementsSimulation {
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

fn update(sim: &mut OrbitElementsSimulation, message: Message) {
    match message {
        Message::Tick => {}
        Message::RaanChanged(v) => {
            sim.raan = v;
            sim.rebuild_orbital_elements();
        }
        Message::InclinationChanged(v) => {
            sim.inclination = v;
            sim.rebuild_orbital_elements();
        }
        Message::EccentricityChanged(v) => {
            sim.eccentricity = v;
            sim.rebuild_orbital_elements();
        }
        Message::ArgPerigeeChanged(v) => {
            sim.arg_perigee = v;
            sim.rebuild_orbital_elements();
        }
        Message::ToggleAscendingNode(v) => {
            sim.show_ascending_node = v;
            sim.rebuild_orbital_elements();
        }
        Message::ToggleOrbitalPlane(v) => {
            sim.show_orbital_plane = v;
            sim.rebuild_orbital_elements();
        }
        Message::ToggleInclinationArc(v) => {
            sim.show_inclination_arc = v;
            sim.rebuild_orbital_elements();
        }
        Message::TogglePerigeeApogee(v) => {
            sim.show_perigee_apogee = v;
            sim.rebuild_orbital_elements();
        }
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

fn view(sim: &OrbitElementsSimulation) -> Element<'_, Message> {
    let scene = shader(sim).width(Length::Fill).height(Length::Fill);
    let panel = sim.control_panel();

    let panel_col = container(panel)
        .width(Length::Fixed(280.0))
        .height(Length::Fill);

    row![panel_col, scene]
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn main() -> iced::Result {
    env_logger::init();

    iced::application(OrbitElementsSimulation::new, update, view)
        .subscription(|_state: &OrbitElementsSimulation| {
            iced::Subscription::batch([
                time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
                iced::event::listen().map(Message::Event),
            ])
        })
        .run()
}
