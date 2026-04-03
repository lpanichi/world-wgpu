use iced::Element;
use iced::widget::{column, pick_list, row, text};

use crate::ui::components::button::{ButtonVariant, action_button, icon_button, icon_text_button};
use crate::ui::components::panel::{collapsible_panel, panel};
use crate::ui::theme::{colors, icons, spacing, typography};
use crate::ui::widgets::control_group::{control_group, labeled_input};

// Re-export layout primitives so consumers can compose the top-level view.
pub use crate::ui::components::layout::workbench_layout;
pub use crate::ui::components::sidebar::sidebar;
pub use crate::ui::components::status_bar::status_bar;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Which top-level tab is active in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Builder,
    Manager,
    Kpi,
}

/// Which builder sub-form is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuilderForm {
    None,
    Orbit,
    Station,
    Satellite,
    RectSurface,
}

/// UI-specific state that lives alongside (not inside) the simulation.
pub struct MainScreenState {
    pub sidebar_tab: SidebarTab,
    pub builder_form: BuilderForm,
    pub advanced_orbit_expanded: bool,
    pub advanced_station_expanded: bool,
}

impl Default for MainScreenState {
    fn default() -> Self {
        Self {
            sidebar_tab: SidebarTab::Builder,
            builder_form: BuilderForm::None,
            advanced_orbit_expanded: false,
            advanced_station_expanded: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Messages (sidebar-scoped)
// ---------------------------------------------------------------------------

/// Messages produced by the main screen's sidebar controls.
///
/// The outer application maps these into its own `Message` enum.
#[derive(Debug, Clone)]
pub enum SidebarMessage {
    SwitchTab(SidebarTab),
    ShowBuilderForm(BuilderForm),
    ToggleAdvancedOrbit,
    ToggleAdvancedStation,
}

// ---------------------------------------------------------------------------
// View helpers
// ---------------------------------------------------------------------------

/// Render the tab selector row (Builder | Manager | KPIs).
pub fn tab_bar<'a, M: Clone + 'a>(
    active: SidebarTab,
    on_switch: impl Fn(SidebarTab) -> M + 'a,
) -> Element<'a, M> {
    let variant_for = |tab: SidebarTab| {
        if tab == active {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Default
        }
    };

    row![
        action_button(
            "Builder",
            variant_for(SidebarTab::Builder),
            Some(on_switch(SidebarTab::Builder)),
        ),
        action_button(
            "Manager",
            variant_for(SidebarTab::Manager),
            Some(on_switch(SidebarTab::Manager)),
        ),
        action_button(
            "KPIs",
            variant_for(SidebarTab::Kpi),
            Some(on_switch(SidebarTab::Kpi)),
        ),
    ]
    .spacing(spacing::TOOLBAR_GAP)
    .into()
}

/// Render the "Builder" sub-toolbar (Orbit | Station | Satellite | Rect).
pub fn builder_toolbar<'a, M: Clone + 'a>(
    active: BuilderForm,
    on_select: impl Fn(BuilderForm) -> M + 'a,
) -> Element<'a, M> {
    let variant_for = |form: BuilderForm| {
        if form == active {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Text
        }
    };

    row![
        action_button(
            "Station",
            variant_for(BuilderForm::Station),
            Some(on_select(BuilderForm::Station)),
        ),
        action_button(
            "Orbit",
            variant_for(BuilderForm::Orbit),
            Some(on_select(BuilderForm::Orbit)),
        ),
        action_button(
            "Satellite",
            variant_for(BuilderForm::Satellite),
            Some(on_select(BuilderForm::Satellite)),
        ),
        action_button(
            "Rect",
            variant_for(BuilderForm::RectSurface),
            Some(on_select(BuilderForm::RectSurface)),
        ),
    ]
    .spacing(spacing::TOOLBAR_GAP)
    .into()
}

// ---------------------------------------------------------------------------
// Panel view factories
// ---------------------------------------------------------------------------

/// Builder panel: orbit creation form.
///
/// Only one primary button ("Create Orbit") per panel.
pub fn orbit_builder_panel<'a, M: Clone + 'a>(
    orbit_name: &str,
    altitude: &str,
    inclination: &str,
    raan: &str,
    arg_perigee: &str,
    on_orbit_name: impl Fn(String) -> M + 'a,
    on_altitude: impl Fn(String) -> M + 'a,
    on_inclination: impl Fn(String) -> M + 'a,
    on_raan: impl Fn(String) -> M + 'a,
    on_arg_perigee: impl Fn(String) -> M + 'a,
    on_create: M,
    advanced_expanded: bool,
    on_toggle_advanced: M,
) -> Element<'a, M> {
    let basic = control_group(
        "Basic Parameters",
        vec![
            labeled_input(
                "Name".into(),
                "e.g. Orbit 1",
                orbit_name.to_string(),
                on_orbit_name,
            ),
            labeled_input(
                "Altitude (km)".into(),
                "e.g. 500",
                altitude.to_string(),
                on_altitude,
            ),
            labeled_input(
                "Inclination (°)".into(),
                "e.g. 20",
                inclination.to_string(),
                on_inclination,
            ),
        ],
    );

