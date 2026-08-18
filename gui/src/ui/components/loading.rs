use iced::widget::{column, container, progress_bar, text};
use iced::{Background, Border, Element, Length};

use crate::ui::theme::{colors, spacing, typography};

/// Optional full-window loading screen shown while render assets preload.
///
/// See [`crate::gpu::assets`] for the corresponding preload API.
pub fn loading_screen<'a, M: 'a>(
    message: impl Into<String>,
    progress: Option<f32>,
) -> Element<'a, M> {
    let message_text = text(message.into())
        .size(typography::SIZE_BASE)
        .color(colors::TEXT_PRIMARY);

    let mut content = column![message_text].spacing(spacing::SM);

    if let Some(value) = progress {
        let bar = container(progress_bar(0.0..=1.0, value.clamp(0.0, 1.0)))
            .width(Length::Fixed(320.0))
            .height(Length::Fixed(spacing::XS));
        content = content.push(bar);
    }

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(colors::BG_BASE)),
            border: Border::default(),
            ..container::Style::default()
        })
        .into()
}