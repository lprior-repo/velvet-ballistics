#![forbid(unsafe_code)]

use vb_core::ids::RunId;
use vb_core::{WorkflowDigest, WorkflowId};
use vb_storage::FjallJournal;

/// Creates a minimal run header in the journal so run_headers() returns it.
///
/// TEST USE ONLY — for setting up replay test scenarios. This is needed
/// because cancel/resume/retry/answer write events but not headers.
#[allow(unreachable_pub)]
pub fn create_run_header(journal: &FjallJournal, run: RunId) -> bool {
    let header = vb_storage::RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(1),
        compiled_digest: WorkflowDigest::from_bytes([0x42u8; 32]),
        status: 1,
        accepted_at_ms: 0,
    };
    journal.put_run_header(&header).is_ok()
}
