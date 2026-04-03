/// 8px-grid spacing tokens.
///
/// All layout spacing MUST use these constants.
/// No arbitrary pixel values elsewhere.

/// Smallest spacing unit — 4px (half-grid for tight internal padding).
pub const XXXS: f32 = 4.0;
/// 8px — base grid unit.
pub const XXS: f32 = 8.0;
/// 12px — compact element gaps.
pub const XS: f32 = 12.0;
/// 16px — standard gap between related items.
pub const SM: f32 = 16.0;
/// 24px — gap between groups / sections.
pub const MD: f32 = 24.0;
/// 32px — large section separation.
pub const LG: f32 = 32.0;
/// 48px — extra-large (e.g. top-level region margins).
pub const XL: f32 = 48.0;

// ---------------------------------------------------------------------------
// Semantic aliases
// ---------------------------------------------------------------------------

/// Padding inside a card / panel.
pub const PANEL_PADDING: f32 = SM;
/// Gap between controls inside a panel.
pub const CONTROL_GAP: f32 = XXS;
/// Gap between panels in the sidebar.
pub const SECTION_GAP: f32 = SM;
/// Internal padding of the sidebar.
pub const SIDEBAR_PADDING: f32 = SM;
/// Gap between toolbar buttons.
pub const TOOLBAR_GAP: f32 = XXS;
/// Spacing between a label and its input.
pub const LABEL_GAP: f32 = XXXS;

// ---------------------------------------------------------------------------
// Layout sizing
// ---------------------------------------------------------------------------

/// Default sidebar width (pixels).
pub const SIDEBAR_WIDTH: f32 = 280.0;
/// Status bar height.
pub const STATUS_BAR_HEIGHT: f32 = 32.0;
/// Panel border radius.
pub const BORDER_RADIUS: f32 = 6.0;
