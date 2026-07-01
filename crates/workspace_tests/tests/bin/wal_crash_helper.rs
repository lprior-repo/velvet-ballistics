#![forbid(unsafe_code)]
//! Sibling helper binary for `integration_subprocess_wal_crash_recovery`.
//!
//! Reads a JSON `HelperOp` from stdin, executes it against a real Fjall
//! keyspace (using `vb_storage`'s strict-commit path), and writes a JSON
//! `HelperResponse` to stdout. This binary intentionally does NOT perform
//! any graceful shutdown of the Fjall handle on commit — exiting at the
//! end of the op simulates a SIGKILL where no shutdown hook runs.

use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::ReadOnlyJournal;
use vb_storage::events::JournalEvent;
use vb_storage::journal::FjallJournal;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum HelperOp {
    OpenAndCommit { path: PathBuf, count: u32 },
    ReopenAndRead { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum HelperResponse {
    Committed { count: u32, last_seq: u64 },
    Replayed { count: u32, last_seq: u64 },
}

fn read_op() -> Result<HelperOp, String> {
    let mut buf = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buf)
        .map_err(|e| format!("stdin read: {e}"))?;
    serde_json::from_slice(&buf).map_err(|e| format!("decode op: {e}"))
}

fn write_response(response: &HelperResponse) {
    let encoded = serde_json::to_vec(response).expect("encode response");
    let mut out = std::io::stdout();
    out.write_all(&encoded).expect("write response");
    out.flush().ok();
}

fn run() -> Result<(), String> {
    let op = read_op()?;
    match op {
        HelperOp::OpenAndCommit { path, count } => {
            let journal = FjallJournal::open(&path, None)
                .map_err(|e| format!("open journal: {e}"))?;
            let mut last_seq: u64 = 0;
            let run = RunId::new(1);
            for i in 0..count {
                let seq = EventSeq::new(u64::from(i));
                let event = JournalEvent::RunAccepted {
                    run,
                    seq,
                    workflow: WorkflowDigest::from_bytes([0u8; 32]),
                };
                journal
                    .append_strict(&event)
                    .map_err(|e| format!("append_strict: {e}"))?;
                last_seq = u64::from(seq);
            }
            // Intentionally drop without orderly shutdown to simulate SIGKILL.
            drop(journal);
            write_response(&HelperResponse::Committed { count, last_seq });
        }
        HelperOp::ReopenAndRead { path } => {
            let journal = ReadOnlyJournal::open_inspect_view(&path)
                .map_err(|e| format!("open_inspect_view: {e}"))?;
            let replayed = journal
                .events_for_run(RunId::new(1))
                .map_err(|e| format!("events_for_run: {e}"))?;
            let count = replayed.len();
            let last_seq = replayed.last().map_or(0u64, |e| e.seq().get());
            write_response(&HelperResponse::Replayed {
                count: count as u32,
                last_seq,
            });
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("wal_crash_helper: {error}");
        std::process::exit(1);
    }
}