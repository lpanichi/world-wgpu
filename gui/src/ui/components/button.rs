use iced::widget::{button, row, text};
use iced::{Background, Border, Element, Theme, widget};
use iced_font_awesome::fa_icon_solid;

use crate::ui::theme::{colors, spacing, typography};

/// Button variant following Ant Design's button hierarchy.
///
/// **Rule**: at most ONE `Primary` button per panel / group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// Main call-to-action — filled brand color.
    Primary,
    /// Secondary actions — outlined neutral.
    #[default]
    Default,
    /// Destructive / irreversible actions — red.
    Danger,
    /// Minimal footprint — text only.
    Text,
}

/// Styled button builder that produces an Iced `Element`.
pub fn action_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    variant: ButtonVariant,
    on_press: Option<M>,
) -> Element<'a, M> {
    let label_string: String = label.into();
    let label_color = match variant {
        ButtonVariant::Primary => colors::TEXT_PRIMARY,
        ButtonVariant::Danger => colors::DANGER,
        ButtonVariant::Default => colors::TEXT_PRIMARY,
        ButtonVariant::Text => colors::TEXT_SECONDARY,
    };

    let btn = button(
        text(label_string)
            .size(typography::SIZE_BASE)
            .color(label_color),
    )
    .style(move |theme: &Theme, status| button_style(theme, status, variant, label_color))
    .padding([spacing::XXXS, spacing::XS]);

    match on_press {
        Some(msg) => btn.on_press(msg).into(),
        None => btn.into(),
    }
}

/// Button with a Font Awesome icon and text label.
pub fn icon_text_button<'a, M: Clone + 'a>(
    icon_name: &str,
    label: impl Into<String>,
    variant: ButtonVariant,
    on_press: Option<M>,
) -> Element<'a, M> {
    let label_string: String = label.into();
    let label_color = match variant {
        ButtonVariant::Primary => colors::TEXT_PRIMARY,
        ButtonVariant::Danger => colors::DANGER,
        ButtonVariant::Default => colors::TEXT_PRIMARY,
        ButtonVariant::Text => colors::TEXT_SECONDARY,
    };

    let content = row![
        fa_icon_solid(icon_name).size(typography::SIZE_SM),
        text(label_string)
            .size(typography::SIZE_BASE)
            .color(label_color),
    ]
    .spacing(spacing::XXXS)
    .align_y(iced::Alignment::Center);

    let btn = button(content)
        .style(move |theme: &Theme, status| button_style(theme, status, variant, label_color))
        .padding([spacing::XXXS, spacing::XS]);

    match on_press {
        Some(msg) => btn.on_press(msg).into(),
        None => btn.into(),
    }
}

/// Icon-only button (compact, no text label).
pub fn icon_button<'a, M: Clone + 'a>(
    icon_name: &str,
    variant: ButtonVariant,
    on_press: Option<M>,
) -> Element<'a, M> {
    let label_color = match variant {
        ButtonVariant::Primary => colors::TEXT_PRIMARY,
        ButtonVariant::Danger => colors::DANGER,
        ButtonVariant::Default => colors::TEXT_PRIMARY,
        ButtonVariant::Text => colors::TEXT_SECONDARY,
    };

    let btn = button(fa_icon_solid(icon_name).size(typography::SIZE_BASE))
        .style(move |theme: &Theme, status| button_style(theme, status, variant, label_color))
        .padding([spacing::XXXS, spacing::XXS]);

    match on_press {
        Some(msg) => btn.on_press(msg).into(),
        None => btn.into(),
    }
}

/// Shared style logic for all button variants.
fn button_style(
    _theme: &Theme,
    status: widget::button::Status,
    variant: ButtonVariant,
    label_color: iced::Color,
) -> widget::button::Style {
    let (bg, border_color) = match variant {
        ButtonVariant::Primary => match status {
            widget::button::Status::Hovered => (colors::PRIMARY_HOVER, colors::PRIMARY_HOVER),
            widget::button::Status::Pressed => (colors::PRIMARY_ACTIVE, colors::PRIMARY_ACTIVE),
            widget::button::Status::Disabled => (colors::BG_CONTAINER, colors::BORDER),
            _ => (colors::PRIMARY, colors::PRIMARY),
        },
        ButtonVariant::Danger => match status {
            widget::button::Status::Hovered => (colors::BG_CONTAINER_HOVER, colors::DANGER_HOVER),
            widget::button::Status::Disabled => (colors::BG_CONTAINER, colors::BORDER),
            _ => (colors::BG_CONTAINER, colors::DANGER),
        },
        ButtonVariant::Default => match status {
            widget::button::Status::Hovered => (colors::BG_CONTAINER_HOVER, colors::PRIMARY_HOVER),
            widget::button::Status::Disabled => (colors::BG_CONTAINER, colors::BORDER),
            _ => (colors::BG_CONTAINER, colors::BORDER_INPUT),
        },
        ButtonVariant::Text => match status {
            widget::button::Status::Hovered => {
                (colors::BG_CONTAINER_HOVER, iced::Color::TRANSPARENT)
            }
            _ => (iced::Color::TRANSPARENT, iced::Color::TRANSPARENT),
        },
    };

    widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: label_color,
        border: Border {
            color: border_color,
            width: if matches!(variant, ButtonVariant::Text) {
                0.0
            } else {
                1.0
            },
            radius: spacing::BORDER_RADIUS.into(),
        },
        ..widget::button::Style::default()
    }
}
