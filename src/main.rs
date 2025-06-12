mod cli;
mod oauth;
mod tui;

use std::env::current_dir;
use std::fs::OpenOptions;

use clap::Parser;
use crossterm::event::{self};
use dirs::config_dir;
use ratatui::DefaultTerminal;

use crate::tui::compose::ComposeWidget;
use crate::tui::inbox::InboxWidget;
use crate::tui::login::LoginWidget;
use crate::tui::reading::ReadingWidget;
use crate::{cli::App, oauth::execute_authentication_flow};
use ratatui::widgets::TableState;

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

            let mut config_path = match config_dir() {
                Some(dir) => dir,
                None => {
                    tracing::warn!(
                    "Failed to find a configuration directory, defaulting to the current one..."
                );
                    current_dir()?
                }
            };
            config_path.push("ectt.json");

            let result = if let Err(err) = OpenOptions::new().open(config_path) {
                tracing::warn!(
                    "Failed to read configuration file (error: {err}), re-configuring..."
                );
                let terminal = ratatui::init();
                run(terminal)
                // TODO: launch login
            } else {
                let terminal = ratatui::init();
                run(terminal)
            };
            ratatui::restore();
            Ok(result?)
        }
    }
}

enum Screen<'w> {
    Login(LoginWidget<'w>),
    Inbox(InboxWidget<'w>),
    Compose(ComposeWidget<'w>),
    Reading(ReadingWidget<'w>),
}

pub enum Page {
    Login,
    Inbox,
    Compose,
    Reading,
}

impl<'w> From<Page> for Screen<'w> {
    fn from(value: Page) -> Self {
        match value {
            Page::Login => Screen::Login(LoginWidget::new("http://localhost:8000".to_string())),
            Page::Inbox => Screen::Inbox(InboxWidget::new()),
            Page::Compose => Screen::Compose(ComposeWidget::default()),
            Page::Reading => Screen::Reading(ReadingWidget::default()),
        }
    }
}

pub enum Action {
    Quit,
    Tick,
    GoTo(Page),
}

struct ScreenState {
    table_state: TableState,
}

impl Default for ScreenState {
    fn default() -> Self {
        Self {
            table_state: TableState::default(),
        }
    }
}
fn run(mut terminal: DefaultTerminal) -> std::io::Result<()> {
    let mut screen = Screen::from(Page::Login);

    let mut state = ScreenState::default();
    state.table_state.select(Some(0));

    loop {
        terminal.draw(|f| match &screen {
            Screen::Login(widget) => f.render_widget(&*widget, f.area()),
            Screen::Inbox(widget) => {
                f.render_stateful_widget(&*widget, f.area(), &mut state.table_state)
            }
            Screen::Compose(widget) => f.render_widget(&*widget, f.area()),
            Screen::Reading(widget) => f.render_widget(&*widget, f.area()),
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            let event = event::read()?;

            let action = match &mut screen {
                Screen::Login(widget) => widget.handle_event(event),
                Screen::Inbox(widget) => widget.handle_event(event, &mut state.table_state),
                Screen::Compose(widget) => widget.handle_event(event),
                Screen::Reading(widget) => widget.handle_event(event),
            };

            match action {
                Action::Quit => break Ok(()),
                Action::Tick => continue,
                Action::GoTo(new_screen) => screen = Screen::from(new_screen),
            };
        }
    }
}
