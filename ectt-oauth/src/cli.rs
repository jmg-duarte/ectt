#[derive(Debug, Clone, clap::Parser)]
pub struct App {
    #[arg(default_value_t = Provider::Gmail)]
    pub provider: Provider,
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
