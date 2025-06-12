use ratatui::{
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Widget},
};
use tui_textarea::TextArea;

use crate::tui::focus::FocusStyle;

pub struct BodyWidget<'w> {
    textarea: TextArea<'w>,
}

impl<'w> BodyWidget<'w> {
    pub fn new() -> Self {
        Self {
            textarea: {
                let mut textarea = TextArea::default();
                textarea.set_cursor_line_style(Style::default());
                textarea.set_placeholder_text("john.doe@kagi.com");
                textarea.set_block(Block::default().borders(Borders::ALL).title("Body"));
                textarea
            },
        }
    }

    pub fn with_contents(contents: String) -> Self {
        Self {
            textarea: {
                let mut textarea = TextArea::new(vec![contents]);
                textarea.set_cursor_line_style(Style::default());
                // textarea.set_placeholder_text("john.doe@kagi.com");
                textarea.set_block(Block::default().borders(Borders::ALL).title("Body"));
                textarea
            },
        }
    }
}

impl<'w> Widget for &BodyWidget<'w> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        self.textarea.render(area, buf);
    }
}

impl<'w> AsRef<TextArea<'w>> for BodyWidget<'w> {
    fn as_ref(&self) -> &TextArea<'w> {
        &self.textarea
    }
}

impl<'w> AsMut<TextArea<'w>> for BodyWidget<'w> {
    fn as_mut(&mut self) -> &mut TextArea<'w> {
        &mut self.textarea
    }
}

impl<'w> FocusStyle for BodyWidget<'w> {
    fn unfocused(&mut self) {
        let Some(block) = self.textarea.block() else {
            return;
        };
        self.textarea.set_block(block.clone().fg(Color::default()));
    }

    fn focused(&mut self) {
        let Some(block) = self.textarea.block() else {
            return;
        };
        self.textarea.set_block(block.clone().fg(Color::Blue));
    }
}
