use std::env;

use axum::{
    extract::{Query, State},
    response::Html,
    routing::{get, post},
    Router,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use clap::Parser;
use oauth2::{
    basic::BasicClient,
    reqwest::{self},
    AccessToken, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet,
    EndpointSet, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use tokio::{
    net::TcpListener,
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
    task::spawn_blocking,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

const GMAIL_CLIENT_ID: &str = env!("GMAIL_CLIENT_ID");
const GMAIL_CLIENT_SECRET: &str = env!("GMAIL_CLIENT_SECRET");

const GMAIL_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/auth";

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Io(std::io::Error),
}

#[derive(Debug, Clone, clap::Parser)]
struct App {
    #[arg(default_value_t = Provider::Gmail)]
    provider: Provider,
}

#[derive(Debug, Clone, Default, clap::ValueEnum)]
enum Provider {
    #[default]
    Gmail,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Gmail => write!(f, "gmail"),
        }
    }
}

type AppClient = BasicClient<
    EndpointSet, // Auth URL
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet, // Token URL
>;

impl From<Provider> for (AppClient, Vec<Scope>) {
    fn from(value: Provider) -> Self {
        match value {
            Provider::Gmail => {
                let client_id = ClientId::new(GMAIL_CLIENT_ID.to_string());
                let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                    .expect("passed URL should be valid");
                let auth_url =
                    AuthUrl::new(GMAIL_AUTH_URL.to_string()).expect("passed URL should be valid");
                let redirect_url = RedirectUrl::new("http://localhost:3000".to_string())
                    .expect("passed URL should be valid");

                let client = BasicClient::new(client_id)
                    .set_token_uri(token_url)
                    .set_auth_uri(auth_url)
                    .set_redirect_uri(redirect_url)
                    .set_client_secret(ClientSecret::new(GMAIL_CLIENT_SECRET.to_string()));

                let scopes = vec![Scope::new("https://mail.google.com/".to_string())];

                (client, scopes)
            }
        }
    }
}

async fn get_authorization(
    mut state: AppState,
    client: AppClient,
    scopes: Vec<Scope>,
) -> Result<(), Error> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(scopes)
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("CSRF: {}", csrf_token.secret());

    println!("Open URL: {}", auth_url);

    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    let authorization_payload = state.rx.recv().await.unwrap();

    debug_assert_eq!(authorization_payload.state.secret(), csrf_token.secret());

    // Now you can trade it for an access token.
    let token_result = client
        .exchange_code(authorization_payload.code)
        // Set the PKCE code verifier.
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
        .unwrap(); // TODO

    println!("{:?}", token_result);

    // imap is blocking, we wrap it here to avoid blocking the runtime
    let imap_client = spawn_blocking(|| {
        return imap::ClientBuilder::new("imap.gmail.com", 993)
            .connect()
            .unwrap();
    })
    .await
    .unwrap();

    let authenticator = OAuth2ImapAuthenticator {
        user: "duarte.gmj@gmail.com".to_string(),
        access_token: token_result.access_token().to_owned(),
    };

    let result =
        spawn_blocking(
            move || match imap_client.authenticate("XOAUTH2", &authenticator) {
                Ok(mut session) => {
                    session.select("INBOX").unwrap();

                    // fetch message number 1 in this mailbox, along with its RFC822 field.
                    // RFC 822 dictates the format of the body of e-mails
                    let messages = session.fetch("1", "RFC822").unwrap();
                    messages.iter().for_each(|msg| {
                        println!("{:?}", msg);

                        // extract the message's body
                        let body = msg.body().expect("message did not have a body!");
                        let body = std::str::from_utf8(body)
                            .expect("message was not valid utf-8")
                            .to_string();

                        println!("{}", body);
                    });

                    // be nice to the server and log out
                    session.logout().unwrap();
                }
                Err((err, client)) => {
                    tracing::error!("Failed to authenticate to server with error: {err}");
                    panic!()
                }
            },
        )
        .await
        .unwrap();

    Ok(())
}

struct OAuth2ImapAuthenticator {
    user: String,
    access_token: AccessToken,
}

impl imap::Authenticator for OAuth2ImapAuthenticator {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user,
            self.access_token.secret(),
        )
    }
}

#[derive(Debug, Clone)]
struct RedirectServerState {
    tx: Sender<AuthorizationPayload>,
    cancellation_token: CancellationToken,
}

impl RedirectServerState {
    fn new(tx: Sender<AuthorizationPayload>) -> Self {
        Self {
            tx,
            cancellation_token: CancellationToken::new(),
        }
    }
}

async fn setup_redirect_server(state: RedirectServerState) -> Result<(), std::io::Error> {
    let token = state.cancellation_token.clone();

    let app = Router::new()
        .route("/", get(authorization_callback))
        .with_state(state);

    tracing::info!("Starting server on 0.0.0.0:3000");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(async move {
            token.cancelled().await;
        })
        .await
}

#[derive(Debug, Deserialize)]
struct AuthorizationPayload {
    state: CsrfToken,
    code: AuthorizationCode,
    scope: Scope,
}

#[axum::debug_handler]
async fn authorization_callback(
    Query(payload): Query<AuthorizationPayload>,
    State(RedirectServerState {
        tx,
        cancellation_token,
    }): State<RedirectServerState>,
) {
    tracing::debug!("Received authorization payload: {payload:?}");
    if let Err(err) = tx.send(payload).await {
        tracing::error!(
            "Failed to send the payload code to the application thread with error: {err}"
        );
    };
    // The server has received the redirect, it can now shutdown
    cancellation_token.cancel();
    tracing::debug!("Requested web server to stop...");
}

struct AppState {
    rx: Receiver<AuthorizationPayload>,
}

async fn run(client: AppClient, scopes: Vec<Scope>) -> Result<(), Error> {
    let (tx, rx) = channel(1);

    let state = AppState { rx };
    let tracker = TaskTracker::new();
    tracker.spawn(get_authorization(state, client, scopes));

    let state = RedirectServerState::new(tx);
    tracker.spawn(setup_redirect_server(state));

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl-C");
    tracing::info!("Received Ctrl-C, closing...");
    tracker.close();

    Ok(())
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
        .block_on(run(client, scopes))
}