    let advanced_content = column![
        labeled_input("RAAN (°)".into(), "e.g. 30", raan.to_string(), on_raan,),
        labeled_input(
            "Arg. Perigee (°)".into(),
            "e.g. 0",
            arg_perigee.to_string(),
            on_arg_perigee,
        ),
    ]
    .spacing(spacing::LABEL_GAP);

    let advanced = collapsible_panel(
        "Advanced",
        advanced_expanded,
        on_toggle_advanced,
        advanced_content,
    );

    panel(
        Some("New Orbit"),
        column![
            basic,
            advanced,
            action_button("Create Orbit", ButtonVariant::Primary, Some(on_create)),
        ]
        .spacing(spacing::CONTROL_GAP),
    )
}

/// Builder panel: ground station creation form.
pub fn station_builder_panel<'a, M: Clone + 'a>(
    name: &str,
    lat: &str,
    lon: &str,
    on_name: impl Fn(String) -> M + 'a,
    on_lat: impl Fn(String) -> M + 'a,
    on_lon: impl Fn(String) -> M + 'a,
    on_create: M,
) -> Element<'a, M> {
    panel(
        Some("New Ground Station"),
        column![
            control_group(
                "Station",
                vec![
                    labeled_input("Name".into(), "Station name", name.to_string(), on_name),
                    labeled_input("Latitude (°)".into(), "-90 to 90", lat.to_string(), on_lat,),
                    labeled_input(
                        "Longitude (°)".into(),
                        "-180 to 180",
                        lon.to_string(),
                        on_lon,
                    ),
                ],
            ),
            action_button("Create Station", ButtonVariant::Primary, Some(on_create)),
        ]
        .spacing(spacing::CONTROL_GAP),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrbitOption {
    index: usize,
    label: String,
}

impl std::fmt::Display for OrbitOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

/// Builder panel: satellite creation form.
pub fn satellite_builder_panel<'a, M: Clone + 'a>(
    name: &str,
    selected_orbit_index: Option<usize>,
    orbit_options: Vec<(usize, String)>,
    on_name: impl Fn(String) -> M + 'a,
    on_orbit_selected: impl Fn(usize) -> M + 'a,
    on_create: M,
) -> Element<'a, M> {
    let orbit_options: Vec<OrbitOption> = orbit_options
        .into_iter()
        .map(|(index, name)| OrbitOption {
            index,
            label: format!("{} (#{})", name, index),
        })
        .collect();
    let selected_orbit = selected_orbit_index.and_then(|idx| {
        orbit_options
            .iter()
            .find(|option| option.index == idx)
            .cloned()
    });
    let orbit_label = if !orbit_options.is_empty() {
        "Orbit".to_string()
    } else {
        "Orbit (no orbits available)".to_string()
    };
    let create_action = if !orbit_options.is_empty() {
        Some(on_create)
    } else {
        None
    };

    panel(
        Some("New Satellite"),
        column![
            control_group(
                "Satellite",
                vec![
                    labeled_input("Name".into(), "Sat-1", name.to_string(), on_name),
                    column![
                        text(orbit_label)
                            .size(typography::SIZE_SM)
                            .color(colors::TEXT_SECONDARY),
                        pick_list(orbit_options, selected_orbit, move |option| {
                            on_orbit_selected(option.index)
                        }),
                    ]
                    .spacing(spacing::LABEL_GAP)
                    .into(),
                ],
            ),
            action_button("Create Satellite", ButtonVariant::Primary, create_action,),
        ]
        .spacing(spacing::CONTROL_GAP),
    )
}

