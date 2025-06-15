mod cli;
mod config;
mod imap;
mod oauth;
mod smtp;
mod tui;

use std::sync::mpsc::channel;

use clap::Parser;
use oauth2::basic::BasicRequestTokenError;
use oauth2::{reqwest, HttpClientError};
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;

use crate::config::{get_config_path, Config};
use crate::imap::config::ReadBackend;
use crate::imap::imap_thread;
use crate::smtp::config::SendBackend;
use crate::{cli::App, oauth::execute_authentication_flow};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    RefreshToken(#[from] BasicRequestTokenError<HttpClientError<reqwest::Error>>),

    #[error(transparent)]
    Imap(#[from] ::imap::Error),

    #[error(transparent)]
    Email(#[from] lettre::error::Error),

    #[error(transparent)]
    Address(#[from] lettre::address::AddressError),

    #[error(transparent)]
    Smtp(#[from] lettre::transport::smtp::Error),
}

fn setup_logging() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::hourly("ectt.log", "");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .with_file(true)
        .with_line_number(true)
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
            let config = Config::load(&config_path).inspect_err(|err| {
                tracing::error!(
                    "Failed to load configuration from path {} with error: {err}",
                    config_path.display()
                );
            })?;

            run(config)
        }
    }
}

fn run(config: Config) -> Result<(), Error> {
    let Config {
        read: ReadBackend::Imap(imap_config),
        send: SendBackend::Smtp(smtp_config),
    } = config;

    let (main_tx_imap, imap_rx_main) = channel::<imap::Command>();
    let (imap_tx_main, main_rx_imap) = channel::<imap::Response>();
    let imap_thread = std::thread::spawn(|| {
        tracing::debug!("Launching IMAP thread");
        imap_thread(imap_config, imap_rx_main, imap_tx_main)
    });

    let (main_tx_smtp, smtp_rx_main) = channel::<smtp::Command>();
    let (smtp_tx_main, main_rx_smtp) = channel::<smtp::Response>();
    let smtp_thread = std::thread::spawn(|| {
        tracing::debug!("Launching SMTP thread");
        smtp::run(smtp_config, smtp_rx_main, smtp_tx_main)
    });

    let terminal = ratatui::init();
    let result = tui::run(
        terminal,
        main_tx_imap,
        main_rx_imap,
        main_tx_smtp,
        main_rx_smtp,
    );
    ratatui::restore();

    if let Err(err) = imap_thread.join() {
        if err.is::<Box<dyn std::error::Error>>() {
            tracing::error!(
                "Thread panicked with error: {}",
                err.downcast::<Box<dyn std::error::Error>>()
                    .expect("`.is` failed us")
            );
        } else {
            tracing::error!("Thread panicked with error: {:?}", err);
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Thread panic",
        ))?
    };

    if let Err(err) = smtp_thread.join() {
        if err.is::<Box<dyn std::error::Error>>() {
            tracing::error!(
                "Thread panicked with error: {}",
                err.downcast::<Box<dyn std::error::Error>>()
                    .expect("`.is` failed us")
            );
        } else {
            tracing::error!("Thread panicked with error: {:?}", err);
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Thread panic",
        ))?
    };

    Ok(result?)
}
