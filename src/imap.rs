use std::sync::mpsc::{Receiver, Sender};

use chrono::{DateTime, TimeZone, Utc};
use imap::{Client, Connection, Session};
use mailparse::{dateparse, parse_header, parse_headers, parse_mail, MailHeader, MailHeaderMap};
use oauth2::{
    basic::BasicRequestTokenError,
    reqwest::{self, ClientBuilder, Error},
    url::form_urlencoded::parse,
    HttpClientError, TokenResponse,
};

use crate::config::{Auth, ImapConfig, OAuthConfig, PasswordConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEmail {
    pub date: DateTime<Utc>,
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

    Error(crate::Error),
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

    fn refresh_oauth_token(
        &mut self,
    ) -> Result<(), BasicRequestTokenError<HttpClientError<Error>>> {
        let Auth::OAuth(ref mut config) = self.config.auth else {
            return Ok(());
        };

        let http_client = reqwest::blocking::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");

        let result = config
            .clone()
            .get_client()
            .exchange_refresh_token(&config.refresh_token)
            .request(&http_client);
        match result {
            Ok(access_token) => {
                config.access_token = access_token.access_token().to_owned();
                Ok(())
            }
            Err(err) => Err(err),
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
            .uid_fetch(
                format!("{bot}:{top}"),
                "(UID INTERNALDATE RFC822 RFC822.TEXT)",
            )
            .unwrap();

        let mut parsed_emails = Vec::with_capacity(messages.len());

        for message in messages.iter() {
            let parsed = match message.body().map(parse_mail) {
                Some(Ok(parsed)) => parsed,
                Some(Err(err)) => {
                    tracing::error!("Failed to parse email with error: {err}");
                    continue;
                }
                None => {
                    tracing::error!("Message does not have a body");
                    continue;
                }
            };

            let headers = parsed.get_headers();
            let date = message
                .internal_date()
                .map(|internal_date| internal_date.to_utc())
                .unwrap_or_else(|| {
                    headers
                        .get_first_value("Received")
                        .map(|received| {
                            chrono::DateTime::from_timestamp(
                                dateparse(&received).unwrap_or_default(),
                                0,
                            )
                        })
                        .flatten()
                        .unwrap_or_else(|| chrono::DateTime::UNIX_EPOCH)
                });

            parsed_emails.push(ParsedEmail {
                date,
                from: headers.get_first_value("From").unwrap_or_default(),
                cc: headers.get_all_values("Cc"),
                bcc: headers.get_all_values("Bcc"),
                subject: headers
                    .get_first_value("Subject")
                    .unwrap_or_else(|| "No subject".to_string()),
                body: parsed.get_body().unwrap(),
            });
        }
        parsed_emails
    }
}

#[tracing::instrument(skip_all)]
pub fn imap_thread(config: ImapConfig, rx: Receiver<ReadMessage>, tx: Sender<Response>) {
    let ImapConfig { host, port, .. } = config.clone();
    let state = UnauthenticatedState {
        config,
        client: imap::ClientBuilder::new(host, port).connect().unwrap(),
    };

    let mut state = match state.authenticate() {
        Ok(state) => state,
        Err((err, mut client)) => {
            match client.config.auth {
                Auth::Password(_) => {
                    tracing::error!("Failed to authenticate using password with error: {err}");
                    if let Err(err) = tx.send(Response::Error(err.into())) {
                        tracing::error!(
                            "Failed to send error message to main thread with error: {err}"
                        )
                    };
                    return; // Nothing else to do, quit thread
                }
                Auth::OAuth(_) => {
                    if let Err(err) = client.refresh_oauth_token() {
                        tracing::error!("Failed to request a new access token with error: {err}");
                        if let Err(err) = tx.send(Response::Error(err.into())) {
                            tracing::error!(
                                "Failed to send error message to main thread with error: {err}"
                            );
                        }
                        return; // Nothing else to do, quit thread
                    }
                    match client.authenticate() {
                        Ok(authenticated) => authenticated,
                        Err((err, _)) => {
                            tracing::error!(
                                "Failed to authenticate using password with error: {err}"
                            );
                            if let Err(err) = tx.send(Response::Error(err.into())) {
                                tracing::error!(
                                    "Failed to send error message to main thread with error: {err}"
                                );
                            }
                            return; // Nothing else to do, quit thread
                        }
                    }
                }
            }
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
