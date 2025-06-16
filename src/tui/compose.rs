use std::str::FromStr;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use lettre::{address::AddressError, Address};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::Widget,
};

use crate::{
    smtp::PartialMessage,
    tui::{
        body::BodyWidget,
        combo::KeyCombo,
        focus::FocusStyle,
        help::{HasHelp, HelpWidget},
        line::LineWidget,
        Action, Page,
    },
};

pub struct ComposeWidget<'w> {
    focused: usize, // 0: to, 1: cc, 2: bcc, 3: body

    to: LineWidget<'w>,
    cc: LineWidget<'w>,
    bcc: LineWidget<'w>,
    subject: LineWidget<'w>,
    body: BodyWidget<'w>,
    help: HelpWidget<'w>,
}

impl<'w> Default for ComposeWidget<'w> {
    fn default() -> Self {
        Self {
            to: LineWidget::new("To"),
            cc: LineWidget::new("Cc"),
            bcc: LineWidget::new("Bcc"),
            subject: LineWidget::new("Subject"),
            body: BodyWidget::new(),
            help: Self::help(),
            focused: Default::default(),
        }
    }
}

impl<'w> HasHelp for ComposeWidget<'w> {
    fn help<'h>() -> super::help::HelpWidget<'h> {
        HelpWidget::new(vec![
            (
                KeyCombo::new()
                    .with_code(KeyCode::Char('S'))
                    .with_modifier(KeyModifiers::CONTROL),
                "Send",
            ),
            (KeyCombo::new().with_code(KeyCode::Tab), "Next"),
            (
                KeyCombo::new()
                    .with_code(KeyCode::Tab)
                    .with_modifier(KeyModifiers::SHIFT),
                "Prev",
            ),
            (KeyCombo::new().with_code(KeyCode::Esc), "Cancel"),
        ])
    }
}

fn parse_addresses(s: &str) -> Result<Vec<Address>, AddressError> {
    if s.is_empty() {
        return Ok(vec![]);
    }
    s.split(",")
        .map(&str::trim)
        .map(FromStr::from_str)
        .collect::<Result<Vec<Address>, AddressError>>()
}

impl<'w> ComposeWidget<'w> {
    pub fn get_partial_message(&self) -> Result<PartialMessage, crate::Error> {
        let to = self
            .to
            .as_ref()
            .lines()
            .get(0)
            .map(|s| s.parse::<Address>())
            .transpose()?;

        let cc = self
            .cc
            .as_ref()
            .lines()
            .get(0)
            .map(|cc| parse_addresses(cc))
            .transpose()?
            .unwrap_or_default();

        let bcc = self
            .bcc
            .as_ref()
            .lines()
            .get(0)
            .map(|bcc| parse_addresses(bcc))
            .transpose()?
            .unwrap_or_default();

        let subject = self.subject.as_ref().lines().get(0).cloned();

        let body = self.body.as_ref().lines().get(0).cloned();

        Ok(PartialMessage {
            to,
            cc,
            bcc,
            subject,
            body,
        })
    }

    pub fn handle_event(&mut self, event: Event) -> Action {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event),
            _ => Action::Tick,
        }
    }

    fn handle_key_event(
        &mut self,
        event @ KeyEvent {
            code, modifiers, ..
        }: KeyEvent,
    ) -> Action {
        match (code, modifiers) {
            (crossterm::event::KeyCode::Esc, _) => Action::GoTo(Page::Inbox),
            (crossterm::event::KeyCode::Tab, _) => {
                self.focused = (self.focused + 1) % 5;
                self.update_focused();
                Action::Tick
            }
            (crossterm::event::KeyCode::BackTab, _) => {
                self.focused = (self.focused + 4) % 5;
                self.update_focused();
                Action::Tick
            }
            _ => {
                match self.focused {
                    0 => self.to.input(event),
                    1 => self.cc.input(event),
                    2 => self.bcc.input(event),
                    3 => self.subject.input(event),
                    4 => self.body.as_mut().input(event),
                    _ => unreachable!(),
                };
                Action::Tick
            }
        }
    }

    fn update_focused(&mut self) {
        let parts: [&mut dyn FocusStyle; 5] = [
            &mut self.to,
            &mut self.cc,
            &mut self.bcc,
            &mut self.subject,
            &mut self.body,
        ];
        for (idx, focusable) in parts.into_iter().enumerate() {
            if idx == self.focused {
                focusable.focused();
            } else {
                focusable.unfocused();
            }
        }
    }
}

impl<'w> Widget for &ComposeWidget<'w> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(area);

        self.to.render(chunks[0], buf);
        self.cc.render(chunks[1], buf);
        self.bcc.render(chunks[2], buf);
        self.subject.render(chunks[3], buf);
        self.body.render(chunks[4], buf);
        self.help.render(chunks[5], buf);
    }
}
