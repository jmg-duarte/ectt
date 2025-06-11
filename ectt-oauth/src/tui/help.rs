use crossterm::event::KeyCode;
use itertools::Itertools;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

pub struct HelpWidget<'w> {
    /// Vector of available keys and a string for their actions.
    // NOTE: There is definitely better ways of representing this
    actions: &'w [(&'w [KeyCode], &'w str)],
}

impl<'w> HelpWidget<'w> {
    pub fn new(actions: &'w [(&'w [KeyCode], &'w str)]) -> Self {
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
            .map(|(key_codes, action)| {
                let mut text = key_codes.into_iter().map(|k| k.to_string()).join("/");
                text.push_str(" ");
                text.push_str(&action);
                text
            })
            .join(" | ");

        Paragraph::new(text)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray))
            .render(area, buf);
    }
}

pub trait HasHelp {
    fn help<'w>() -> HelpWidget<'w>;
}
