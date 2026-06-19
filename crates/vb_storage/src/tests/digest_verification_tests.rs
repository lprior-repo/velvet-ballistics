#![forbid(unsafe_code)]
//! SECTION 2.3: verify_content_digest — Checksum Verification (Unit Tests)

use crate::JournalError;

/// TEST: verify_content_digest accepts matching hash
///
/// Contract §2.3 Postcondition (Success): blake3::hash(content) == expected.
#[test]
fn verify_content_digest_accepts_matching_hash() -> Result<(), String> {
    let content = b"test content for hashing".to_vec();
    let expected: [u8; 32] = blake3::hash(&content).into();

    let result = crate::journal::verify_content_digest(&content, &expected);
    assert!(
        result.is_ok(),
        "verify_content_digest must accept content matching expected hash"
    );
    Ok(())
}

/// TEST: verify_content_digest rejects mismatched hash
///
/// Contract §2.3 Postcondition (Failure): Returns PayloadDigestMismatch.
#[test]
fn verify_content_digest_rejects_mismatched_hash() -> Result<(), String> {
    let content = b"test content".to_vec();
    let wrong_digest: [u8; 32] = [0xFF; 32];

    let result = crate::journal::verify_content_digest(&content, &wrong_digest);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "mismatched hash must return PayloadDigestMismatch"
    );
    Ok(())
}

/// TEST: verify_content_digest never panics
///
/// Contract §2.3: Function returns Result — no panics on any input.
#[test]
fn verify_content_digest_never_panics() -> Result<(), String> {
    let content = b"any content".to_vec();
    let digest = [0x42u8; 32];

    // This must not panic — must return Result
    let result =
        std::panic::catch_unwind(|| crate::journal::verify_content_digest(&content, &digest));
    assert!(
        result.is_ok(),
        "verify_content_digest must not panic on any input"
    );
    Ok(())
}
