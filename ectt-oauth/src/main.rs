mod cli;
mod oauth;

use clap::Parser;

use crate::{cli::App, oauth::execute_authentication_flow};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(std::io::Error),
}
fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = App::parse();

    let (client, scopes) = app.provider.into();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build the runtime")
        .block_on(execute_authentication_flow(client, scopes))
}
