use iced::widget::{column, container, row};
use iced::{Background, Border, Element, Length};

use crate::ui::theme::colors;

/// Top-level workbench layout:
///
/// ```text
/// ┌──────────────────────────────────┐
/// │           status_bar             │
/// ├──────────┬───────────────────────┤
/// │ sidebar  │       viewport        │
/// │  (≤30%)  │        (≥70%)         │
/// └──────────┴───────────────────────┘
/// ```
///
/// The viewport element receives `Length::Fill` so it dominates the layout.
pub fn workbench_layout<'a, M: 'a>(
    status_bar: Element<'a, M>,
    sidebar: Element<'a, M>,
    viewport: Element<'a, M>,
) -> Element<'a, M> {
    let body = row![sidebar, viewport]
        .spacing(0)
        .height(Length::Fill);

    container(
        column![status_bar, body]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_theme| container::Style {
        background: Some(Background::Color(colors::BG_BASE)),
        border: Border::default(),
        ..container::Style::default()
    })
    .into()
}
