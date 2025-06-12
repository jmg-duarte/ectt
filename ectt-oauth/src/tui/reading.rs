use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::Widget,
};

use crate::{
    tui::{
        address::AddressWidget,
        body::BodyWidget,
        combo::KeyCombo,
        focus::FocusStyle,
        help::{HasHelp, HelpWidget},
    },
    Action,
};
pub struct ReadingWidget<'w> {
    focused: usize, // 0: to, 1: cc, 2: bcc, 3: body

    to: AddressWidget<'w>,
    cc: AddressWidget<'w>,
    bcc: AddressWidget<'w>,
    body: BodyWidget<'w>,
    help: HelpWidget<'w>,
}

impl<'w> Default for ReadingWidget<'w> {
    fn default() -> Self {
        Self {
            to: {
                let mut widget = AddressWidget::with_contents("To", "jose@kagi.com".to_string());
                // ensure its focused on the first render
                widget.focused();
                widget
            },
            cc: AddressWidget::with_contents("Cc", "jose@kagi.com".to_string()),
            bcc: AddressWidget::with_contents("Bcc", "jose@kagi.com".to_string()),
            body: BodyWidget::new(),
            help: Self::help(),
            focused: Default::default(),
        }
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
            (crossterm::event::KeyCode::Esc, _) => Action::Back,
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
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(area);

        self.to.render(chunks[0], buf);
        self.cc.render(chunks[1], buf);
        self.bcc.render(chunks[2], buf);
        self.body.render(chunks[3], buf);
        self.help.render(chunks[4], buf);
    }
}
