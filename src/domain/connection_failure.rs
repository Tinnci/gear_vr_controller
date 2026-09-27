//! Transport failures cross the application boundary without UI text or Windows types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionFailureKind {
    Unreachable,
    AccessDenied,
    Protocol,
    Incompatible,
    Timeout,
    Other,
}

#[derive(Debug, Clone)]
pub struct ConnectionFailure {
    pub kind: ConnectionFailureKind,
    pub detail: String,
}

impl ConnectionFailure {
    pub fn new(kind: ConnectionFailureKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}
impl std::fmt::Display for ConnectionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail)
    }
}
impl std::error::Error for ConnectionFailure {}
