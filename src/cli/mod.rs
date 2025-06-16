use std::path::PathBuf;

#[derive(Debug, Clone, clap::Parser)]
pub struct App {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Command {
    /// Utility to refresh OAuth tokens (only Gmail is currently supported).
    #[cfg(feature = "refresher")]
    Login {
        #[arg(default_value_t = crate::oauth::Provider::Gmail)]
        provider: crate::oauth::Provider,
    },

    /// Run the eCTT TUI.
    Run {
        /// Path to the configuration file (only JSON is supported).
        #[arg(long)]
        config: Option<PathBuf>,
    },
}
