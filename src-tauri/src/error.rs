use serde::Serialize;

/// What every command rejects with, serialised as `{ code, message }`.
#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl CommandError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<compas_core::Error> for CommandError {
    fn from(error: compas_core::Error) -> Self {
        Self::new(error.code(), error.to_string())
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
