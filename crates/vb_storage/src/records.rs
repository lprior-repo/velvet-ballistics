#![forbid(unsafe_code)]
//! Durable record types for storage.

mod entities;
mod kinds;
mod status;

pub use entities::{BlobRecord, CompiledIrRecord, RunHeaderRecord, WorkflowSourceRecord};
pub use kinds::RecordKind;

pub use status::{
    KnownRunHeaderStatus, RunHeaderStatus, RunHeaderStatusClass, UnknownRunHeaderStatus,
};

#[cfg(test)]
mod tests {
    use super::{
        KnownRunHeaderStatus, RunHeaderStatus, RunHeaderStatusClass, UnknownRunHeaderStatus,
    };

    #[test]
    fn run_header_status_known_bytes_classify_as_known_statuses() {
        let cases = [
            (0, KnownRunHeaderStatus::Pending),
            (1, KnownRunHeaderStatus::Accepted),
            (2, KnownRunHeaderStatus::Active),
            (3, KnownRunHeaderStatus::Finished),
        ];

        for (byte, expected) in cases {
            let status = RunHeaderStatus::from_byte(byte);

            assert_eq!(status.as_byte(), byte);
            assert_eq!(status.known(), Ok(expected));
            assert_eq!(status.classify(), RunHeaderStatusClass::Known(expected));
            assert_eq!(RunHeaderStatus::from(expected).as_byte(), byte);
        }
    }

    #[test]
    fn run_header_status_unknown_byte_returns_typed_error_and_lossless_unknown() {
        let status = RunHeaderStatus::from_byte(255);

        assert_eq!(status.known(), Err(UnknownRunHeaderStatus::from_byte(255)));
        assert_eq!(status.classify(), RunHeaderStatusClass::Unknown(255));
        assert_eq!(status.as_byte(), 255);
    }

    #[test]
    fn run_header_status_known_try_from_rejects_unknown_byte() {
        assert_eq!(
            KnownRunHeaderStatus::try_from(9),
            Err(UnknownRunHeaderStatus::from_byte(9))
        );
    }
}
