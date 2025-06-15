use std::iter::empty;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, StatefulWidget, Table, TableState, Widget},
};

use crate::{
    imap::ParsedEmail,
    tui::{
        combo::KeyCombo,
        help::{HasHelp, HelpWidget},
    },
    Action, Page,
};

pub struct InboxState {
    pub inbox: Vec<ParsedEmail>,
    pub table: TableState,
}

impl InboxState {
    pub fn new() -> Self {
        Self {
            inbox: vec![],
            table: TableState::default().with_selected(0),
        }
    }
}

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
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Fill(3),
        ];
        let table = Table::new(empty::<Row>(), widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Posts"))
            .row_highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

        Self {
            table,
            help: Self::help(),
        }
    }

    pub fn handle_event(&mut self, event: Event, state: &mut InboxState) -> Action {
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
        state: &mut InboxState,
    ) -> Action {
        match (code, modifiers) {
            (crossterm::event::KeyCode::Char('w'), KeyModifiers::CONTROL) => Action::Quit,
            (crossterm::event::KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                Action::GoTo(Page::Compose)
            }
            (crossterm::event::KeyCode::Down, _) => {
                state.table.select_next();
                Action::Tick
            }
            (crossterm::event::KeyCode::Up, _) => {
                state.table.select_previous();
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
            (KeyCombo::new().with_code(KeyCode::Down), "Load more"),
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

impl StatefulWidget for &mut InboxWidget<'_> {
    type State = InboxState;

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

        let table = std::mem::take(&mut self.table);
        let table = table.rows(state.inbox.iter().map(|parsed| {
            Row::new(vec![
                Cell::from(parsed.date.clone().to_string()),
                Cell::from(parsed.from.clone()),
                Cell::from(parsed.subject.clone()),
            ])
        }));
        let _ = std::mem::replace(&mut self.table, table);

        StatefulWidget::render(&self.table, chunks[0], buf, &mut state.table);
        Widget::render(&self.help, chunks[1], buf);
    }
}