/// Builder panel: rectangular surface.
pub fn rect_surface_builder_panel<'a, M: Clone + 'a>(
    min_lat: &str,
    max_lat: &str,
    min_lon: &str,
    max_lon: &str,
    on_min_lat: impl Fn(String) -> M + 'a,
    on_max_lat: impl Fn(String) -> M + 'a,
    on_min_lon: impl Fn(String) -> M + 'a,
    on_max_lon: impl Fn(String) -> M + 'a,
    on_create: M,
) -> Element<'a, M> {
    panel(
        Some("Rectangular Surface"),
        column![
            control_group(
                "Latitude bounds",
                vec![
                    labeled_input("Min lat (°)".into(), "-90", min_lat.to_string(), on_min_lat,),
                    labeled_input("Max lat (°)".into(), "90", max_lat.to_string(), on_max_lat,),
                ],
            ),
            control_group(
                "Longitude bounds",
                vec![
                    labeled_input(
                        "Min lon (°)".into(),
                        "-180",
                        min_lon.to_string(),
                        on_min_lon,
                    ),
                    labeled_input("Max lon (°)".into(), "180", max_lon.to_string(), on_max_lon,),
                ],
            ),
            action_button("Create Surface", ButtonVariant::Primary, Some(on_create)),
        ]
        .spacing(spacing::CONTROL_GAP),
    )
}

// ---------------------------------------------------------------------------
// Simulation controls panel
// ---------------------------------------------------------------------------

/// The persistent simulation controls shown at the top of the sidebar,
/// above the mode tabs.
pub fn sim_controls_panel<'a, M: Clone + 'a>(
    paused: bool,
    time_scale: f32,
    precession: bool,
    following: bool,
    on_pause: M,
    on_reset_time: M,
    on_toggle_frame: M,
    on_slower: M,
    on_faster: M,
    on_reset_speed: M,
    on_toggle_precession: M,
    on_toggle_follow: M,
) -> Element<'a, M> {
    let pause_icon = if paused { icons::PLAY } else { icons::PAUSE };
    let pause_label = if paused { "Resume" } else { "Pause" };
    let follow_label = if following {
        "Free Camera"
    } else {
        "Follow…"
    };
    let speed_label = format!("{:.0}x", time_scale);

    panel(
        Some("Simulation"),
        column![
            row![
                icon_text_button(
                    pause_icon,
                    pause_label,
                    ButtonVariant::Default,
                    Some(on_pause)
                ),
                icon_text_button(
                    icons::BACKWARD_STEP,
                    "Reset",
                    ButtonVariant::Default,
                    Some(on_reset_time)
                ),
                icon_text_button(
                    icons::GLOBE,
                    "Frame",
                    ButtonVariant::Default,
                    Some(on_toggle_frame)
                ),
            ]
            .spacing(spacing::TOOLBAR_GAP),
            row![
                icon_text_button(
                    icons::BACKWARD,
                    "Slower",
                    ButtonVariant::Default,
                    Some(on_slower)
                ),
                icon_text_button(
                    icons::FORWARD,
                    "Faster",
                    ButtonVariant::Default,
                    Some(on_faster)
                ),
                action_button(speed_label, ButtonVariant::Text, Some(on_reset_speed)),
            ]
            .spacing(spacing::TOOLBAR_GAP),
            row![
                icon_text_button(
                    if precession {
                        icons::CHECK
                    } else {
                        icons::XMARK
                    },
                    "Precession",
                    ButtonVariant::Default,
                    Some(on_toggle_precession),
                ),
                icon_text_button(
                    if following {
                        icons::VIDEO_SLASH
                    } else {
                        icons::VIDEO
                    },
                    follow_label,
                    ButtonVariant::Default,
                    Some(on_toggle_follow),
                ),
            ]
            .spacing(spacing::TOOLBAR_GAP),
        ]
        .spacing(spacing::CONTROL_GAP),
    )
}

