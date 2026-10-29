use serde::Serialize;

/// Account errors cross the IPC boundary as JSON `{code, message}` so the
/// interface can translate `code`; `message` keeps technical details.
#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("{0}")]
    Offline(String),
    #[error("{0}")]
    Server(String),
    #[error("Sign-in was cancelled")]
    Cancelled,
    #[error("Sign-in was denied in the browser")]
    Denied,
    #[error("Sign-in timed out")]
    Timeout,
    #[error("Another sign-in is already in progress")]
    Busy,
    #[error("The account session was revoked or expired; sign in again")]
    SessionEnded,
    #[error("Not signed in")]
    SignedOut,
    #[error("Secure credential storage is unavailable: {0}")]
    Storage(String),
    #[error("{0}")]
    Invalid(String),
}

impl AccountError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Offline(_) => "ACCOUNT_OFFLINE",
            Self::Server(_) => "ACCOUNT_SERVER_ERROR",
            Self::Cancelled => "ACCOUNT_SIGN_IN_CANCELLED",
            Self::Denied => "ACCOUNT_SIGN_IN_DENIED",
            Self::Timeout => "ACCOUNT_SIGN_IN_TIMEOUT",
            Self::Busy => "ACCOUNT_SIGN_IN_BUSY",
            Self::SessionEnded => "ACCOUNT_SESSION_ENDED",
            Self::SignedOut => "ACCOUNT_SIGNED_OUT",
            Self::Storage(_) => "ACCOUNT_STORAGE_UNAVAILABLE",
            Self::Invalid(_) => "ACCOUNT_INVALID_RESPONSE",
        }
    }
}

impl Serialize for AccountError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let body = serde_json::json!({ "code": self.code(), "message": self.to_string() });
        s.serialize_str(&body.to_string())
    }
}

impl From<crate::core::AppError> for AccountError {
    fn from(error: crate::core::AppError) -> Self {
        Self::Storage(error.to_string())
    }
}

impl From<sqlx::Error> for AccountError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

pub type AccountResult<T> = Result<T, AccountError>;
