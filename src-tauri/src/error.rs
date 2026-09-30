use serde::Serialize;

/// Error type returned to the frontend. Always serialises to a plain, user-presentable
/// object: `{ code, message }`. Internal details are logged, not leaked.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("audio: {0}")]
    Audio(String),
    #[error("io: {0}")]
    Io(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Invalid(_) => "invalid",
            AppError::NotFound(_) => "not_found",
            AppError::Unsupported(_) => "unsupported",
            AppError::Audio(_) => "audio",
            AppError::Io(_) => "io",
            AppError::Internal(_) => "internal",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Invalid(e.to_string())
    }
}
#[cfg(windows)]
impl From<windows::core::Error> for AppError {
    fn from(e: windows::core::Error) -> Self {
        AppError::Audio(format!("{} (0x{:08X})", e.message(), e.code().0 as u32))
    }
}

pub type AppResult<T> = Result<T, AppError>;
