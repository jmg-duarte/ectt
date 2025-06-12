mod cli;
mod oauth;
mod tui;

use clap::Parser;
use crossterm::event::{self, Event, KeyEvent};
use ratatui::{DefaultTerminal, Frame};

use crate::tui::compose::ComposeWidget;
use crate::tui::reading::ReadingWidget;
use crate::{cli::App, oauth::execute_authentication_flow};
use crossterm::event::KeyModifiers;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = App::parse();

    match app.command {
        cli::Command::Login { provider } => {
            let (client, scopes) = provider.into();
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build the runtime")
                .block_on(execute_authentication_flow(client, scopes))
        }
        cli::Command::Run {} => {
            color_eyre::install().unwrap();
            let terminal = ratatui::init();
            let result = run(terminal);
            ratatui::restore();
            Ok(result?)
        }
    }
}

enum Screen {
    Login,   // Unused for now
    Main,    // The main table screen
    Compose, // Compose an email
    Reading, // Read an email
}

struct ScreenState<'w> {
    screen: Screen,
    table_state: TableState,
    items: Vec<[&'static str; 3]>,
    compose: ComposeWidget<'w>,
    reading: ReadingWidget<'w>,
    login_url: String,
}

impl<'w> Default for ScreenState<'w> {
    fn default() -> Self {
        Self {
            screen: Screen::Login,
            table_state: TableState::default(),
            items: vec![
                ["2024-06-01", "Alice", "First Post"],
                ["2024-06-02", "Bob", "Second Post"],
                ["2024-06-03", "Carol", "Third Post"],
            ],
            compose: ComposeWidget::default(),
            reading: ReadingWidget::default(),
            login_url: "https://example.com/login".to_string(),
        }
    }
}

impl<'w> ScreenState<'w> {
    fn render_login(&self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        let block = Block::default().borders(Borders::ALL).title("Login");
        let text = format!("Open URL: {}", self.login_url);
        let paragraph = Paragraph::new(text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, chunks[0]);
        let help = Paragraph::new("[Any key] Continue | [Ctrl+W] Quit")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, chunks[1]);
    }

    fn render_main(&mut self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        let header = Row::new(vec![
            Cell::from("Date"),
            Cell::from("Author"),
            Cell::from("Title"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD));
        let rows = self
            .items
            .iter()
            .map(|item| Row::new(item.iter().map(|c| Cell::from(*c))));
        let table = Table::new(
            rows,
            &[
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Min(20),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Posts"))
        .row_highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
        .highlight_symbol(">> ");
        f.render_stateful_widget(table, chunks[0], &mut self.table_state);
        let help =
            Paragraph::new("[Ctrl+W] Quit | [Ctrl+N] Compose | [Enter] Read | [Up/Down] Move")
                .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, chunks[1]);
    }
}

fn handle_login(state: &mut ScreenState, event: Event) {
    if let Event::Key(_) = event {
        state.screen = Screen::Main
    }
}

fn handle_main(state: &mut ScreenState, event: Event) {
    if let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = event
    {
        match (code, modifiers) {
            (crossterm::event::KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                todo!("send signal to stop the run loop")
            }
            (crossterm::event::KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                state.screen = Screen::Compose
            }
            (crossterm::event::KeyCode::Enter, _) => state.screen = Screen::Reading,
            (crossterm::event::KeyCode::Down, _) => {
                let i = match state.table_state.selected() {
                    Some(i) => {
                        if i >= state.items.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                state.table_state.select(Some(i));
            }
            (crossterm::event::KeyCode::Up, _) => {
                let i = match state.table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            state.items.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                state.table_state.select(Some(i));
            }
            _ => {}
        }
    }
}

fn run(mut terminal: DefaultTerminal) -> std::io::Result<()> {
    let mut state = ScreenState::default();
    state.table_state.select(Some(0));

    loop {
        terminal.draw(|f| match state.screen {
            Screen::Login => state.render_login(f),
            Screen::Main => state.render_main(f),
            Screen::Compose => state.compose.render_compose(f),
            Screen::Reading => state.reading.render_reading(f),
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            let event = event::read()?;

            match state.screen {
                Screen::Login => handle_login(&mut state, event),
                Screen::Main => handle_main(&mut state, event),
                Screen::Compose => state.compose.handle_event(event),
                Screen::Reading => state.reading.handle_event(event),
            }
        }
    }
}
