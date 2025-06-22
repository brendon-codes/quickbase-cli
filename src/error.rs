use thiserror::Error;

pub type Result<T> = std::result::Result<T, QuickbaseCliError>;

#[derive(Debug, Error)]
pub enum QuickbaseCliError {
    #[error("{area} is not implemented yet; see plans/initial for the staged implementation order")]
    NotImplemented { area: String },

    #[error("config error: {message}")]
    Config { message: String },

    #[error("cmd error: {message}")]
    Command { message: String },

    #[error("Quickbase returned HTTP status {status}")]
    HttpStatus { status: u16 },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
