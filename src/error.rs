//! Uses thiserror to define custom error types.

use thiserror::Error;

use crate::FuncError;

/// An error that can occur when parsing an identifier.
#[derive(Debug, Error)]
pub enum Error {
    /// The identifier is empty.
    #[error("Function Error")]
    Func(#[from] Box<FuncError>),
    /// Anyhow Error
    #[error("invalid identifier")]
    Anyhow(#[from] anyhow::Error),
}
