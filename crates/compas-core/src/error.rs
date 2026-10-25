use std::path::{Path, PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{} ({source})", path.display())]
    IoAt {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("{url} answered with status {status}")]
    HttpStatus { status: u16, url: String },
    #[error("GitHub is limiting requests; try again in about {} minutes", retry_in_secs.div_ceil(60).max(1))]
    RateLimited { retry_in_secs: u64 },
    #[error("download failed verification: {0}")]
    Verification(String),
    #[error("invalid plugin list: {0}")]
    InvalidIndex(String),
    #[error("invalid release: {0}")]
    InvalidRelease(String),
    #[error("not a Compás card: {0}")]
    InvalidCard(String),
    #[error("not enough free space: {needed} bytes needed, {available} available")]
    NoSpace { needed: u64, available: u64 },
    #[error("not found: {0}")]
    NotFound(String),
    #[error(
        "The plugin record on this card is damaged (more than {limit} entries). \
         Move the record aside to start over; installed files stay in place."
    )]
    RecordDamaged { limit: usize },
    #[error("{}", crate::sdtool::friendly_message(code, message))]
    Sdtool { code: String, message: String },
    #[error("adb: {0}")]
    Adb(String),
    #[error("{0}")]
    Rejected(String),
}

impl Error {
    /// Stable machine-readable code for the UI.
    pub fn code(&self) -> &'static str {
        match self {
            Error::IoAt { .. } | Error::Io(_) => "io",
            Error::Network(_) | Error::HttpStatus { .. } => "network",
            Error::RateLimited { .. } => "rate_limited",
            Error::Verification(_) => "verification",
            Error::InvalidIndex(_) => "invalid_index",
            Error::InvalidRelease(_) => "invalid_release",
            Error::InvalidCard(_) => "invalid_card",
            Error::NoSpace { .. } => "no_space",
            Error::Sdtool { .. } => "sdtool",
            Error::Adb(_) => "adb",
            Error::RecordDamaged { .. } => "record_damaged",
            Error::NotFound(_) => "not_found",
            Error::Rejected(_) => "rejected",
        }
    }
}

/// Attaches the failing path to an I/O error.
pub trait IoContext<T> {
    fn at(self, path: &Path) -> Result<T>;
}

impl<T> IoContext<T> for std::io::Result<T> {
    fn at(self, path: &Path) -> Result<T> {
        self.map_err(|source| Error::IoAt {
            path: path.to_path_buf(),
            source,
        })
    }
}
