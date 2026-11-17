use super::{decode_event_seq_for_trim, decode_snapshot_seq_for_trim};
use crate::keys::{run_event_key, run_snapshot_key};
use crate::trimming::TrimError;
use crate::{EventSeq, JournalError};
use vb_core::RunId;

fn malformed_keys(
    valid: &[u8],
    wrong_run: &[u8],
    wrong_kind: &[u8],
) -> Vec<(&'static str, Vec<u8>)> {
    let mut overlong = valid.to_vec();
    overlong.push(0);
    vec![
        ("empty", Vec::new()),
        ("wrong run", wrong_run.to_vec()),
        ("wrong kind", wrong_kind.to_vec()),
        (
            "truncated",
            valid
                .iter()
                .copied()
                .take(valid.len().saturating_sub(1))
                .collect(),
        ),
        ("overlong", overlong),
        (
            "zero run sentinel",
            valid
                .iter()
                .enumerate()
                .map(|(index, byte)| if (1..9).contains(&index) { 0 } else { *byte })
                .collect(),
        ),
        (
            "reserved sequence sentinel",
            valid
                .iter()
                .enumerate()
                .map(|(index, byte)| if index >= 9 { u8::MAX } else { *byte })
                .collect(),
        ),
        (
            "unknown prefix",
            valid
                .iter()
                .enumerate()
                .map(|(index, byte)| if index == 0 { u8::MAX } else { *byte })
                .collect(),
        ),
    ]
}

#[test]
fn trim_key_guards_return_exact_valid_sequence() -> Result<(), JournalError> {
    let run = RunId::new(71);
    for seq in [
        EventSeq::new(0),
        EventSeq::new(13),
        EventSeq::new(u64::MAX - 1),
    ] {
        let snapshot = run_snapshot_key(run, seq)?;
        let event = run_event_key(run, seq)?;
        assert!(
            matches!(decode_snapshot_seq_for_trim(&snapshot, run), Ok(actual) if actual == seq)
        );
        for count in [0, 7, u64::MAX] {
            assert!(
                matches!(decode_event_seq_for_trim(&event, run, count), Ok(actual) if actual == seq)
            );
        }
    }
    Ok(())
}

#[test]
fn snapshot_trim_key_guard_rejects_malformed_keys_with_zero_count() -> Result<(), JournalError> {
    let run = RunId::new(71);
    let seq = EventSeq::new(13);
    let valid = run_snapshot_key(run, seq)?;
    let wrong_run = run_snapshot_key(RunId::new(72), seq)?;
    let wrong_kind = run_event_key(run, seq)?;
    for (case, key) in malformed_keys(&valid, &wrong_run, &wrong_kind) {
        assert!(
            matches!(
                decode_snapshot_seq_for_trim(&key, run),
                Err(TrimError::IncompleteTrim { deleted_count: 0 })
            ),
            "snapshot key case: {case}"
        );
    }
    Ok(())
}

#[test]
fn event_trim_key_guard_preserves_supplied_count_on_rejection() -> Result<(), JournalError> {
    let run = RunId::new(71);
    let seq = EventSeq::new(13);
    let valid = run_event_key(run, seq)?;
    let wrong_run = run_event_key(RunId::new(72), seq)?;
    let wrong_kind = run_snapshot_key(run, seq)?;
    for (case, key) in malformed_keys(&valid, &wrong_run, &wrong_kind) {
        for count in [0, 7, u64::MAX] {
            assert!(
                matches!(
                    decode_event_seq_for_trim(&key, run, count),
                    Err(TrimError::IncompleteTrim { deleted_count }) if deleted_count == count
                ),
                "event key case: {case}, supplied count: {count}"
            );
        }
    }
    Ok(())
}
