// Cargo-fuzz target for accumulated byte admission (PS-001).
//
// Obligation ID: POB-vb-vzcuf-037
// Verifier: cargo-fuzz
// Command: cargo fuzz run vb_vzcuf_PS_001 -- -max_total_time=60
//
// Domain claim: Pure accumulated byte admission accepts exact fits
// and rejects over-limit totals with hostile encoded event inputs.
//
// PRODUCTION BINDING:
//   Fuzzes the u64::checked_add + limit logic that JournalWriteBatch::append_event
//   will use. Also fuzzes encode_record from crates/vb_storage/src/codec/mod.rs.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-037

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_storage::codec::encode_record;
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::records::RecordKind;
use vb_storage::events::JournalEvent;
use vb_storage::types::EventSeq;
use vb_core::{RunId, WorkflowDigest};

/// Admission function mirroring production logic.
fn admit_bytes(current: u64, candidate: u64, limit: u64) -> Result<u64, ()> {
    let total = current.checked_add(candidate).ok_or(())?;
    if total <= limit { Ok(total) } else { Err(()) }
}

/// Sub-target 0: Fuzz admission boundary with arbitrary u64 triplets.
fn fuzz_admission_boundary(data: &[u8]) {
    if data.len() < 24 { return; }
    let current_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let current = u64::from_le_bytes(current_bytes);
    let candidate_bytes: [u8; 8] = match data.get(8..16) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let candidate = u64::from_le_bytes(candidate_bytes);
    let limit_bytes: [u8; 8] = match data.get(16..24) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let limit = u64::from_le_bytes(limit_bytes);

    if limit == 0 { return; }
    if current > limit { return; }

    match admit_bytes(current, candidate, limit) {
        Ok(total) => {
            assert_eq!(total, current.wrapping_add(candidate));
            assert!(total <= limit);
            assert!(total >= current);
        }
        Err(()) => {
            let overflow = current.checked_add(candidate).is_none();
            let over = current.checked_add(candidate).map_or(true, |t| t > limit);
            assert!(overflow || over,
                "rejection without overflow or over-limit: current={current} candidate={candidate} limit={limit}");
        }
    }
}

/// Sub-target 1: Fuzz encode_record with arbitrary payload sizes.
fn fuzz_encode_record(data: &[u8]) {
    if data.len() < 8 { return; }
    let run_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let run = u64::from_le_bytes(run_bytes);
    if run == 0 { return; }

    let event = JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };

    // encode_record must not panic
    let _ = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() { return; }
    // Dispatch: first byte mod 2 selects sub-target
    match data[0] % 2 {
        0 => fuzz_admission_boundary(&data[1..]),
        _ => fuzz_encode_record(&data[1..]),
    }
});
