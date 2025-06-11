mod cli;
mod oauth;
mod tui;

use clap::Parser;
use crossterm::event::{self, Event, KeyEvent};
use ratatui::{DefaultTerminal, Frame};

use crate::tui::compose::{handle_compose, ComposeFields};
use crate::{cli::App, oauth::execute_authentication_flow};
use crossterm::event::KeyModifiers;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap};

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

struct ScreenState {
    screen: Screen,
    table_state: TableState,
    items: Vec<[&'static str; 3]>,
    compose: ComposeFields,
    reading: ReadingFields,
    login_url: String,
}

impl Default for ScreenState {
    fn default() -> Self {
        Self {
            screen: Screen::Login,
            table_state: TableState::default(),
            items: vec![
                ["2024-06-01", "Alice", "First Post"],
                ["2024-06-02", "Bob", "Second Post"],
                ["2024-06-03", "Carol", "Third Post"],
            ],
            compose: ComposeFields::default(),
            reading: ReadingFields {
                from: "alice@example.com".to_string(),
                cc: vec!["bob@example.com".to_string()],
                bcc: vec!["carol@example.com".to_string()],
                body: "This is the email body.\nIt can be very long and should wrap and scroll."
                    .to_string(),
                scroll: 0,
            },
            login_url: "https://example.com/login".to_string(),
        }
    }
}

impl ScreenState {
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

fn handle_reading(state: &mut ScreenState, event: Event) {
    if let Event::Key(KeyEvent { code, .. }) = event {
        match code {
            crossterm::event::KeyCode::Esc => state.screen = Screen::Main,
            crossterm::event::KeyCode::Down => state.reading.scroll += 1,
            crossterm::event::KeyCode::Up => {
                if state.reading.scroll > 0 {
                    state.reading.scroll -= 1;
                }
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
                Screen::Compose => handle_compose(&mut state, event),
                Screen::Reading => handle_reading(&mut state, event),
            }
        }
    }
}

struct ReadingFields {
    from: String,
    cc: Vec<String>,
    bcc: Vec<String>,
    body: String,
    scroll: u16,
}

impl ReadingFields {
    fn render_reading(&self, f: &mut Frame) {
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
        let fields = [
            ("To", &self.from),
            ("Cc", &self.cc.join(", ")),
            ("Bcc", &self.bcc.join(", ")),
        ];
        for (i, (label, value)) in fields.iter().enumerate() {
            let block = Block::default().borders(Borders::ALL).title(*label);
            let para = Paragraph::new(value.as_str()).block(block);
            f.render_widget(para, chunks[i]);
        }
        let body_block = Block::default().borders(Borders::ALL).title("Body");
        let para = Paragraph::new(self.body.as_str())
            .block(body_block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));
        f.render_widget(para, chunks[3]);
        let help = Paragraph::new("[Esc] Back | [Up/Down] Scroll")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, chunks[4]);
    }
}
