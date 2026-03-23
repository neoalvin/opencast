use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenCastError {
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Player error: {0}")]
    PlayerError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] std::io::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Timeout")]
    Timeout,

    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}
