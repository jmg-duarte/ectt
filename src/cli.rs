#[derive(Debug, Clone, clap::Parser)]
pub struct App {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Command {
    Login {
        #[arg(default_value_t = Provider::Gmail)]
        provider: Provider,
    },

    Run {},
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
