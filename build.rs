use std::env::current_dir;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct InstalledCredentials {
    pub client_id: String,
    pub project_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleCredentials {
    pub installed: InstalledCredentials,
}

fn main() {
    let credentials = ["gapi.creds.json", "../gapi.creds.json"]
        .into_iter()
        .find_map(|p| {
            let credentials_path = current_dir()
                .expect("current directory should be valid")
                .join(p);

            match std::fs::File::open(&credentials_path) {
                Ok(credentials) => Some(credentials),
                Err(err) => {
                    println!(
                        "cargo::warning=failed to read {} with error: {err}",
                        credentials_path.display()
                    );
                    None
                }
            }
        });

    let Some(credentials_file) = credentials else {
        return;
    };

    match serde_json::from_reader::<_, GoogleCredentials>(credentials_file) {
        Ok(credentials) => {
            println!(
                "cargo::rustc-env=GMAIL_CLIENT_ID={}",
                credentials.installed.client_id
            );
            println!(
                "cargo::rustc-env=GMAIL_CLIENT_SECRET={}",
                credentials.installed.client_secret
            );
        }
        Err(err) => {
            println!("cargo::warning=failed to read gapis.creds.json with error: {err}");
            return;
        }
    }
}
