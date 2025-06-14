use std::sync::mpsc::{Receiver, Sender};

use imap::{Client, Connection, Session};
use mailparse::{parse_header, parse_headers, parse_mail, MailHeader, MailHeaderMap};
use oauth2::url::form_urlencoded::parse;

use crate::config::{Auth, ImapConfig, OAuthConfig, PasswordConfig};

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedEmail {
    pub from: String,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
}

pub enum ReadMessage {
    ReadInbox { count: u32, offset: u32 },
}

pub enum Response {
    Inbox(Vec<ParsedEmail>),
}

struct UnauthenticatedState {
    config: ImapConfig,
    client: imap::Client<Connection>,
}

impl UnauthenticatedState {
    fn authenticate(self) -> Result<AuthenticatedState, (imap::Error, Self)> {
        match &self.config.auth {
            Auth::Password(password_config) => {
                let login = self.config.login.clone();

                match self.client.login(login, &password_config.raw) {
                    Ok(session) => Ok(AuthenticatedState { session }),
                    Err((err, client)) => Err((
                        err,
                        Self {
                            config: self.config,
                            client,
                        },
                    )),
                }
            }
            Auth::OAuth(oauth_config) => {
                let authenticator = OAuthConfigWithUser {
                    user: &self.config.login,
                    config: &oauth_config,
                };
                match self.client.authenticate("XOAUTH2", &authenticator) {
                    Ok(session) => Ok(AuthenticatedState { session }),
                    Err((err, client)) => Err((
                        err,
                        Self {
                            config: self.config,
                            client,
                        },
                    )),
                }
            }
        }
    }
}

struct AuthenticatedState {
    session: imap::Session<Connection>,
}

impl AuthenticatedState {
    fn read_inbox(&mut self, count: u32, offset: u32) -> Vec<ParsedEmail> {
        self.session.select("INBOX").unwrap();
        // Fetch the 20 most recent emails by getting the highest UID and fetching the last 20
        let uids = self.session.uid_search("ALL").unwrap();
        let max_uid = uids.iter().max().copied().unwrap_or(1);

        let top = max_uid.saturating_sub(offset).max(1);
        let bot = max_uid.saturating_sub(offset + count).max(1);

        let messages = self
            .session
            .uid_fetch(format!("{bot}:{top}"), "RFC822")
            .unwrap();

        messages
            .iter()
            .filter_map(
                |message: &imap::types::Fetch<'_>| match message.body().map(parse_mail) {
                    Some(Ok(parsed)) => Some(parsed),
                    Some(Err(err)) => {
                        tracing::error!("Failed to parse email with error: {err}");
                        None
                    }
                    None => {
                        tracing::error!("Message does not have a body");
                        None
                    }
                },
            )
            .map(|parsed| {
                let headers = parsed.get_headers();

                tracing::info!("headers: {:?}", headers);

                ParsedEmail {
                    from: headers.get_first_value("From").unwrap_or_default(),
                    cc: headers.get_all_values("Cc"),
                    bcc: headers.get_all_values("Bcc"),
                    subject: headers
                        .get_first_value("Subject")
                        .unwrap_or_else(|| "No subject".to_string()),
                    body: parsed.get_body().unwrap(),
                }
            })
            .collect()
    }
}

pub fn imap_thread(config: ImapConfig, rx: Receiver<ReadMessage>, tx: Sender<Response>) {
    let ImapConfig { host, port, .. } = config.clone();
    let state = UnauthenticatedState {
        config,
        client: imap::ClientBuilder::new(host, port).connect().unwrap(),
    };

    let mut state = match state.authenticate() {
        Ok(state) => state,
        Err((err, client)) => panic!("{err}"), // TODO
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
            ReadMessage::ReadInbox { count, offset } => {
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
