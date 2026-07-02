//! `From` conversions for [`crate::error::JournalError`].

use crate::error::JournalError;

impl From<crate::TrimError> for JournalError {
    fn from(err: crate::TrimError) -> Self {
        match err {
            crate::TrimError::Fjall(e) => Self::Fjall(e),
            crate::TrimError::Journal(e) => e,
            _ => Self::Trim(Box::new(err)),
        }
    }
}

impl From<std::io::Error> for JournalError {
    fn from(_: std::io::Error) -> Self {
        JournalError::UnexpectedEof
    }
}
