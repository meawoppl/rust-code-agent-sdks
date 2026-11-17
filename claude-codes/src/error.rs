//! Error types for the Claude Code protocol

use crate::io::ParseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Invalid message type: {0}")]
    InvalidMessageType(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid state transition: {0}")]
    InvalidState(String),

    #[error("Authorization code rejected: {message}")]
    CodeRejected { message: String },

    #[error(
        "Timed out awaiting a login outcome; CLI screen content after submission:\n{transcript}"
    )]
    LoginTimeout { transcript: String },

    /// The login CLI process exited without producing a login outcome.
    ///
    /// Reading `code` (signal deaths surface as `128 + signo`):
    /// - `0` / small integers — the CLI exited on its own (its error text,
    ///   if any, is in `transcript`).
    /// - `143` (SIGTERM) — terminated externally, OR the host application
    ///   dropped the `LoginFlow` mid-flight (the crate's `Drop` kills with
    ///   SIGTERM and, with the `log` feature, warns
    ///   "LoginFlow dropped while unfinished" — check logs to attribute).
    /// - `137` (SIGKILL) — killed externally (OOM killer, `kill -9`); the
    ///   crate never sends SIGKILL.
    #[error("Login CLI exited (code {code:?}) without a login outcome; output:\n{transcript}")]
    LoginChildExited {
        code: Option<u32>,
        transcript: String,
    },

    /// The login CLI exited cleanly (code 0) without an observed credentials
    /// write, and `claude auth status` did not confirm a login of the flow's
    /// kind. Not proof the login failed — completion just couldn't be
    /// verified.
    ///
    /// - `logged_in` / `auth_method` — what the status reported; `None` when
    ///   the status check itself failed (see `status_error`). `logged_in:
    ///   Some(false)` means signed out; `Some(true)` with another
    ///   `auth_method` means a different credential kind is active.
    /// - `status_error` — why the status check failed (spawn error, timeout,
    ///   unparseable output).
    #[error(
        "Login CLI exited cleanly but the login could not be verified (logged_in: {logged_in:?}, auth_method: {auth_method:?}, status_error: {status_error:?}); output:\n{transcript}"
    )]
    LoginExitUnverified {
        logged_in: Option<bool>,
        auth_method: Option<String>,
        status_error: Option<String>,
        transcript: String,
    },

    #[error("Timeout occurred")]
    Timeout,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] ParseError),

    #[error("Session UUID not yet available - no response received")]
    SessionNotInitialized,

    #[error("Binary not found: '{name}' is not on PATH. Is it installed?")]
    BinaryNotFound { name: String },

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;
