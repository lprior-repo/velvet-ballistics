#![forbid(unsafe_code)]
//! Strict persist drop-reopen durability scenarios (vb-o9zht).
//!
//! Existing durability tests often assert same-handle visibility rather
//! than true drop/reopen strict persistence. This module adds a focused
//! drop-and-reopen scenario layer that explicitly closes the FjallJournal
//! handle, drops it, opens a fresh inspect-view handle, and verifies the
//! strictly committed events are still surfaced in correct sequence order.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::ReadOnlyJournal;
use vb_storage::events::JournalEvent;
use vb_storage::journal::FjallJournal;

fn unique_temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!(
        "vb_persist_drop_reopen_{}_{}_{}",
        label,
        std::process::id(),
        nonce
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Strict-persist one run with N events, drop the writer handle, reopen
/// via a fresh inspect-view handle, and verify all N events are present
/// in dense ascending sequence order.
#[test]
fn strict_persist_drop_reopen_single_run_density() {
    let dir = unique_temp_dir("single_run");
    let run = RunId::new(0xAB);
    let event_count: u64 = 24;

    {
        let writer = FjallJournal::open(&dir, None).expect("open writer");
        for i in 0..event_count {
            let event = JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(i),
                workflow: WorkflowDigest::from_bytes([0u8; 32]),
            };
            writer.append_strict(&event).expect("strict append");
        }
        drop(writer);
    }

    let reader = ReadOnlyJournal::open_inspect_view(&dir).expect("open inspect view");
    let replayed = reader.events_for_run(run).expect("events for run");
    assert_eq!(
        replayed.len() as u64,
        event_count,
        "fresh handle must surface every strictly committed event"
    );
    for (index, event) in replayed.iter().enumerate() {
        assert_eq!(
            event.seq().get(),
            index as u64,
            "replay sequence must be dense and zero-indexed"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// Strict-persist multiple distinct runs, drop the writer, reopen via a
/// fresh inspect-view handle, and verify each run replays with the exact
/// event count committed for it. This guards against cross-run leakage in
/// the per-run index keyspace.
#[test]
fn strict_persist_drop_reopen_multiple_runs_isolated() {
    let dir = unique_temp_dir("multi_run");
    let run_a = RunId::new(0xC0);
    let run_b = RunId::new(0xC1);
    let run_c = RunId::new(0xC2);
    let events_per_run: u64 = 6;

    {
        let writer = FjallJournal::open(&dir, None).expect("open writer");
        for run in [run_a, run_b, run_c] {
            for i in 0..events_per_run {
                let event = JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(i),
                    workflow: WorkflowDigest::from_bytes([0u8; 32]),
                };
                writer.append_strict(&event).expect("strict append");
            }
        }
        drop(writer);
    }

    let reader = ReadOnlyJournal::open_inspect_view(&dir).expect("open inspect view");
    for run in [run_a, run_b, run_c] {
        let replayed = reader
            .events_for_run(run)
            .unwrap_or_else(|e| panic!("events_for_run {run:?}: {e}"));
        assert_eq!(
            replayed.len() as u64,
            events_per_run,
            "fresh handle must surface every strictly committed event for {run:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// Drop-reopen against repeated write/open cycles. This guards against
/// replay drift across multiple strict-commit + drop cycles within the
/// same keyspace path.
#[test]
fn strict_persist_drop_reopen_repeated_cycles() {
    let dir = unique_temp_dir("cycles");
    let run = RunId::new(0xDEAD_BEEF);
    let cycle_count: u32 = 4;
    let events_per_cycle: u64 = 8;

    let mut total_committed: u64 = 0;
    for cycle in 0..cycle_count {
        {
            let writer = FjallJournal::open(&dir, None)
                .unwrap_or_else(|e| panic!("open writer cycle {cycle}: {e}"));
            for i in 0..events_per_cycle {
                let event = JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(total_committed + i),
                    workflow: WorkflowDigest::from_bytes([0u8; 32]),
                };
                writer.append_strict(&event).expect("strict append");
            }
            drop(writer);
        }
        total_committed = total_committed
            .checked_add(events_per_cycle)
            .expect("non-overflow");
    }

    let reader = ReadOnlyJournal::open_inspect_view(&dir).expect("open inspect view");
    let replayed = reader.events_for_run(run).expect("events for run");
    assert_eq!(
        replayed.len() as u64,
        total_committed,
        "fresh handle after {cycle_count} cycles must replay every event"
    );
    for (index, event) in replayed.iter().enumerate() {
        assert_eq!(
            event.seq().get(),
            index as u64,
            "replay sequence must be dense across cycles"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}