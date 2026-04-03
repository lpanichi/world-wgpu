use iced::widget::{Column, container, scrollable};
use iced::{Background, Border, Element, Length};

use crate::ui::theme::{colors, spacing};

/// Sidebar container — a fixed-width scrollable area for controls.
///
/// Wraps its children in a vertically scrollable column with
/// consistent padding and section spacing.
pub fn sidebar<'a, M: 'a>(children: Vec<Element<'a, M>>) -> Element<'a, M> {
    let mut col = Column::new().spacing(spacing::SECTION_GAP);
    for child in children {
        col = col.push(child);
    }

    let scrollable_content = scrollable(
        container(col)
            .padding(spacing::SIDEBAR_PADDING)
            .width(Length::Fill),
    )
    .height(Length::Fill);

    container(scrollable_content)
        .width(Length::Fixed(spacing::SIDEBAR_WIDTH))
        .height(Length::Fill)
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
