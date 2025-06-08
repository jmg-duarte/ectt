use clap::Parser;
use oauth2::{
    basic::BasicClient,
    reqwest::{self, Client},
    ClientId, DeviceAuthorizationUrl, EndpointNotSet, EndpointSet, Scope,
    StandardDeviceAuthorizationResponse, StandardErrorResponse, StandardRevocableToken,
    StandardTokenIntrospectionResponse, StandardTokenResponse, TokenUrl,
};

#[cfg(debug_assertions)]
const GMAIL_CLIENT_ID: &str =
    "333808756948-n0ndev2ihe3hgqb91ljv0ln3j68c67jt.apps.googleusercontent.com";
#[cfg(not(debug_assertions))]
const GMAIL_CLIENT_ID: &str = env!("GMAIL_CLIENT_ID");

#[derive(Debug, Clone, clap::Parser)]
struct App {
    provider: Provider,
}

#[derive(Debug, Clone, Default, clap::ValueEnum)]
enum Provider {
    #[default]
    Gmail,
}

type AppClient =
    BasicClient<EndpointNotSet, EndpointSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

impl From<Provider> for (AppClient, Vec<Scope>) {
    fn from(value: Provider) -> Self {
        match value {
            Provider::Gmail => {
                let client_id = ClientId::new(GMAIL_CLIENT_ID.to_string());

                let device_authorization_url =
                    DeviceAuthorizationUrl::new("https://oauth2.googleapis.com/device/code".into())
                        .expect("passed URL should be valid");

                let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                    .expect("passed URL should be valid");

                let client = BasicClient::new(client_id)
                    .set_device_authorization_url(device_authorization_url)
                    .set_token_uri(token_url);

                let scopes = vec![Scope::new("https://mail.google.com/".to_string())];

                (client, scopes)
            }
        }
    }
}

fn get_authorization(client: AppClient, scopes: Vec<Scope>) {
    let http_client = reqwest::blocking::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    let details: StandardDeviceAuthorizationResponse = client
        .exchange_device_code()
        .add_scope(Scope::new("read".to_string()))
        .request(&http_client)
        .unwrap();

    println!(
        "Open this URL in your browser:\n{}\nand enter the code: {}",
        details.verification_uri().to_string(),
        details.user_code().secret().to_string()
    );

    let token_result = client
        .exchange_device_access_token(&details)
        .request(&http_client, std::thread::sleep, None)
        .unwrap();
}

fn main() {
    let app = App::parse();

    let (client, scopes) = app.provider.into();

    get_authorization(client, scopes);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build the runtime")
        .block_on(async {});
}
