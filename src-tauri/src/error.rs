use thiserror::Error;

/// Typed application error with stable codes for API and frontend consumers.
#[derive(Debug, Error)]
pub enum NetRailError {
    #[error("Invalid query: {message}")]
    InvalidQuery {
        code: &'static str,
        message: String,
    },

    #[error("Invalid open URL: {message}")]
    InvalidOpenUrl {
        code: &'static str,
        message: String,
    },

    #[error("Invalid configuration: {message}")]
    InvalidConfig {
        code: &'static str,
        message: String,
    },

    #[error("Invalid backend URL: {message}")]
    InvalidBackendUrl {
        code: &'static str,
        message: String,
    },

    #[error("Missing required field: {field}")]
    MissingField {
        code: &'static str,
        field: String,
    },

    #[error("{entity} not found")]
    NotFound {
        code: &'static str,
        entity: String,
    },

    #[error("{backend}: HTTP {status}")]
    BackendHttp {
        code: &'static str,
        backend: String,
        status: u16,
    },

    #[error("{backend}: {message}")]
    BackendFailure {
        code: &'static str,
        backend: String,
        message: String,
    },

    #[error("Fanout failed: {message}")]
    FanoutFailure {
        code: &'static str,
        message: String,
    },

    #[error("Database error: {message}")]
    Database {
        code: &'static str,
        message: String,
    },

    #[error("Network error: {message}")]
    Network {
        code: &'static str,
        message: String,
    },

    #[error("Parse error: {message}")]
    Parse {
        code: &'static str,
        message: String,
    },

    #[error("Encryption error: {message}")]
    Encryption {
        code: &'static str,
        message: String,
    },

    #[error("Internal error: {message}")]
    Internal {
        code: &'static str,
        message: String,
    },
}

pub type NetRailResult<T> = Result<T, NetRailError>;

impl NetRailError {
    pub fn status_code(&self) -> http::StatusCode {
        use http::StatusCode;
        match self {
            Self::InvalidQuery { .. }
            | Self::InvalidOpenUrl { .. }
            | Self::InvalidConfig { .. }
            | Self::InvalidBackendUrl { .. }
            | Self::MissingField { .. } => StatusCode::BAD_REQUEST,

            Self::NotFound { .. } => StatusCode::NOT_FOUND,

            Self::BackendHttp { .. }
            | Self::BackendFailure { .. }
            | Self::FanoutFailure { .. } => StatusCode::BAD_GATEWAY,

            Self::Database { .. }
            | Self::Network { .. }
            | Self::Parse { .. }
            | Self::Encryption { .. }
            | Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidQuery { code, .. }
            | Self::InvalidOpenUrl { code, .. }
            | Self::InvalidConfig { code, .. }
            | Self::InvalidBackendUrl { code, .. }
            | Self::MissingField { code, .. }
            | Self::NotFound { code, .. }
            | Self::BackendHttp { code, .. }
            | Self::BackendFailure { code, .. }
            | Self::FanoutFailure { code, .. }
            | Self::Database { code, .. }
            | Self::Network { code, .. }
            | Self::Parse { code, .. }
            | Self::Encryption { code, .. }
            | Self::Internal { code, .. } => code,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "code": self.error_code(),
            "detail": self.to_string(),
            "status": self.status_code().as_u16(),
        })
    }
}

impl From<rusqlite::Error> for NetRailError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database {
            code: "DB_ERROR",
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for NetRailError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse {
            code: "JSON_PARSE_ERROR",
            message: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for NetRailError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Network {
                code: "NETWORK_TIMEOUT",
                message: "Request timed out".into(),
            }
        } else if err.is_connect() {
            Self::Network {
                code: "NETWORK_CONNECT",
                message: "Connection failed".into(),
            }
        } else {
            Self::Network {
                code: "NETWORK_ERROR",
                message: err.to_string(),
            }
        }
    }
}

impl From<url::ParseError> for NetRailError {
    fn from(err: url::ParseError) -> Self {
        Self::InvalidBackendUrl {
            code: "URL_PARSE_ERROR",
            message: err.to_string(),
        }
    }
}