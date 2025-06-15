use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::Widget,
};

use crate::{
    imap::ParsedEmail,
    tui::{
        body::BodyWidget,
        combo::KeyCombo,
        focus::FocusStyle,
        help::{HasHelp, HelpWidget},
        line::LineWidget,
    },
    Action, Page,
};

#[derive(Debug, PartialEq, Eq)]
enum Focus {
    From,
    Cc,
    Bcc,
    Subject,
    Body,
}

impl Focus {
    fn next(&mut self, cc_is_empty: bool, bcc_is_empty: bool) {
        match (self, cc_is_empty, bcc_is_empty) {
            (self_ @ Focus::From, false, _) => {
                *self_ = Focus::Cc;
            }
            (self_ @ Focus::From, true, false) => {
                *self_ = Focus::Bcc;
            }
            (self_ @ Focus::From, true, true) => *self_ = Focus::Subject,

            (self_ @ Focus::Cc, _, true) => *self_ = Focus::Subject,
            (self_ @ Focus::Cc, _, false) => *self_ = Focus::Bcc,
            (self_ @ Focus::Bcc, _, _) => *self_ = Focus::Subject,
            (self_ @ Focus::Subject, _, _) => *self_ = Focus::Body,
            (self_ @ Focus::Body, _, _) => *self_ = Focus::From,
        }
    }

    fn previous(&mut self, cc_is_empty: bool, bcc_is_empty: bool) {
        match (self, cc_is_empty, bcc_is_empty) {
            (self_ @ Focus::From, _, _) => {
                *self_ = Focus::Body;
            }
            (self_ @ Focus::Cc, _, _) => *self_ = Focus::From,
            (self_ @ Focus::Bcc, false, _) => *self_ = Focus::From,
            (self_ @ Focus::Bcc, true, _) => *self_ = Focus::Cc,
            (self_ @ Focus::Subject, _, false) => *self_ = Focus::Bcc,
            (self_ @ Focus::Subject, false, true) => *self_ = Focus::Cc,
            (self_ @ Focus::Subject, true, true) => *self_ = Focus::From,
            (self_ @ Focus::Body, _, _) => *self_ = Focus::Subject,
        }
    }
}

pub struct ReadingWidget<'w> {
    focused: Focus,

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
            to: LineWidget::with_contents("From", vec![from]),
            cc: LineWidget::with_contents("Cc", cc),
            bcc: LineWidget::with_contents("Bcc", bcc),
            subject: LineWidget::with_contents("Subject", vec![subject]),
            body: BodyWidget::with_contents(
                body.replace("\r", "")
                    .split("\n")
                    .map(ToString::to_string)
                    .collect(),
            ),
            help: Self::help(),
            focused: Focus::From,
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
        tracing::debug!("{:?}", self.cc.as_ref());

        let cc_is_empty = self.cc.as_ref().lines().is_empty() || self.cc.as_ref().is_empty();
        let bcc_is_empty = self.bcc.as_ref().lines().is_empty() || self.cc.as_ref().is_empty();

        match (code, modifiers) {
            (crossterm::event::KeyCode::Esc, _) => return Action::GoTo(Page::Inbox),
            (crossterm::event::KeyCode::Tab, _) => {
                self.focused.next(cc_is_empty, bcc_is_empty);
                self.update_focused();
            }
            (crossterm::event::KeyCode::BackTab, _) => {
                self.focused.previous(cc_is_empty, bcc_is_empty);
                self.update_focused();
            }
            (crossterm::event::KeyCode::Char(_), _)
            | (crossterm::event::KeyCode::Backspace, _)
            | (crossterm::event::KeyCode::Delete, _) => {
                // Ignore editing inputs
                // this could be placed in the underlying component too but
                // the logic there would become more complicated, here is good enough
            }
            _ => {
                match self.focused {
                    Focus::From => self.to.input(event),
                    Focus::Cc => self.cc.input(event),
                    Focus::Bcc => self.bcc.input(event),
                    Focus::Subject => self.subject.input(event),
                    Focus::Body => self.body.as_mut().input(event),
                };
            }
        }
        Action::Tick
    }

    fn update_focused(&mut self) {
        let parts: [(Focus, &mut dyn FocusStyle); 5] = [
            (Focus::From, &mut self.to),
            (Focus::Cc, &mut self.cc),
            (Focus::Bcc, &mut self.bcc),
            (Focus::Subject, &mut self.subject),
            (Focus::Body, &mut self.body),
        ];
        for (focus, focusable) in parts.into_iter() {
            if self.focused == focus {
                focusable.focused();
            } else {
                focusable.unfocused();
            }
        }
    }
}

impl<'w> Widget for &ReadingWidget<'w> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut constraints = vec![Constraint::Length(3)];

        let layout = Layout::default().direction(Direction::Vertical);

        // TextArea.is_empty() is not very clear, it will write an empty line behind your back sometimes
        // https://github.com/rhysd/tui-textarea/issues/107
        let cc_is_empty = self.cc.as_ref().lines().is_empty() || self.cc.as_ref().is_empty();
        let bcc_is_empty = self.bcc.as_ref().lines().is_empty() || self.cc.as_ref().is_empty();

        match (cc_is_empty, bcc_is_empty) {
            (false, false) => {
                constraints.extend_from_slice(&[
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(1),
                ]);
                let chunks = layout.constraints(constraints).split(area);
                self.to.render(chunks[0], buf);
                self.cc.render(chunks[1], buf);
                self.bcc.render(chunks[2], buf);
                self.subject.render(chunks[3], buf);
                self.body.render(chunks[4], buf);
                self.help.render(chunks[5], buf);
            }
            (false, true) => {
                constraints.extend_from_slice(&[
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(1),
                ]);
                let chunks = layout.constraints(constraints).split(area);
                self.to.render(chunks[0], buf);
                self.cc.render(chunks[1], buf);
                self.subject.render(chunks[2], buf);
                self.body.render(chunks[3], buf);
                self.help.render(chunks[4], buf);
            }
            (true, false) => {
                constraints.extend_from_slice(&[
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(1),
                ]);
                let chunks = layout.constraints(constraints).split(area);
                self.to.render(chunks[0], buf);
                self.bcc.render(chunks[1], buf);
                self.subject.render(chunks[2], buf);
                self.body.render(chunks[3], buf);
                self.help.render(chunks[4], buf);
            }
            (true, true) => {
                constraints.extend_from_slice(&[
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(1),
                ]);
                let chunks = layout.constraints(constraints).split(area);
                self.to.render(chunks[0], buf);
                self.subject.render(chunks[1], buf);
                self.body.render(chunks[2], buf);
                self.help.render(chunks[3], buf);
            }
        }
    }
}
