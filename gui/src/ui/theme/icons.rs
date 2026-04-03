use iced::Element;
use iced_font_awesome::fa_icon_solid;

use super::typography;

// ---------------------------------------------------------------------------
// Semantic icon names (Font Awesome 6 solid glyph names)
// ---------------------------------------------------------------------------

pub const PLAY: &str = "play";
pub const PAUSE: &str = "pause";
pub const BACKWARD_STEP: &str = "backward-step";
pub const ROTATE: &str = "rotate";
pub const TRASH: &str = "trash";
pub const CAMERA: &str = "camera";
pub const CHEVRON_RIGHT: &str = "chevron-right";
pub const CHEVRON_DOWN: &str = "chevron-down";
pub const CHEVRON_LEFT: &str = "chevron-left";
pub const CHECK: &str = "check";
pub const XMARK: &str = "xmark";
pub const GAUGE_HIGH: &str = "gauge-high";
pub const BACKWARD: &str = "backward";
pub const FORWARD: &str = "forward";
pub const GLOBE: &str = "globe";
pub const EYE: &str = "eye";
pub const EYE_SLASH: &str = "eye-slash";
pub const FILL_DRIP: &str = "fill-drip";
pub const SATELLITE: &str = "satellite";
pub const TOWER_BROADCAST: &str = "tower-broadcast";
pub const VIDEO: &str = "video";
pub const VIDEO_SLASH: &str = "video-slash";

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Create a Font Awesome solid icon element at the standard UI size.
pub fn icon<'a, M: 'a>(name: &str) -> Element<'a, M> {
    fa_icon_solid(name)
        .size(typography::SIZE_BASE)
        .into()
}

/// Create a Font Awesome solid icon element at a custom size.
pub fn icon_sized<'a, M: 'a>(name: &str, size: f32) -> Element<'a, M> {
    fa_icon_solid(name)
        .size(size)
        .into()
}
