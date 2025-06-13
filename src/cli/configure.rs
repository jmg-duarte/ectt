#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Authentication {
    OAuth,
    Password,
}

pub struct Options {
    authentication: Authentication,
}
