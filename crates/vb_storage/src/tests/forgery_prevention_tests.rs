#![forbid(unsafe_code)]
//! SECTION 2.5: Digest Forgery Prevention (BH-01 through BH-04, BH-14)

use crate::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES};
use crate::records::RecordKind;
use crate::{
    BlobRecord, DIGEST_BYTES, EventSeq, FjallJournal, JournalError, JournalEvent,
    WorkflowSourceRecord,
};
use vb_core::WorkflowDigest;

use crate::tests::fixtures::temp_journal;

/// TEST: forged_workflow_source_digest_rejected (BH-01)
///
/// Contract §6 BH-01: Direct put_workflow_source rejects forged digest.
#[test]
fn forged_workflow_source_digest_rejected() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let forged_digest = WorkflowDigest::from_bytes([0xDE; 32]);
    let record = WorkflowSourceRecord {
        digest: forged_digest,
        source: b"this will not hash to 0xDE..DE".to_vec(),
    };

    let result = journal.put_workflow_source(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "forged workflow source digest must be rejected"
    );
    Ok(())
}

/// TEST: forged_blob_digest_rejected (BH-01)
///
/// Contract §6 BH-01: Direct put_blob rejects forged digest.
#[test]
fn forged_blob_digest_rejected() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let forged_digest = [0xAB; 32];
    let record = BlobRecord {
        digest: forged_digest,
        bytes: b"these bytes won't hash to 0xAB..AB".to_vec(),
    };

    let result = journal.put_blob(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "forged blob digest must be rejected"
    );
    Ok(())
}

/// TEST: batch_forged_workflow_source_digest_rejected (BH-02)
///
/// Contract §6 BH-02: Batch put_workflow_source rejects forged digest.
#[test]
fn batch_forged_workflow_source_digest_rejected() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);
    let mut batch = journal.batch();

    let result = batch.put_workflow_source(&WorkflowSourceRecord {
        digest: forged_digest,
        source: b"not the right hash".to_vec(),
    });

    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "batch forged workflow source digest must be rejected"
    );
    Ok(())
}

/// TEST: batch_forged_blob_digest_rejected (BH-02)
///
/// Contract §6 BH-02: Batch put_blob rejects forged digest.
#[test]
fn batch_forged_blob_digest_rejected() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let mut batch = journal.batch();

    let result = batch.put_blob(&BlobRecord {
        digest: [0x11; 32],
        bytes: b"wrong hash".to_vec(),
    });

    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "batch forged blob digest must be rejected"
    );
    Ok(())
}

/// TEST: all_zero_digest_rejects_nonempty_content (BH-14)
///
/// Contract §6 BH-14: All-zero 32-byte digest rejects non-empty content.
#[test]
fn all_zero_digest_rejects_nonempty_content() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let content = b"some content that definitely won't hash to zeros";
    let zero_digest = [0u8; 32];
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(zero_digest),
        source: content.to_vec(),
    };

    let result = journal.put_workflow_source(&record);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "all-zero digest must reject non-empty content"
    );
    Ok(())
}
