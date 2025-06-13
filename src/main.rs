mod cli;
mod config;
mod imap;
mod oauth;
mod tui;

use std::env::current_dir;
use std::fs::OpenOptions;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender, TryRecvError};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use dirs::config_dir;
use ratatui::DefaultTerminal;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;

use crate::config::{get_config_path, load_config, ImapConfig, ReadBackend};
use crate::imap::{imap_thread, ReadMessage, Response};
use crate::tui::compose::ComposeWidget;
use crate::tui::inbox::{InboxState, InboxWidget};
use crate::tui::login::LoginWidget;
use crate::tui::reading::ReadingWidget;
use crate::{cli::App, oauth::execute_authentication_flow};
use ratatui::widgets::TableState;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

fn setup_logging() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::never(".", "ectt.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    _guard
}

fn main() -> Result<(), Error> {
    let _guard = setup_logging();
    color_eyre::install().unwrap();

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
        cli::Command::Run { config } => {
            let config_path = get_config_path(config).inspect_err(|err| {
                tracing::error!("Failed to get a configuration path: {err}");
            })?;
            let config = load_config(&config_path).inspect_err(|err| {
                tracing::error!(
                    "Failed to load configuration from path {} with error: {err}",
                    config_path.display()
                );
            })?;

            r(config)
        }
    }
}

fn r(ReadBackend::Imap(config): ReadBackend) -> Result<(), Error> {
    let (to_imap, from_main) = channel::<ReadMessage>();
    let (to_main, from_imap) = channel::<Response>();

    let imap_thread = std::thread::spawn(|| {
        tracing::debug!("Launching IMAP thread");
        imap_thread(config, from_main, to_main)
    });

    let terminal = ratatui::init();
    let result = run_tui(terminal, to_imap, from_imap);
    ratatui::restore();

    imap_thread.join().unwrap();

    Ok(result?)
}

enum Screen<'w> {
    Inbox(InboxWidget<'w>),
    Compose(ComposeWidget<'w>),
    Reading(ReadingWidget<'w>),
}

pub enum Page {
    Inbox,
    Compose,
    Reading,
}

impl<'w> From<Page> for Screen<'w> {
    fn from(value: Page) -> Self {
        match value {
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
    inbox_state: InboxState,
    request_inflight: bool,
}

impl Default for ScreenState {
    fn default() -> Self {
        Self {
            inbox_state: InboxState::new(),
            request_inflight: false,
        }
    }
}
fn run_tui(
    mut terminal: DefaultTerminal,
    to_imap: Sender<ReadMessage>,
    from_imap: Receiver<Response>,
) -> std::io::Result<()> {
    let mut screen = Screen::from(Page::Inbox);

    let mut state = ScreenState::default();

    to_imap
        .send(ReadMessage::ReadInbox {
            count: 5,
            offset: 0,
        })
        .unwrap();
    state.request_inflight = true;

    loop {
        match from_imap.try_recv() {
            Ok(Response::Inbox(inbox)) => {
                state.inbox_state.inbox.extend(inbox);
                state.request_inflight = false;
            }
            Err(TryRecvError::Empty) => { /* no-op */ }
            Err(TryRecvError::Disconnected) => {
                panic!()
            }
        }

        terminal.draw(|f| match &mut screen {
            Screen::Inbox(widget) => {
                f.render_stateful_widget(widget, f.area(), &mut state.inbox_state)
            }
            Screen::Compose(widget) => f.render_widget(&*widget, f.area()),
            Screen::Reading(widget) => f.render_widget(&*widget, f.area()),
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            let event = event::read()?;

            let action = match &mut screen {
                Screen::Inbox(widget) => {
                    if let Event::Key(KeyEvent {
                        code: KeyCode::Down,
                        ..
                    }) = event
                    {
                        if !state.request_inflight {
                            if let Some(selected) = state.inbox_state.table.selected() {
                                if selected == state.inbox_state.inbox.len() - 1 {
                                    to_imap
                                        .send(ReadMessage::ReadInbox {
                                            count: 5,
                                            offset: state.inbox_state.inbox.len() as u32,
                                        })
                                        .unwrap();
                                    state.request_inflight = true;
                                }
                            }
                        }
                    }

                    widget.handle_event(event, &mut state.inbox_state)
                }
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
