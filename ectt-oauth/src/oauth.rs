use std::env;

use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use oauth2::{
    basic::BasicClient,
    reqwest::{self},
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, RedirectUrl, Scope, TokenUrl,
};
use serde::Deserialize;
use tokio::{io::AsyncWriteExt, net::TcpListener, sync::mpsc};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{cli::Provider, Error};

const GMAIL_CLIENT_ID: &str = env!("GMAIL_CLIENT_ID");
const GMAIL_CLIENT_SECRET: &str = env!("GMAIL_CLIENT_SECRET");

const GMAIL_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/auth";

pub type AppClient = BasicClient<
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

pub async fn execute_authentication_flow(
    client: AppClient,
    scopes: Vec<Scope>,
) -> Result<(), Error> {
    let tracker = TaskTracker::new();
    let cancellation_token = CancellationToken::new();

    let (tx, rx) = mpsc::channel(1);

    let state = AuthorizationState {
        rx,
        cancellation_token: cancellation_token.child_token(),
    };
    tracker.spawn(get_authorization(state, client, scopes));

    let state = RedirectServerState {
        tx,
        cancellation_token: cancellation_token.child_token(),
    };
    tracker.spawn(setup_redirect_server(state));

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl-C");
    tracing::info!("Received Ctrl-C, closing...");
    cancellation_token.cancel();
    tracker.close();

    Ok(())
}

struct AuthorizationState {
    rx: mpsc::Receiver<AuthorizationPayload>,
    cancellation_token: CancellationToken,
}

async fn get_authorization(
    mut state: AuthorizationState,
    client: AppClient,
    scopes: Vec<Scope>,
) -> Result<(), Error> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(scopes)
        .set_pkce_challenge(pkce_challenge)
        .url();

    let mut stdout = tokio::io::stdout();
    stdout
        .write_all(format!("Open URL: {}", auth_url).as_bytes())
        .await
        // If it fails writing to the stdout this error message is useless but ¯\_(ツ)_/¯
        .expect("Failed to write to stdout");
    stdout.flush().await.expect("Failed to flush stdout");

    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    // cancel should be here
    let authorization_payload = state.rx.recv().await.unwrap();

    debug_assert_eq!(authorization_payload.state.secret(), csrf_token.secret());

    // Now you can trade it for an access token.
    let token_result = client
        .exchange_code(authorization_payload.code)
        // Set the PKCE code verifier.
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        // and here
        .await
        .unwrap(); // TODO

    println!("{:?}", token_result);

    state.cancellation_token.cancel();

    Ok(())
}

#[derive(Debug, Clone)]
struct RedirectServerState {
    // Ideally, this would be a oneshot channel, however, due to oneshot being moved on send,
    // it doesn't work for our purposes, we could use an Arc<Mutex<C>> but that's overkill
    tx: mpsc::Sender<AuthorizationPayload>,
    cancellation_token: CancellationToken,
}

async fn setup_redirect_server(state: RedirectServerState) -> Result<(), std::io::Error> {
    let token = state.cancellation_token.clone();

    let app = Router::new()
        .route("/", get(authorization_callback))
        .with_state(state);

    tracing::debug!("Starting redirect server on 0.0.0.0:3000");
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
    // The server has received the redirect (or an error), it can now shutdown
    tracing::debug!("Requesting web server to stop...");
    cancellation_token.cancel();
}
