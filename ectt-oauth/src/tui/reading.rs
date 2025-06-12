use std::{default, hint::unreachable_unchecked};

use color_eyre::owo_colors::OwoColorize;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    widgets::{block::Title, Block, Borders, Paragraph, Widget},
    Frame,
};
use tui_textarea::TextArea;

use crate::{
    tui::{
        address::{self, AddressWidget},
        body::BodyWidget,
        combo::KeyCombo,
        focus::FocusStyle,
        help::{HasHelp, HelpWidget},
    },
    Screen, ScreenState,
};

#[derive(Debug, Default)]
pub struct ReadingState {
    to: String,
    cc: String,
    bcc: String,
    body: Vec<String>,
}

pub struct ReadingWidget<'w> {
    state: ReadingState, // TODO: handle ctrl+s to send the email

    focused: usize, // 0: to, 1: cc, 2: bcc, 3: body

    to: AddressWidget<'w>,
    cc: AddressWidget<'w>,
    bcc: AddressWidget<'w>,
    body: BodyWidget<'w>,
}

impl<'w> Default for ReadingWidget<'w> {
    fn default() -> Self {
        Self {
            state: Default::default(),
            to: {
                let mut widget = AddressWidget::with_contents("To", "jose@kagi.com".to_string());
                // ensure its focused on the first render
                widget.focused();
                widget
            },
            cc: AddressWidget::with_contents("Cc", "jose@kagi.com".to_string()),
            bcc: AddressWidget::with_contents("Bcc", "jose@kagi.com".to_string()),
            body: BodyWidget::new(),
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
    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event),
            _ => {}
        }
    }

    fn handle_key_event(
        &mut self,
        event @ KeyEvent {
            code, modifiers, ..
        }: KeyEvent,
    ) {
        match (code, modifiers) {
            (crossterm::event::KeyCode::Char('s'), event::KeyModifiers::CONTROL) => {
                todo!("state.screen = Screen::Main");
            }
            (crossterm::event::KeyCode::Tab, _) => {
                self.focused = (self.focused + 1) % 4;
                self.update_focused();
            }
            (crossterm::event::KeyCode::BackTab, _) => {
                self.focused = (self.focused + 3) % 4;
                self.update_focused();
            }
            (crossterm::event::KeyCode::Esc, _) => todo!("state.screen = Screen::Main"),
            (crossterm::event::KeyCode::Char(_), _)
            | (crossterm::event::KeyCode::Backspace, _)
            | (crossterm::event::KeyCode::Delete, _) => {
                // Ignore editing inputs
                // this could be placed in the underlying component too but
                // the logic there would become more complicated, here is good enough
            }
            _ => {
                match self.focused {
                    0 => self.to.input(event),
                    1 => self.cc.input(event),
                    2 => self.bcc.input(event),
                    3 => self.body.as_mut().input(event),
                    _ => unreachable!(),
                };
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

    pub fn render_reading(&self, f: &mut Frame) {
        let area = f.area();
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

        f.render_widget(&self.to, chunks[0]);
        f.render_widget(&self.cc, chunks[1]);
        f.render_widget(&self.bcc, chunks[2]);
        f.render_widget(&self.body, chunks[3]);
        f.render_widget(Self::help(), chunks[4]);
    }
}
