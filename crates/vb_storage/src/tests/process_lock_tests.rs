#![forbid(unsafe_code)]
//! SECTION 2.7: Process Lock Invariant (BH-16)

use crate::FjallJournal;

/// TEST: second_journal_open_on_same_path_is_prevented_by_process_lock (BH-16)
///
/// Contract §6 BH-16: Second FjallJournal::open → ProcessLockHeld.
#[test]
fn second_journal_open_on_same_path_is_prevented_by_process_lock() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|e| format!("tempdir failed: {e}"))?;

    let _journal1 =
        FjallJournal::open(temp.path(), None).map_err(|e| format!("first open failed: {e}"))?;

    let result = FjallJournal::open(temp.path(), None);
    assert!(
        result.is_err(),
        "second open on same path must fail due to process lock"
    );
    Ok(())
}
