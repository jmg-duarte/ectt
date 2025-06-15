use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::tui::{
    combo::KeyCombo,
    help::{HasHelp, HelpWidget},
    Action, Page,
};

pub struct LoginWidget<'w> {
    url: String,

    help: HelpWidget<'w>,
}

impl<'w> LoginWidget<'w> {
    pub fn new(url: String) -> LoginWidget<'w> {
        Self {
            url,
            help: Self::help(),
        }
    }

    pub fn handle_event(&mut self, event: Event) -> Action {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event),
            _ => Action::Tick,
        }
    }

    fn handle_key_event(
        &mut self,
        KeyEvent {
            code, modifiers, ..
        }: KeyEvent,
    ) -> Action {
        match (code, modifiers) {
            (KeyCode::Esc, _) => Action::Quit,
            (KeyCode::Enter, _) => Action::GoTo(Page::Inbox),
            _ => {
                // TODO: solve this later, we need to advance the status
                Action::Tick
            }
        }
    }
}

impl<'w> HasHelp for LoginWidget<'w> {
    fn help<'h>() -> super::help::HelpWidget<'h> {
        HelpWidget::new(vec![(
            KeyCombo::new().with_code(crossterm::event::KeyCode::Esc),
            "Exit",
        )])
    }
}

impl<'w> Widget for &LoginWidget<'w> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        let block = Block::default().borders(Borders::ALL).title("Login");

        let text = format!("Open URL: {}", self.url);
        let paragraph = Paragraph::new(text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        paragraph.render(chunks[0], buf);

        self.help.render(chunks[1], buf);
    }
}
