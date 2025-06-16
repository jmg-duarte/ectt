use std::path::PathBuf;

pub mod configure;

#[derive(Debug, Clone, clap::Parser)]
pub struct App {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Command {
    #[cfg(feature = "refresher")]
    Login {
        #[arg(default_value_t = Provider::Gmail)]
        provider: Provider,
    },

    Run {
        config: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub enum Provider {
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