// ---------------------------------------------------------------------------
// Manager list items
// ---------------------------------------------------------------------------

/// A single orbit entry in the resource manager.
pub fn orbit_manager_item<'a, M: Clone + 'a>(
    index: usize,
    altitude_km: f32,
    inclination_deg: f32,
    satellite_count: usize,
    show_orbit: bool,
    show_fov: bool,
    fill_fov: bool,
    fov_half_angle: f32,
    on_delete: M,
    on_toggle_orbit: M,
    on_toggle_fov: M,
    on_toggle_fill: M,
    on_fov_angle: impl Fn(String) -> M + 'a,
    on_inc_edit: impl Fn(String) -> M + 'a,
    on_raan_edit: impl Fn(String) -> M + 'a,
    raan_deg: f32,
) -> Element<'a, M> {
    let header = row![
        text(format!(
            "#{} alt={:.0}km inc={:.1}° sats={}",
            index, altitude_km, inclination_deg, satellite_count,
        ))
        .size(typography::SIZE_SM)
        .color(colors::TEXT_PRIMARY),
        icon_button(icons::TRASH, ButtonVariant::Danger, Some(on_delete)),
    ]
    .spacing(spacing::TOOLBAR_GAP);

    let toggles = row![
        icon_text_button(
            if show_orbit {
                icons::EYE
            } else {
                icons::EYE_SLASH
            },
            "Orbit",
            ButtonVariant::Default,
            Some(on_toggle_orbit),
        ),
        icon_text_button(
            if show_fov { icons::CHECK } else { icons::XMARK },
            "FOV",
            ButtonVariant::Default,
            Some(on_toggle_fov),
        ),
        icon_text_button(
            if fill_fov { icons::CHECK } else { icons::XMARK },
            "Fill",
            ButtonVariant::Default,
            Some(on_toggle_fill),
        ),
    ]
    .spacing(spacing::XXXS);

    let params = row![
        labeled_input(
            "FOV (°)".into(),
            "deg",
            format!("{:.1}", fov_half_angle),
            on_fov_angle,
        ),
        labeled_input(
            "Inc (°)".into(),
            "deg",
            format!("{:.1}", inclination_deg),
            on_inc_edit,
        ),
        labeled_input(
            "RAAN (°)".into(),
            "deg",
            format!("{:.1}", raan_deg),
            on_raan_edit,
        ),
    ]
    .spacing(spacing::TOOLBAR_GAP);

    panel(
        None,
        column![header, toggles, params].spacing(spacing::CONTROL_GAP),
    )
}

