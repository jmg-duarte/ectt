mod cli;
mod config;
mod imap;
mod oauth;
mod tui;

use std::sync::mpsc::channel;

use clap::Parser;
use oauth2::basic::BasicRequestTokenError;
use oauth2::{reqwest, HttpClientError};
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;

use crate::config::{get_config_path, load_config, ReadBackend};
use crate::imap::{imap_thread, ReadMessage, Response};
use crate::{cli::App, oauth::execute_authentication_flow};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    RefreshToken(#[from] BasicRequestTokenError<HttpClientError<reqwest::Error>>),

    #[error(transparent)]
    Imap(#[from] ::imap::Error),
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

            run(config)
        }
    }
}

fn run(ReadBackend::Imap(config): ReadBackend) -> Result<(), Error> {
    let (to_imap, from_main) = channel::<ReadMessage>();
    let (to_main, from_imap) = channel::<Response>();

    let imap_thread = std::thread::spawn(|| {
        tracing::debug!("Launching IMAP thread");
        imap_thread(config, from_main, to_main)
    });

    let terminal = ratatui::init();
    let result = tui::run(terminal, to_imap, from_imap);
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

    Ok(result?)
}
