
use chrono::{DateTime, Utc};
use imap::Connection;
use itertools::Itertools;
use mail_parser::MessageParser;
use oauth2::{
    basic::BasicRequestTokenError,
    reqwest::{self, Error},
    HttpClientError, TokenResponse,
};

use crate::{
    config::{Auth, ImapConfig},
    imap::{OAuthConfigWithUser, ParsedEmail},
};

pub struct UnauthenticatedState {
    pub config: ImapConfig,
    pub client: imap::Client<Connection>,
}

impl UnauthenticatedState {
    pub fn new(config: ImapConfig) -> Result<Self, crate::Error> {
        let ImapConfig { ref host, port, .. } = config;
        let client = imap::ClientBuilder::new(host.clone(), port).connect()?;
        Ok(UnauthenticatedState { config, client })
    }

    pub fn authenticate(self) -> Result<AuthenticatedState, (imap::Error, Self)> {
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

    pub fn refresh_oauth_token(
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

pub struct AuthenticatedState {
    session: imap::Session<Connection>,
}

impl AuthenticatedState {
    pub fn read_inbox(&mut self, count: u32, offset: u32) -> Vec<ParsedEmail> {
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

        let parser = MessageParser::new();

        for message in messages.iter() {
            let Some(body) = message.body() else {
                tracing::warn!("Email does not contain a body, ignoring");
                continue;
            };

            let parsed = match parser.parse(body) {
                Some(parsed) => parsed,
                None => {
                    tracing::error!("Failed to parse email message, ignoring...");
                    continue;
                }
            };

            let date = match message.internal_date() {
                Some(date) => date.to_utc(),
                None => match parsed.date() {
                    Some(parsed_date) => {
                        DateTime::parse_from_rfc3339(parsed_date.to_rfc3339().as_str())
                            .expect("one of the libraries messed up RFC3339")
                            .to_utc()
                    }
                    None => {
                        tracing::warn!("No date was found, defaulting to UNIX_EPOCH");
                        DateTime::<Utc>::UNIX_EPOCH
                    }
                },
            };

            parsed_emails.push(ParsedEmail {
                date,
                from: Self::get_from(&parsed),
                cc: Self::get_cc(&parsed),
                bcc: Self::get_bcc(&parsed),
                subject: parsed.subject().unwrap_or("No subject").to_string(),
                body: (0..parsed.text_body_count())
                    .into_iter()
                    .map(|idx| parsed.body_text(idx).unwrap_or_default().to_string())
                    .join(""),
            });
        }
        parsed_emails
    }

    pub fn get_from(parsed: &mail_parser::Message) -> String {
        let from = match parsed.from() {
            Some(from) => from,
            None => return "No sender".to_string(),
        };

        let sender = match from.first() {
            Some(sender) => sender,
            None => return "No sender".to_string(),
        };

        match (&sender.name, &sender.address) {
            (None, None) => "Unknown sender".to_string(),
            (None, Some(address)) => address.to_string(),
            (Some(name), None) => name.to_string(),
            (Some(name), Some(address)) => format!("{name} ({address})"),
        }
    }

    pub fn get_cc(parsed: &mail_parser::Message) -> Vec<String> {
        let cc = match parsed.cc() {
            Some(cc) => cc,
            None => return vec![],
        };

        // I could parse the groups manually and probably get slightly better perf here but this works
        cc.clone()
            .into_list()
            .iter()
            .map(|addr| match (&addr.name, &addr.address) {
                (None, None) => "Unknown CC".to_string(),
                (None, Some(address)) => address.to_string(),
                (Some(name), None) => name.to_string(),
                (Some(name), Some(address)) => format!("{name} ({address})"),
            })
            .collect::<Vec<_>>()
    }

    pub fn get_bcc(parsed: &mail_parser::Message) -> Vec<String> {
        let bcc = match parsed.bcc() {
            Some(bcc) => bcc,
            None => return vec![],
        };

        // I could parse the groups manually and probably get slightly better perf here but this works
        bcc.clone()
            .into_list()
            .iter()
            .map(|addr| match (&addr.name, &addr.address) {
                (None, None) => "Unknown BCC".to_string(),
                (None, Some(address)) => address.to_string(),
                (Some(name), None) => name.to_string(),
                (Some(name), Some(address)) => format!("{name} ({address})"),
            })
            .collect::<Vec<_>>()
    }
}
