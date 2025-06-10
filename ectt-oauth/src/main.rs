mod cli;
mod oauth;

use clap::Parser;
use crossterm::event::{self, Event};
use ratatui::{DefaultTerminal, Frame};

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

struct ComposeFields {
    to: String,
    cc: String,
    bcc: String,
    body: String,
    focused: usize, // 0: to, 1: cc, 2: bcc, 3: body
}

struct ReadingFields {
    to: String,
    cc: String,
    bcc: String,
    body: String,
    scroll: u16,
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
            compose: ComposeFields {
                to: String::new(),
                cc: String::new(),
                bcc: String::new(),
                body: String::new(),
                focused: 0,
            },
            reading: ReadingFields {
                to: "alice@example.com".to_string(),
                cc: "bob@example.com".to_string(),
                bcc: "carol@example.com".to_string(),
                body: "This is the email body.\nIt can be very long and should wrap and scroll."
                    .to_string(),
                scroll: 0,
            },
            login_url: "https://example.com/login".to_string(),
        }
    }
}

fn run(mut terminal: DefaultTerminal) -> std::io::Result<()> {
    let mut state = ScreenState::default();
    state.table_state.select(Some(0));

    loop {
        terminal.draw(|f| match state.screen {
            Screen::Login => render_login(f, &state),
            Screen::Main => render_main(f, &mut state),
            Screen::Compose => render_compose(f, &state.compose),
            Screen::Reading => render_reading(f, &state.reading),
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match state.screen {
                    Screen::Login => {
                        // Any key goes to Main for demo
                        state.screen = Screen::Main;
                    }
                    Screen::Main => match (key.code, key.modifiers) {
                        (crossterm::event::KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                            break Ok(())
                        }
                        (crossterm::event::KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                            state.screen = Screen::Compose;
                        }
                        (crossterm::event::KeyCode::Enter, _) => {
                            state.screen = Screen::Reading;
                        }
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
                    },
                    Screen::Compose => {
                        match (key.code, key.modifiers) {
                            (crossterm::event::KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                                // Send email (demo: go back to main)
                                state.screen = Screen::Main;
                            }
                            (crossterm::event::KeyCode::Tab, _) => {
                                state.compose.focused = (state.compose.focused + 1) % 4;
                            }
                            (crossterm::event::KeyCode::BackTab, _) => {
                                state.compose.focused = (state.compose.focused + 3) % 4;
                            }
                            (crossterm::event::KeyCode::Esc, _) => {
                                state.screen = Screen::Main;
                            }
                            (crossterm::event::KeyCode::Char(c), _) => {
                                match state.compose.focused {
                                    0 => state.compose.to.push(c),
                                    1 => state.compose.cc.push(c),
                                    2 => state.compose.bcc.push(c),
                                    3 => state.compose.body.push(c),
                                    _ => {}
                                }
                            }
                            (crossterm::event::KeyCode::Backspace, _) => {
                                match state.compose.focused {
                                    0 => {
                                        state.compose.to.pop();
                                    }
                                    1 => {
                                        state.compose.cc.pop();
                                    }
                                    2 => {
                                        state.compose.bcc.pop();
                                    }
                                    3 => {
                                        state.compose.body.pop();
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                    Screen::Reading => match key.code {
                        crossterm::event::KeyCode::Esc => state.screen = Screen::Main,
                        crossterm::event::KeyCode::Down => state.reading.scroll += 1,
                        crossterm::event::KeyCode::Up => {
                            if state.reading.scroll > 0 {
                                state.reading.scroll -= 1;
                            }
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}

fn render_login(f: &mut Frame, state: &ScreenState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let block = Block::default().borders(Borders::ALL).title("Login");
    let text = format!("Open URL: {}", state.login_url);
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(paragraph, chunks[0]);
    let help = Paragraph::new("[Any key] Continue | [Ctrl+Q] Quit")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[1]);
}

fn render_main(f: &mut Frame, state: &mut ScreenState) {
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
    let rows = state
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
    f.render_stateful_widget(table, chunks[0], &mut state.table_state);
    let help = Paragraph::new("[Ctrl+Q] Quit | [Ctrl+N] Compose | [Enter] Read | [Up/Down] Move")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[1]);
}

fn render_compose(f: &mut Frame, compose: &ComposeFields) {
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
        ("To", &compose.to, compose.focused == 0),
        ("Cc", &compose.cc, compose.focused == 1),
        ("Bcc", &compose.bcc, compose.focused == 2),
        ("Body", &compose.body, compose.focused == 3),
    ];
    for (i, (label, value, focused)) in fields.iter().enumerate() {
        let block = Block::default().borders(Borders::ALL).title(*label);
        let style = if *focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let para = Paragraph::new(value.as_str()).block(block).style(style);
        f.render_widget(para, chunks[i]);
    }
    let help = Paragraph::new("[Ctrl+S] Send | [Tab] Next | [Shift+Tab] Prev | [Esc] Cancel")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[4]);
}

fn render_reading(f: &mut Frame, reading: &ReadingFields) {
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
        ("To", &reading.to),
        ("Cc", &reading.cc),
        ("Bcc", &reading.bcc),
    ];
    for (i, (label, value)) in fields.iter().enumerate() {
        let block = Block::default().borders(Borders::ALL).title(*label);
        let para = Paragraph::new(value.as_str()).block(block);
        f.render_widget(para, chunks[i]);
    }
    let body_block = Block::default().borders(Borders::ALL).title("Body");
    let para = Paragraph::new(reading.body.as_str())
        .block(body_block)
        .wrap(Wrap { trim: false })
        .scroll((reading.scroll, 0));
    f.render_widget(para, chunks[3]);
    let help =
        Paragraph::new("[Esc] Back | [Up/Down] Scroll").style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[4]);
}
