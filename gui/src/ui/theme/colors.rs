/// Centralized color tokens adapted from Ant Design for a dark
/// space-simulation workbench.
///
/// Every color used in the UI MUST come from this module.
/// No raw `Color::from_rgb` calls elsewhere.
use iced::Color;

// ---------------------------------------------------------------------------
// Neutral palette (dark mode)
// ---------------------------------------------------------------------------

/// Application background — deepest layer.
pub const BG_BASE: Color = Color::from_rgb(0.08, 0.08, 0.10);
/// Sidebar / elevated surface background.
pub const BG_ELEVATED: Color = Color::from_rgb(0.11, 0.11, 0.14);
/// Card / panel surface.
pub const BG_CONTAINER: Color = Color::from_rgb(0.14, 0.14, 0.17);
/// Hover state over container.
pub const BG_CONTAINER_HOVER: Color = Color::from_rgb(0.17, 0.17, 0.21);
/// Divider / border lines.
pub const BORDER: Color = Color::from_rgb(0.22, 0.22, 0.26);
/// Subtle border for inputs.
pub const BORDER_INPUT: Color = Color::from_rgb(0.28, 0.28, 0.33);

// ---------------------------------------------------------------------------
// Text hierarchy
// ---------------------------------------------------------------------------

/// Primary text — high contrast.
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.92, 0.92, 0.95);
/// Secondary text — labels, descriptions.
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.62, 0.62, 0.68);
/// Disabled / placeholder text.
pub const TEXT_DISABLED: Color = Color::from_rgb(0.38, 0.38, 0.42);

// ---------------------------------------------------------------------------
// Brand — primary action color (blue, Ant-style)
// ---------------------------------------------------------------------------

/// Primary brand color.
pub const PRIMARY: Color = Color::from_rgb(0.22, 0.52, 0.96);
/// Hovered primary.
pub const PRIMARY_HOVER: Color = Color::from_rgb(0.30, 0.58, 1.0);
/// Active / pressed primary.
pub const PRIMARY_ACTIVE: Color = Color::from_rgb(0.16, 0.44, 0.82);
/// Muted primary for backgrounds.
pub const PRIMARY_BG: Color = Color::from_rgb(0.10, 0.18, 0.30);

// ---------------------------------------------------------------------------
// Semantic — success / warning / danger / info
// ---------------------------------------------------------------------------

pub const SUCCESS: Color = Color::from_rgb(0.20, 0.78, 0.42);
pub const WARNING: Color = Color::from_rgb(0.98, 0.74, 0.18);
pub const DANGER: Color = Color::from_rgb(0.94, 0.30, 0.26);
pub const DANGER_HOVER: Color = Color::from_rgb(1.0, 0.40, 0.36);
pub const INFO: Color = Color::from_rgb(0.22, 0.52, 0.96);

// ---------------------------------------------------------------------------
// Status indicators
// ---------------------------------------------------------------------------

/// Running simulation / online.
pub const STATUS_RUNNING: Color = SUCCESS;
/// Paused simulation.
pub const STATUS_PAUSED: Color = WARNING;
/// Error state.
pub const STATUS_ERROR: Color = DANGER;
