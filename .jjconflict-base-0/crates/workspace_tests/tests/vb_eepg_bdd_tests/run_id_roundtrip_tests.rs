use super::*;

#[test]
fn run_header_key_preserves_run_id_bytes() -> Result<(), JournalError> {
    // Given: a RunId constructed from any u64 value
    let run_id = RunId::new(0xDEAD_BEEF_CAFE_BABE);

    // When: run_header_key is called
    let key = keys::run_header_key(run_id)?;

    // Then: the resulting key bytes are exactly [0x10] followed by the big-endian u64 bytes
    assert_eq!(
        key[0],
        vb_storage::constants::PREFIX_RUN_HEADER,
        "prefix must be 0x10"
    );
    assert_eq!(
        &key[1..9],
        run_id.get().to_be_bytes(),
        "run_id bytes must be preserved in big-endian"
    );

    // And: the first 9 bytes decode back to the original RunId
    let recovered_run_id = RunId::new(u64::from_be_bytes(key[1..9].try_into().unwrap()));
    assert_eq!(recovered_run_id, run_id);
    Ok(())
}

#[test]
fn run_header_key_prefix_is_0x10() -> Result<(), JournalError> {
    // Given: a non-zero RunId (PO-cn2v4-001: zero is reserved)
    let run_id = RunId::new(1);

    // When: run_header_key is called
    let key = keys::run_header_key(run_id)?;

    // Then: key[0] equals 0x10 (PREFIX_RUN_HEADER)
    assert_eq!(key[0], 0x10, "prefix byte must be 0x10");
    assert_eq!(
        key[0],
        vb_storage::constants::PREFIX_RUN_HEADER,
        "prefix must match constant"
    );
    Ok(())
}

#[test]
fn run_header_key_zero_run_id() -> Result<(), JournalError> {
    // Given: RunId::new(0)
    let run_id = RunId::new(0);

    // When: run_header_key is called
    let result = keys::run_header_key(run_id);

    // Then: the encoder rejects RunId::new(0) with
    // `JournalError::InvalidRunId { run: RunId::new(0) }` because
    // zero is reserved as the "no run" sentinel (PO-cn2v4-001).
    match result {
        Err(JournalError::InvalidRunId { run }) => {
            assert_eq!(run, RunId::new(0), "the run must round-trip back as zero");
        }
        other => panic!(
            "expected JournalError::InvalidRunId for zero RunId, got {:?}",
            other
        ),
    }
    Ok(())
}

#[test]
fn run_header_key_max_run_id() -> Result<(), JournalError> {
    // Given: RunId::new(u64::MAX)
    let run_id = RunId::new(u64::MAX);

    // When: run_header_key is called
    let key = keys::run_header_key(run_id)?;

    // Then: key[1..9] equals u64::MAX.to_be_bytes()
    assert_eq!(
        &key[1..9],
        u64::MAX.to_be_bytes(),
        "max run_id must encode as all-0xFF bytes"
    );
    Ok(())
}
