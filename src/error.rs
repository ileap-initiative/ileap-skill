use thiserror::Error;

/// Typed errors that carry a semantic CLI exit code.
/// All variants include the full human-readable message so that callers
/// at the `main` boundary can emit it without re-walking an error chain.
#[derive(Debug, Error)]
pub enum CliError {
    /// HTTP 404 — resource not found. Exit code 3.
    #[error("{0}")]
    NotFound(String),

    /// HTTP 401/403 or missing credentials. Exit code 4.
    #[error("{0}")]
    Auth(String),

    /// Any other failure (network, server 5xx, parse). Exit code 1.
    #[error("{0}")]
    Other(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::NotFound(_) => 3,
            CliError::Auth(_) => 4,
            CliError::Other(_) => 1,
        }
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            CliError::NotFound(_) => "not_found",
            CliError::Auth(_) => "auth_error",
            CliError::Other(_) => "error",
        }
    }
}
