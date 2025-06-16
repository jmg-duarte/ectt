use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{imap::config::ReadBackend, smtp::config::SendBackend};

pub fn get_config_path<P>(path: Option<P>) -> Result<PathBuf, crate::Error>
where
    P: AsRef<Path>,
{
    if let Some(path) = path {
        return Ok(path.as_ref().to_path_buf());
    };

    if let Some(dir) = dirs::config_dir() {
        let config_file = dir.join("config.json");
        if config_file.exists() {
            return Ok(config_file);
        }
        tracing::warn!("Missing configuration file at: {}", config_file.display());
    }

    let config_file = std::env::current_dir()?.join("config.json");
    if config_file.exists() {
        return Ok(config_file);
    }

    tracing::error!(
        "Failed to find a configuration file at the fallback path: {}",
        config_file.display()
    );
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "No such file or directory",
    ))?
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub read: ReadBackend,
    pub send: SendBackend,
}

impl Config {
    pub fn load<P>(path: P) -> Result<Self, crate::Error>
    where
        P: AsRef<Path>,
    {
        let file = OpenOptions::new().read(true).open(path)?;
        Ok(serde_json::from_reader(file)?)
    }
}
