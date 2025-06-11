use itertools::Itertools;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

use crate::tui::combo::KeyCombo;

pub struct HelpWidget<'w> {
    /// Vector of available keys and a string for their actions.
    // NOTE: There is definitely better ways of representing this
    actions: Vec<(KeyCombo, &'w str)>,
}

impl<'w> HelpWidget<'w> {
    pub fn new(actions: Vec<(KeyCombo, &'w str)>) -> Self {
        Self { actions }
    }
}

impl<'w> Widget for HelpWidget<'w> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let text = self
            .actions
            .into_iter()
            .map(|(combo, action)| format!("{combo}: {action}"))
            .join(" | ");

        Paragraph::new(text)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray))
            .render(area, buf);
    }
}

pub trait HasHelp {
    fn help<'w>() -> HelpWidget<'w>;
}
