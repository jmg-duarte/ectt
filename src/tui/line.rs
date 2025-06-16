use crossterm::event::KeyEvent;
use ratatui::{
    style::{Color, Style, Stylize},
    widgets::{block::Title, Block, Borders, Widget},
};
use tui_textarea::TextArea;

use crate::tui::focus::FocusStyle;

pub struct LineWidget<'w> {
    textarea: TextArea<'w>,
}

impl<'w> LineWidget<'w> {
    pub fn new<T: Into<Title<'w>>>(title: T) -> Self {
        Self {
            textarea: {
                let mut textarea = TextArea::default();
                textarea.set_cursor_line_style(Style::default());
                // textarea.set_placeholder_text("john.doe@kagi.com");
                textarea.set_block(Block::default().borders(Borders::ALL).title(title));
                textarea
            },
        }
    }

    pub fn with_contents<T: Into<Title<'w>>>(title: T, contents: Vec<String>) -> Self {
        Self {
            textarea: {
                let mut textarea = TextArea::new(contents);
                textarea.set_cursor_line_style(Style::default());
                // textarea.set_placeholder_text("john.doe@kagi.com");
                textarea.set_block(Block::default().borders(Borders::ALL).title(title));
                textarea
            },
        }
    }

    pub fn input(&mut self, event @ KeyEvent { code, .. }: KeyEvent) -> bool {
        match code {
            crossterm::event::KeyCode::Enter => {
                // ignore enter because we don't support newlines here
                false
            }
            _ => self.textarea.input(event),
        }
    }
}

impl<'w> Widget for &LineWidget<'w> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        self.textarea.render(area, buf);
    }
}

impl<'w> AsRef<TextArea<'w>> for LineWidget<'w> {
    fn as_ref(&self) -> &TextArea<'w> {
        &self.textarea
    }
}

impl<'w> AsMut<TextArea<'w>> for LineWidget<'w> {
    fn as_mut(&mut self) -> &mut TextArea<'w> {
        &mut self.textarea
    }
}

impl<'w> FocusStyle for LineWidget<'w> {
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
