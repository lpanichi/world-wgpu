use iced::widget::{container, row, text};
use iced::{Background, Border, Element, Length};

use crate::ui::theme::{colors, spacing, typography};

/// Persistent status bar shown at the top of the workbench.
///
/// Displays simulation state, date, and contextual info.
/// Mirrors Ant Design's "page header" / global feedback bar.
pub fn status_bar<'a, M: 'a>(
    left: impl Into<String>,
    center: impl Into<String>,
    right: impl Into<String>,
) -> Element<'a, M> {
    let left_text = text(left.into())
        .size(typography::SIZE_SM)
        .color(colors::TEXT_PRIMARY);

    let center_text = text(center.into())
        .size(typography::SIZE_SM)
        .color(colors::TEXT_SECONDARY);

    let right_text = text(right.into())
        .size(typography::SIZE_SM)
        .color(colors::TEXT_SECONDARY);

    container(
        row![
            container(left_text)
                .width(Length::Fill)
                .align_left(Length::Fill),
            container(center_text)
                .width(Length::Shrink)
                .center_x(Length::Shrink),
            container(right_text)
                .width(Length::Fill)
                .align_right(Length::Fill),
        ]
        .spacing(spacing::SM)
        .width(Length::Fill),
    )
    .padding([spacing::XXXS, spacing::SM])
    .height(Length::Fixed(spacing::STATUS_BAR_HEIGHT))
    .width(Length::Fill)
    .style(|_theme| container::Style {
        background: Some(Background::Color(colors::BG_ELEVATED)),
        border: Border {
            color: colors::BORDER,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}
