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
    widget::{button, column, container, pane_grid, row, scrollable, shader, text},
};
use log::info;

mod program;
use crate::program::SelectedObject;

#[derive(Clone)]
enum Message {
    KeyboardEvent(keyboard::Event),
    Event(iced::event::Event),
    Tick,
    OnObjectSelected(SelectedObject, Option<f32>),
    PaneClicked(pane_grid::Pane),
    PaneDragged(pane_grid::DragEvent),
    PaneResized(pane_grid::ResizeEvent),
    AddOrbit,
    RemoveOrbit,
    AddSatellite(usize),
    RemoveSatellite(usize),
    ToggleOrbit(usize),
    ToggleSatelliteMode,
    ToggleFrame,
    TogglePause,
    IncreaseTimeScale,
    DecreaseTimeScale,
    ResetTimeScale,
    AddStation,
    RemoveStation,
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
}

impl Textured {
    fn update(&mut self, message: Message) {
        match message {
            Message::KeyboardEvent(event) => self.handle_keyboard_event(event),
            Message::Event(event) => self.handle_event(event),
            Message::Tick => {
                // Tick drives redraw and optionally simulation progression via elapsed_time.
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
            Message::AddOrbit => {
                self.modify_model(|model| {
                    let idx = model.orbits.len();
                    let altitude_km = 500.0 + idx as f32 * 150.0;
                    let semi_major_axis = EARTH_RADIUS_KM + altitude_km;
                    let period_seconds = Orbit::circular_period_seconds(semi_major_axis);
                    model.orbits.push(
                        Orbit::builder(semi_major_axis, period_seconds)
                            .inclination(10.0 + idx as f32 * 5.0)
                            .raan(15.0 + idx as f32 * 10.0)
                            .show_orbit(true)
                            .add_satellite(
                                Satellite::builder(format!("Sat-{}", idx * 2 + 1))
                                    .phase_offset(0.0)
                                    .build(),
                            )
                            .build(),
                    );
                });
                self.status_message =
                    format!("Added orbit. Total {}", self.program.model.orbits.len());
            }
            Message::RemoveOrbit => {
                self.modify_model(|model| {
                    if !model.orbits.is_empty() {
                        model.orbits.pop();
                    }
                });
                self.status_message =
                    format!("Removed orbit. Total {}", self.program.model.orbits.len());
            }
            Message::ToggleOrbit(index) => {
                self.modify_model(|model| {
                    if let Some(orbit) = model.orbits.get_mut(index) {
                        orbit.show_orbit = !orbit.show_orbit;
                    }
                });
            }
            Message::AddSatellite(index) => {
                self.modify_model(|model| {
                    if let Some(orbit) = model.orbits.get_mut(index) {
                        let sat_id = orbit.satellites.len() + 1;
                        orbit.satellites.push(
                            Satellite::builder(format!("{}-{}", index, sat_id))
                                .phase_offset(0.0)
                                .build(),
                        );
                    }
                });
            }
            Message::RemoveSatellite(index) => {
                self.modify_model(|model| {
                    if let Some(orbit) = model.orbits.get_mut(index) {
                        orbit.satellites.pop();
                    }
                });
            }
            Message::ToggleSatelliteMode => {
                self.program.satellite_mode = match self.program.satellite_mode {
                    SatelliteRenderMode::Cube => SatelliteRenderMode::Dot,
                    SatelliteRenderMode::Dot => SatelliteRenderMode::Cube,
                };
            }
            Message::ToggleFrame => {
                self.program.frame_mode = match self.program.frame_mode {
                    crate::program::FrameMode::Eci => crate::program::FrameMode::Ecef,
                    crate::program::FrameMode::Ecef => crate::program::FrameMode::Eci,
                };
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
            Message::AddStation => {
                self.modify_model(|model| {
                    let id = model.ground_stations.len();
                    model.ground_stations.push(GroundStation::new(
                        format!("Station {}", id + 1),
                        -30.0 + (id as f32 * 20.0).rem_euclid(180.0),
                        -180.0 + (id as f32 * 45.0).rem_euclid(360.0),
                    ));
                });
            }
            Message::RemoveStation => {
                self.modify_model(|model| {
                    model.ground_stations.pop();
                });
            }
        }
    }

    fn modify_model(&mut self, mut f: impl FnMut(&mut Simulation)) {
        let mut simulation = (*self.program.model).clone();
        f(&mut simulation);
        self.program.model = std::sync::Arc::new(simulation);
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

                            if let Some((origin, direction)) =
                                self.program.world_ray_from_cursor(local_pos, (w, h))
                            {
                                let (selected, hit_distance) =
                                    self.program.pick_object(origin, direction);
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
        let meta = &self.program.model;

        let mut content = column![
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
            row![
                button("Add Orbit").on_press(Message::AddOrbit),
                button("Remove Orbit").on_press(Message::RemoveOrbit),
            ]
            .spacing(8),
            row![
                button("Add Station").on_press(Message::AddStation),
                button("Remove Station").on_press(Message::RemoveStation),
            ]
            .spacing(8),
            button("Toggle Satellite Mode").on_press(Message::ToggleSatelliteMode),
        ]
        .spacing(10)
        .padding(10);

        for (i, orbit) in meta.orbits.iter().enumerate() {
            let orbit_row = row![
                text(format!(
                    "Orbit {} (a={:.1}, incl={:.1}):",
                    i, orbit.semi_major_axis, orbit.inclination_deg
                )),
                button(if orbit.show_orbit { "Hide" } else { "Show" })
                    .on_press(Message::ToggleOrbit(i)),
                button("+ Sat").on_press(Message::AddSatellite(i)),
                button("- Sat").on_press(Message::RemoveSatellite(i)),
                text(format!("{} sats", orbit.satellites.len())),
            ]
            .spacing(8);

            content = content.push(orbit_row);
        }

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

        let model = Simulation::builder()
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
            .add_ground_station(GroundStation::new("Station A", 30.0, 10.0))
            .add_ground_station(GroundStation::new("Station B", -20.0, 100.0))
            .build();

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
                model: std::sync::Arc::new(model),
                camera,
                start_time: std::time::Instant::now(),
                satellite_mode: SatelliteRenderMode::Dot,
                frame_mode: crate::program::FrameMode::Eci,
                paused: false,
                paused_elapsed: 0.0,
                time_scale: 120.0,
                pick_radius_scale: 2.0,
            },
            panes,
            focus: Some(root_pane),
            status_message: "No selection".to_string(),
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
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

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
