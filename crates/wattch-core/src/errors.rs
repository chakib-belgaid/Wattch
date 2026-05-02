use thiserror::Error;

pub type Result<T> = std::result::Result<T, WattchError>;

#[derive(Debug, Error)]
pub enum WattchError {
    #[error("frame payload too large: {size} bytes (max {max} bytes)")]
    FrameTooLarge { size: usize, max: usize },

    #[error("truncated frame payload: expected {expected} bytes, got {actual} bytes")]
    TruncatedPayload { expected: usize, actual: usize },

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("source not found: {0}")]
    SourceNotFound(u32),

    #[error("source unavailable: {0}")]
    SourceUnavailable(u32),

    #[error("stream already running")]
    StreamAlreadyRunning,

    #[error("stream not running")]
    StreamNotRunning,

    #[error("sampling interval too low: {interval_ns} ns (minimum {min_interval_ns} ns)")]
    IntervalTooLow {
        interval_ns: u64,
        min_interval_ns: u64,
    },

    #[error("internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Decode(#[from] prost::DecodeError),

    #[error(transparent)]
    Encode(#[from] prost::EncodeError),
}