/// A single ground station entry in the resource manager.
pub fn station_manager_item<'a, M: Clone + 'a>(
    name: &str,
    lat: f32,
    lon: f32,
    show_cone: bool,
    min_elevation_deg: f32,
    on_delete: M,
    on_toggle_cone: M,
    on_min_elev: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    let header = row![
        text(format!("{} ({:.1}°, {:.1}°)", name, lat, lon))
            .size(typography::SIZE_SM)
            .color(colors::TEXT_PRIMARY),
        icon_button(icons::TRASH, ButtonVariant::Danger, Some(on_delete)),
    ]
    .spacing(spacing::TOOLBAR_GAP);

    let controls = row![
        icon_text_button(
            if show_cone {
                icons::CHECK
            } else {
                icons::XMARK
            },
            "Cone",
            ButtonVariant::Default,
            Some(on_toggle_cone),
        ),
        labeled_input(
            "Min elev (°)".into(),
            "deg",
            format!("{:.1}", min_elevation_deg),
            on_min_elev,
        ),
    ]
    .spacing(spacing::TOOLBAR_GAP);

    panel(
        None,
        column![header, controls].spacing(spacing::CONTROL_GAP),
    )
}

// ---------------------------------------------------------------------------
// KPI panel
// ---------------------------------------------------------------------------

/// The KPI dashboard panel with station-satellite distance plot.
pub fn kpi_panel<'a, M: Clone + 'a>(
    station_idx: &str,
    orbit_idx: &str,
    sat_idx: &str,
    current_distance: Option<f32>,
    sparkline: Option<&str>,
    min_distance: Option<f32>,
    max_distance: Option<f32>,
    orbit_count: usize,
    station_count: usize,
    satellite_count: usize,
    on_station_idx: impl Fn(String) -> M + 'a,
    on_orbit_idx: impl Fn(String) -> M + 'a,
    on_sat_idx: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    let mut content = column![].spacing(spacing::CONTROL_GAP);

    content = content.push(
        row![
            labeled_input(
                "Station #".into(),
                "0",
                station_idx.to_string(),
                on_station_idx,
            ),
            labeled_input("Orbit #".into(), "0", orbit_idx.to_string(), on_orbit_idx,),
            labeled_input("Sat #".into(), "0", sat_idx.to_string(), on_sat_idx),
        ]
        .spacing(spacing::TOOLBAR_GAP),
    );

    if let Some(dist) = current_distance {
        content = content.push(
            text(format!("Distance: {:.1} km", dist))
                .size(typography::SIZE_BASE)
                .color(colors::TEXT_PRIMARY),
        );
    }

    if let (Some(min), Some(max)) = (min_distance, max_distance) {
        content = content.push(
            text(format!("Min: {:.0} km  Max: {:.0} km", min, max))
                .size(typography::SIZE_XS)
                .color(colors::TEXT_SECONDARY),
        );
    }

    if let Some(spark) = sparkline {
        content = content.push(
            text(spark.to_string())
                .size(typography::SIZE_LG)
                .color(colors::PRIMARY),
        );
    }

    content = content.push(panel(
        Some("Summary"),
        text(format!(
            "Orbits: {}  Stations: {}  Satellites: {}",
            orbit_count, station_count, satellite_count,
        ))
        .size(typography::SIZE_SM)
        .color(colors::TEXT_SECONDARY),
    ));

    panel(Some("KPI Dashboard"), content)
}

// ---------------------------------------------------------------------------
// Error display
// ---------------------------------------------------------------------------

/// An inline error banner (visible only when `message` is non-empty).
pub fn error_banner<'a, M: 'a>(message: &str) -> Element<'a, M> {
    if message.is_empty() {
        return column![].into();
    }

    iced::widget::container(
        text(message.to_string())
            .size(typography::SIZE_SM)
            .color(colors::STATUS_ERROR),
    )
    .padding(spacing::XXXS)
    .width(iced::Length::Fill)
    .style(|_theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(
            0.94, 0.30, 0.26, 0.12,
        ))),
        border: iced::Border {
            color: colors::DANGER,
            width: 1.0,
            radius: spacing::BORDER_RADIUS.into(),
        },
        ..iced::widget::container::Style::default()
    })
    .into()
}
