pub mod body;
pub mod combo;
pub mod compose;
pub mod focus;
pub mod help;
pub mod inbox;
pub mod line;
pub mod loading;
pub mod login;
pub mod reading;

use std::io::ErrorKind;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::DefaultTerminal;
use std::sync::mpsc::SendError;

use crate::imap::{Command, Response};
use crate::tui::compose::ComposeWidget;
use crate::tui::inbox::{InboxState, InboxWidget};
use crate::tui::loading::LoadingPopup;
use crate::tui::reading::ReadingWidget;
use crate::Error;

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
            Page::Reading => unreachable!("This should be handled in a different way"),
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

    to_imap: Sender<Command>,
    from_imap: Receiver<Response>,
}

impl ScreenState {
    fn new(to_imap: Sender<Command>, from_imap: Receiver<Response>) -> Self {
        Self {
            inbox_state: InboxState::new(),
            request_inflight: false,
            to_imap,
            from_imap,
        }
    }
}

impl ScreenState {
    fn load(&mut self) -> Result<(), SendError<Command>> {
        self.to_imap.send(Command::ReadInbox {
            count: 5,
            offset: 0,
        })?;
        self.request_inflight = true;
        Ok(())
    }

    fn load_more(&mut self, count: u32) -> Result<(), SendError<Command>> {
        if !self.request_inflight {
            if let Some(selected) = self.inbox_state.table.selected() {
                if selected == self.inbox_state.inbox.len() - 1 {
                    self.to_imap.send(Command::ReadInbox {
                        count,
                        offset: self.inbox_state.inbox.len() as u32,
                    })?;
                    self.request_inflight = true;
                }
            }
        };
        Ok(())
    }
}

#[tracing::instrument(skip_all)]
pub fn run(
    mut terminal: DefaultTerminal,
    to_imap: Sender<Command>,
    from_imap: Receiver<Response>,
) -> Result<(), Error> {
    let mut screen = Screen::from(Page::Inbox);

    let mut state = ScreenState::new(to_imap, from_imap);

    state.load().unwrap();

    loop {
        match state.from_imap.try_recv() {
            Ok(Response::Inbox(inbox)) => {
                state.inbox_state.inbox.extend(inbox);
                state.request_inflight = false;
            }
            Ok(Response::Error(err)) => {
                tracing::error!("IMAP thread failed with error: {err}");
                tracing::error!("Exiting...");
                return Err(err);
            }
            Err(TryRecvError::Empty) => { /* no-op */ }
            Err(TryRecvError::Disconnected) => {
                tracing::error!("IMAP channel disconnected");
                tracing::error!("Exiting...");
                return Err(Error::Io(std::io::Error::new(
                    ErrorKind::Other,
                    "IMAP channel got disconnected",
                )));
            }
        }

        terminal.draw(|f| match &mut screen {
            Screen::Inbox(widget) => {
                f.render_stateful_widget(widget, f.area(), &mut state.inbox_state);

                if state.request_inflight {
                    f.render_widget(LoadingPopup, f.area());
                }
            }
            Screen::Compose(widget) => f.render_widget(&*widget, f.area()),
            Screen::Reading(widget) => f.render_widget(&*widget, f.area()),
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            let event = event::read()?;

            let action = match &mut screen {
                Screen::Inbox(widget) => {
                    // Special "pre-events"
                    match event {
                        Event::Key(KeyEvent {
                            code: KeyCode::Down,
                            ..
                        }) => {
                            if let Err(err) = state.load_more(5) {
                                tracing::error!("Failed to send message to IMAP thread: {err}");
                                if cfg!(debug_assertions) {
                                    panic!("Channel was closed with pending messages");
                                } else {
                                    // If the channel is closed, it should mean that the program is exiting
                                    // so we close gracefully
                                    return Ok(());
                                }
                            };
                        }

                        Event::Key(KeyEvent {
                            code: KeyCode::Enter,
                            ..
                        }) => {
                            if let Some(selected) = state.inbox_state.table.selected() {
                                let Some(parsed_email) = state.inbox_state.inbox.get(selected)
                                else {
                                    tracing::warn!("Selected non-existing email, ignoring command");
                                    continue;
                                };
                                tracing::debug!("Parsed: {parsed_email:?}");
                                screen = Screen::Reading(ReadingWidget::from(parsed_email.clone()));
                                // We've handled what there is to handle, don't handle at the widget level
                                continue;
                            }
                        }
                        _ => { /* no-op */ }
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
