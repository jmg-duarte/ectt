use std::sync::mpsc::{Receiver, Sender};

use lettre::{
    address::AddressError,
    message::{header::ContentType, Mailbox},
    transport::smtp::{
        authentication::{Credentials, Mechanism},
        response::{Category, Code, Detail, Severity},
        SmtpTransportBuilder,
    },
    Address, Message, SmtpTransport, Transport,
};
use oauth2::{
    basic::BasicRequestTokenError,
    reqwest::{self, Error},
    HttpClientError, TokenResponse,
};

use crate::{
    oauth,
    smtp::{
        self,
        config::{Auth, SmtpConfig},
    },
};

pub mod config;

#[derive(Debug)]
pub struct PartialMessage {
    pub to: Option<Address>,
    pub cc: Vec<Address>,
    pub bcc: Vec<Address>,
    pub subject: Option<String>,
    pub body: Option<String>,
}

impl PartialMessage {
    pub fn to_message(self, from: Address) -> Result<Message, crate::Error> {
        let mut builder = Message::builder()
            .from(Mailbox::new(None, from))
            .subject(self.subject.unwrap_or_default())
            .header(ContentType::TEXT_PLAIN);

        if let Some(to) = self.to {
            builder = builder.to(to.into());
        }

        for cc in self.cc {
            builder = builder.cc(cc.into());
        }

        for bcc in self.bcc {
            builder = builder.bcc(bcc.into());
        }

        Ok(builder.body(self.body.unwrap_or_default())?)
    }
}

pub enum Command {
    SendMail(PartialMessage),
}

pub enum Response {
    SendMailSuccess,
    Error(crate::Error),
}

pub fn run(
    config: SmtpConfig,
    rx: Receiver<Command>,
    tx: Sender<Response>,
) -> Result<(), crate::Error> {
    let mut client = Client::new(config)?;

    loop {
        match rx.recv() {
            Ok(Command::SendMail(message)) => {
                match client.send(message) {
                    Ok(_) => {
                        if let Err(err) = tx.send(Response::SendMailSuccess) {
                            tracing::error!(
                                "Failed to send success message to main thread with error: {err}"
                            );
                            // It's ok to just break and return here because it means the main thread has closed the channel
                            break;
                        };
                    }
                    Err(err) => {
                        tracing::error!("Failed to send email with error: {err}");
                        if let Err(err) = tx.send(Response::Error(err)) {
                            tracing::error!(
                                "Failed to send error message to main thread with error: {err}"
                            );
                            // It's ok to just break and return here because it means the main thread has closed the channel
                            break;
                        }
                    }
                };
            }
            Err(err) => {
                tracing::error!("Failed to receive message with error: {err}");
                // It's ok to just break and return here because it means the main thread has closed the channel
                break;
            }
        }
    }
    Ok(())
}

pub struct Client {
    config: SmtpConfig,
    transport: lettre::SmtpTransport,
}

impl Client {
    pub fn new(config: SmtpConfig) -> Result<Self, crate::Error> {
        let (mechanisms, credentials) = match &config.auth {
            config::Auth::Password(password_config) => (
                vec![Mechanism::Xoauth2],
                Credentials::new(config.login.clone(), password_config.raw.clone()),
            ),
            config::Auth::OAuth(oauth_config) => (
                vec![Mechanism::Plain],
                Credentials::new(
                    config.login.clone(),
                    oauth_config.access_token.secret().clone(),
                ),
            ),
        };

        let transport = SmtpTransport::relay(&config.host)?
            .authentication(mechanisms)
            .credentials(credentials)
            .build();

        Ok(Self { config, transport })
    }

    pub fn send(&mut self, message: PartialMessage) -> Result<(), crate::Error> {
        let message = message.to_message(self.config.login.parse::<Address>()?)?;

        let Err(err) = self.transport.send(&message) else {
            return Ok(());
        };

        let Some(Code {
            severity: Severity::PermanentNegativeCompletion,
            category: Category::Unspecified3,
            detail: Detail::Five,
        }) = err.status()
        else {
            return Err(err.into());
        };

        match &self.config.auth {
            Auth::Password(_) => Err(err.into()),
            Auth::OAuth(_) => {
                tracing::debug!("Trying to refresh OAuth token");
                self.refresh_oauth_access_token()?;
                tracing::debug!("Successfully refreshed OAuth token");
                self.transport.send(&message)?;
                Ok(())
            }
        }
    }

    pub fn refresh_oauth_access_token(&mut self) -> Result<(), crate::Error> {
        let Auth::OAuth(ref mut config) = self.config.auth else {
            return Ok(());
        };

        let http_client = reqwest::blocking::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");

        let access_token = config
            .clone()
            .get_client()
            .exchange_refresh_token(&config.refresh_token)
            .request(&http_client)?;
        config.access_token = access_token.access_token().to_owned();

        let updated_transport = SmtpTransport::relay(&self.config.host)?
            .authentication(vec![Mechanism::Xoauth2])
            .credentials(Credentials::new(
                self.config.login.clone(),
                config.access_token.secret().clone(),
            ))
            .build();
        let _ = std::mem::replace(&mut self.transport, updated_transport);

        Ok(())
    }
}
