pub mod config;
pub mod oauth;
pub mod state;

use crate::imap::state::UnauthenticatedState;
use chrono::{DateTime, Utc};
use config::ImapConfig;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEmail {
    pub uid: u32,
    pub date: DateTime<Utc>,
    pub from: String,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
}

pub enum Command {
    ReadInbox { count: u32, offset: u32 },
}

pub enum Response {
    Inbox(Vec<ParsedEmail>),
    Error(crate::Error),
}

#[tracing::instrument(skip_all)]
pub fn imap_thread(
    config: ImapConfig,
    rx: Receiver<Command>,
    tx: Sender<Response>,
) -> Result<(), crate::Error> {
    let state = UnauthenticatedState::new(config)?;
    let mut state = match state.authenticate() {
        Ok(state) => state,
        Err((err, _)) => {
            if let Err(err) = tx.send(Response::Error(err.into())) {
                tracing::error!("Failed to send error message to main thread with error: {err}");
            };
            return Ok(()); // Nothing left to do since authenticate already tries to refresh the token
        }
    };

    loop {
        let message = match rx.recv() {
            Ok(message) => message,
            Err(err) => {
                tracing::error!("Error while receiving a message from main thread: {err}");
                // It's ok to just break and return here because it means the main thread has closed the channel
                break;
            }
        };

        match message {
            Command::ReadInbox { count, offset } => {
                let emails = match state.read_inbox(count, offset) {
                    Ok(emails) => emails,
                    Err(err) => {
                        if let Err(err) = tx.send(Response::Error(err)) {
                            tracing::error!(
                                "Failed to send inbox response to main thread with error: {err}"
                            );
                            // It's ok to just break and return here because it means the main thread has closed the channel
                            break;
                        }
                        continue;
                    }
                };
                if let Err(err) = tx.send(Response::Inbox(emails)) {
                    tracing::error!(
                        "Failed to send inbox response to main thread with error: {err}"
                    );
                    // It's ok to just break and return here because it means the main thread has closed the channel
                    break;
                }
            }
        }
    }

    Ok(())
}
