use std::iter::empty;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, StatefulWidget, Table, TableState, Widget},
};

use crate::{
    tui::{
        combo::KeyCombo,
        help::{HasHelp, HelpWidget},
    },
    Action, Page,
};

pub struct InboxWidget<'w> {
    table: Table<'w>,
    help: HelpWidget<'w>,
}

impl<'w> InboxWidget<'w> {
    pub fn new() -> Self {
        let header = Row::new(
            [
                Cell::from("Date"),
                Cell::from("Author"),
                Cell::from("Title"),
            ]
            .into_iter(),
        );
        let widths = &[
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Min(20),
        ];
        let table = Table::new(empty::<Row>(), widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Posts"))
            .row_highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol(">> ");

        Self {
            table,
            help: Self::help(),
        }
    }

    pub fn handle_event(
        &mut self,
        event: Event,
        state: &mut <&InboxWidget<'_> as StatefulWidget>::State,
    ) -> Action {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event, state),
            _ => Action::Tick,
        }
    }

    fn handle_key_event(
        &mut self,
        KeyEvent {
            code, modifiers, ..
        }: KeyEvent,
        state: &mut <&InboxWidget<'_> as StatefulWidget>::State,
    ) -> Action {
        match (code, modifiers) {
            (crossterm::event::KeyCode::Char('w'), KeyModifiers::CONTROL) => Action::Quit,
            (crossterm::event::KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                Action::GoTo(Page::Compose)
            }
            (crossterm::event::KeyCode::Enter, _) => {
                // TODO: read the email from the "provider"
                Action::GoTo(Page::Reading)
            }
            (crossterm::event::KeyCode::Down, _) => {
                state.select_next();
                Action::Tick
            }
            (crossterm::event::KeyCode::Up, _) => {
                state.select_previous();
                Action::Tick
            }
            _ => Action::Tick,
        }
    }
}

impl<'w> HasHelp for InboxWidget<'w> {
    fn help<'h>() -> HelpWidget<'h> {
        HelpWidget::new(vec![
            (KeyCombo::new().with_code(KeyCode::Enter), "Read email"),
            (
                KeyCombo::new()
                    .with_code(KeyCode::Char('n'))
                    .with_modifier(KeyModifiers::CONTROL),
                "New email",
            ),
            (
                KeyCombo::new()
                    .with_code(KeyCode::Char('w'))
                    .with_modifier(KeyModifiers::CONTROL),
                "Quit",
            ),
        ])
    }
}

impl StatefulWidget for &InboxWidget<'_> {
    type State = TableState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        StatefulWidget::render(&self.table, chunks[0], buf, state);
        Widget::render(&self.help, chunks[1], buf);
    }
}
