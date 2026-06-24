use thiserror::Error;

#[derive(Error, Debug)]
pub enum SprawlError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration parse error: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("Configuration missing required field: {0}")]
    ConfigMissingField(String),

    #[error("Action locked: {0}")]
    ActionLocked(String),

    #[error("Safety Gate vetoed operation: {0}")]
    SafetyVeto(String),

    #[error("Unknown error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, SprawlError>;
