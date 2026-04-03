use iced::widget::{column, text, Column};
use iced::Element;

use crate::ui::theme::{colors, spacing, typography};

/// A reusable control group: a labeled section containing a vertical
/// list of controls. Used inside panels for logical grouping
/// (e.g. "Orbital Parameters", "Station Settings").
///
/// Analogous to Ant Design's `<Form.Item>` or a fieldset.
pub fn control_group<'a, M: 'a>(
    label: &'a str,
    children: Vec<Element<'a, M>>,
) -> Element<'a, M> {
    let mut col: Column<'a, M> = column![
        text(label)
            .size(typography::SIZE_SM)
            .color(colors::TEXT_SECONDARY),
    ]
    .spacing(spacing::LABEL_GAP);

    for child in children {
        col = col.push(child);
    }

    col.spacing(spacing::LABEL_GAP).into()
}

/// Labeled text input — a convenience wrapper that pairs a label
/// with an `iced::widget::text_input`.
///
/// Accepts owned `String` for label and value so callers can pass
/// `format!(...)` directly without lifetime issues.
pub fn labeled_input<'a, M: Clone + 'a>(
    label: String,
    placeholder: &'a str,
    value: String,
    on_input: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    column![
        text(label)
            .size(typography::SIZE_SM)
            .color(colors::TEXT_SECONDARY),
        iced::widget::text_input(placeholder, &value)
            .on_input(on_input)
            .size(typography::SIZE_BASE)
            .padding(spacing::XXXS),
    ]
    .spacing(spacing::LABEL_GAP)
    .into()
}
