pub mod state;
use crate::{
    config::{ImapConfig, OAuthConfig},
    imap::state::UnauthenticatedState,
};
use chrono::{DateTime, Utc};
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEmail {
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
pub fn imap_thread(config: ImapConfig, rx: Receiver<Command>, tx: Sender<Response>) {
    let state = UnauthenticatedState::new(config).unwrap();
    let mut state = match state.authenticate() {
        Ok(state) => state,
        Err((err, _)) => {
            if let Err(err) = tx.send(Response::Error(err.into())) {
                tracing::error!("Failed to send error message to main thread with error: {err}");
            };
            return; // Nothing left to do since authenticate already tries to refresh the token
        }
    };

    loop {
        let message = match rx.recv() {
            Ok(message) => message,
            Err(err) => {
                tracing::error!("Error while receiving a message from main thread: {err}");
                break;
            }
        };

        match message {
            Command::ReadInbox { count, offset } => {
                let emails = state.read_inbox(count, offset);
                if let Err(err) = tx.send(Response::Inbox(emails)) {
                    tracing::error!(
                        "Failed to send inbox response to main thread with error: {err}"
                    );
                    break;
                }
            }
        }
    }
}

struct OAuthConfigWithUser<'a> {
    user: &'a str,
    config: &'a OAuthConfig,
}

impl imap::Authenticator for OAuthConfigWithUser<'_> {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user,
            self.config.access_token.secret(),
        )
    }
}
