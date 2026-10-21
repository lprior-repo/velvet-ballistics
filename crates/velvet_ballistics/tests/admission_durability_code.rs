#![forbid(unsafe_code)]

use std::sync::Arc;

#[test]
fn api_envelope_preserves_admission_durability_code() {
    let error = vb_runtime::RuntimeError::AdmissionHeaderPersistenceFailed {
        source: Arc::new(vb_storage::JournalError::QueueFull),
    };

    assert_eq!(error.runtime_code(), Some("ADMISSION_DURABILITY_ERROR"));
    assert_eq!(
        error.diagnostic_code(),
        vb_runtime::RuntimeError::ADMISSION_HEADER_PERSISTENCE_FAILED_CODE
    );
}
