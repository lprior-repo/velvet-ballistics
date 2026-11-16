#![forbid(unsafe_code)]
//! Cargo-linked child operations for the existing subprocess WAL test.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, ReadOnlyJournal};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const RESPONSE_MARKER: &str = "VB_WAL_RESPONSE ";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum HelperOp {
    OpenAndCommit { path: PathBuf, count: u32 },
    ReopenAndRead { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) enum HelperResponse {
    Committed {
        count: u32,
        last_seq: u64,
    },
    Replayed {
        count: u32,
        last_seq: u64,
        event_bytes: Vec<u8>,
    },
}

pub(super) fn expected_events(count: u32) -> Vec<JournalEvent> {
    (0..count)
        .map(|index| JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(u64::from(index)),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        })
        .collect()
}

fn write_response(response: &HelperResponse) -> TestResult {
    let encoded = serde_json::to_string(response)?;
    let mut out = std::io::stdout().lock();
    writeln!(out, "{RESPONSE_MARKER}{encoded}")?;
    out.flush()?;
    Ok(())
}

pub(super) fn run() -> TestResult {
    let mut input = String::new();
    std::io::stdin().lock().read_line(&mut input)?;
    match serde_json::from_str::<HelperOp>(&input)? {
        HelperOp::OpenAndCommit { path, count } => {
            if count == 0 {
                return Err("commit count must be nonzero".into());
            }
            let journal = FjallJournal::open(&path, None)
                .map_err(|error| format!("open journal: {error}"))?;
            for event in expected_events(count) {
                journal.append_strict(&event)?;
            }
            write_response(&HelperResponse::Committed {
                count,
                last_seq: u64::from(count.checked_sub(1).ok_or("nonzero count")?),
            })?;
            // The parent kills this process while this actual journal still owns its lock.
            let mut release = [0u8; 1];
            std::io::stdin().read_exact(&mut release)?;
            drop(journal);
            Err("commit child was released instead of killed".into())
        }
        HelperOp::ReopenAndRead { path } => {
            let journal = ReadOnlyJournal::open_inspect_view(&path)?;
            let events = journal.events_for_run(RunId::new(1))?;
            write_response(&HelperResponse::Replayed {
                count: u32::try_from(events.len())?,
                last_seq: events.last().map_or(0, |event| event.seq().get()),
                event_bytes: postcard::to_allocvec(&events)?,
            })
        }
    }
}
