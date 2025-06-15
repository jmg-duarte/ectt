use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::Widget,
};

use crate::{
    imap::ParsedEmail,
    tui::{
        address::LineWidget,
        body::BodyWidget,
        combo::KeyCombo,
        focus::FocusStyle,
        help::{HasHelp, HelpWidget},
    },
    Action, Page,
};
pub struct ReadingWidget<'w> {
    focused: usize, // 0: to, 1: cc, 2: bcc, 3: body

    to: LineWidget<'w>,
    cc: LineWidget<'w>,
    bcc: LineWidget<'w>,
    subject: LineWidget<'w>,
    body: BodyWidget<'w>,
    help: HelpWidget<'w>,
}

impl ReadingWidget<'_> {
    pub fn new(
        from: String,
        cc: Vec<String>,
        bcc: Vec<String>,
        subject: String,
        body: String,
    ) -> Self {
        Self {
            to: {
                let mut widget = LineWidget::with_contents("From", from);
                // ensure its focused on the first render
                widget.focused();
                widget
            },
            cc: LineWidget::with_contents("Cc", cc.join(", ")),
            bcc: LineWidget::with_contents("Bcc", bcc.join(", ")),
            subject: LineWidget::with_contents("Subject", subject),
            body: BodyWidget::with_contents(body),
            help: Self::help(),
            focused: Default::default(),
        }
    }
}

impl From<ParsedEmail> for ReadingWidget<'_> {
    fn from(value: ParsedEmail) -> Self {
        Self::new(value.from, value.cc, value.bcc, value.subject, value.body)
    }
}

impl<'w> HasHelp for ReadingWidget<'w> {
    fn help<'h>() -> super::help::HelpWidget<'h> {
        HelpWidget::new(vec![
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

impl<'w> ReadingWidget<'w> {
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
                self.focused = (self.focused + 1) % 4;
                self.update_focused();
                Action::Tick
            }
            (crossterm::event::KeyCode::BackTab, _) => {
                self.focused = (self.focused + 3) % 4;
                self.update_focused();
                Action::Tick
            }
            (crossterm::event::KeyCode::Char(_), _)
            | (crossterm::event::KeyCode::Backspace, _)
            | (crossterm::event::KeyCode::Delete, _) => {
                // Ignore editing inputs
                // this could be placed in the underlying component too but
                // the logic there would become more complicated, here is good enough
                Action::Tick
            }
            _ => {
                match self.focused {
                    0 => self.to.input(event),
                    1 => self.cc.input(event),
                    2 => self.bcc.input(event),
                    3 => self.body.as_mut().input(event),
                    _ => unreachable!(),
                };
                Action::Tick
            }
        }
    }

    fn update_focused(&mut self) {
        let parts: [&mut dyn FocusStyle; 4] =
            [&mut self.to, &mut self.cc, &mut self.bcc, &mut self.body];
        for (idx, focusable) in parts.into_iter().enumerate() {
            if idx == self.focused {
                focusable.focused();
            } else {
                focusable.unfocused();
            }
        }
    }
}

impl<'w> Widget for &ReadingWidget<'w> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
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
