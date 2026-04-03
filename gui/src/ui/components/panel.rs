use iced::widget::{Column, column, container, text};
use iced::{Background, Border, Element, Length};
use iced_font_awesome::fa_icon_solid;

use crate::ui::theme::{colors, icons, spacing, typography};

/// A card / panel container with optional title.
///
/// Maps to Ant Design's `<Card>` component: grouped content with
/// clear visual boundary, title, and consistent internal spacing.
pub fn panel<'a, M: 'a>(
    title: Option<&'a str>,
    content: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    let mut col: Column<'a, M> = column![].spacing(spacing::CONTROL_GAP);

    if let Some(t) = title {
        col = col.push(
            text(t)
                .size(typography::SIZE_LG)
                .color(colors::TEXT_PRIMARY),
        );
    }

    col = col.push(content);

    container(col)
        .padding(spacing::PANEL_PADDING)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(colors::BG_CONTAINER)),
            border: Border {
                color: colors::BORDER,
                width: 1.0,
                radius: spacing::BORDER_RADIUS.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// A collapsible panel for progressive disclosure of advanced options.
///
/// When `expanded` is `false`, only the title row with a toggle is shown.
pub fn collapsible_panel<'a, M: Clone + 'a>(
    title: &'a str,
    expanded: bool,
    on_toggle: M,
    content: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    use iced::widget::{button, row};

    let chevron_icon = if expanded {
        icons::CHEVRON_DOWN
    } else {
        icons::CHEVRON_RIGHT
    };
    let header = row![
        button(
            row![
                fa_icon_solid(chevron_icon).size(typography::SIZE_SM),
                text(title)
                    .size(typography::SIZE_LG)
                    .color(colors::TEXT_PRIMARY),
            ]
            .spacing(spacing::XXXS)
            .align_y(iced::Alignment::Center),
        )
        .on_press(on_toggle)
        .style(|_theme, _status| {
            iced::widget::button::Style {
                background: Some(Background::Color(iced::Color::TRANSPARENT)),
                text_color: colors::TEXT_PRIMARY,
                border: Border::default(),
                ..iced::widget::button::Style::default()
            }
        })
        .padding(0),
    ];

    let mut col: Column<'a, M> = column![header].spacing(spacing::CONTROL_GAP);

    if expanded {
        col = col.push(content);
    }

    container(col)
        .padding(spacing::PANEL_PADDING)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(colors::BG_CONTAINER)),
            border: Border {
                color: colors::BORDER,
                width: 1.0,
                radius: spacing::BORDER_RADIUS.into(),
            },
            ..container::Style::default()
        })
        .into()
}
