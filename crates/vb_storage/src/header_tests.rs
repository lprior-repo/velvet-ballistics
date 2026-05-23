#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod header_tests {
    use crate::{DIGEST_BYTES, FjallJournal, RunHeaderRecord};
    use vb_core::{RunId, WorkflowDigest, WorkflowId};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_header(run: RunId) -> RunHeaderRecord {
        RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
            status: 1,
            accepted_at_ms: 1000,
        }
    }

    #[test]
    fn put_run_header_stores_and_retrieves_by_run_id() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(100);
        let header = make_header(run);

        journal.put_run_header(&header).expect("put_run_header should succeed");

        let loaded = journal.run_header(run).expect("get should succeed");
        let found = loaded.expect("header should exist");
        assert_eq!(found.run, run);
        assert_eq!(found.workflow_id, header.workflow_id);
        assert_eq!(found.compiled_digest, header.compiled_digest);
        assert_eq!(found.status, header.status);
    }

    #[test]
    fn run_header_returns_none_for_missing_run() {
        let (_temp, journal) = temp_journal();
        let missing = RunId::new(9999);
        let result = journal.run_header(missing).expect("get should succeed");
        assert!(result.is_none(), "should return None for missing run header");
    }

    #[test]
    fn run_headers_returns_all_stored_headers() {
        let (_temp, journal) = temp_journal();
        for i in 1u64..=5 {
            let run = RunId::new(i);
            let header = RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
                status: 1,
                accepted_at_ms: i * 100,
            };
            journal.put_run_header(&header).expect("put should succeed");
        }

        let all = journal.run_headers().expect("run_headers should succeed");
        assert_eq!(all.len(), 5, "should have 5 run headers");
        assert!(all.iter().any(|h| h.run == RunId::new(1)));
        assert!(all.iter().any(|h| h.run == RunId::new(5)));
    }

    #[test]
    fn run_headers_returns_empty_for_empty_journal() {
        let (_temp, journal) = temp_journal();
        let all = journal.run_headers().expect("run_headers should succeed");
        assert!(all.is_empty(), "should return empty vec for empty journal");
    }

    #[test]
    fn put_run_header_updates_existing_run_header() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(200);

        let h1 = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(2),
            compiled_digest: WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]),
            status: 0,
            accepted_at_ms: 100,
        };
        journal.put_run_header(&h1).expect("first put should succeed");

        let h2 = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(2),
            compiled_digest: WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]),
            status: 1,
            accepted_at_ms: 200,
        };
        journal.put_run_header(&h2).expect("second put should succeed");

        let loaded = journal.run_header(run).expect("get should succeed");
        let found = loaded.expect("header should exist after update");
        assert_eq!(found.status, 1, "status should reflect the update");
        assert_eq!(found.accepted_at_ms, 200);
    }

    #[test]
    fn put_run_header_with_extreme_values() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(u64::MAX);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(u32::MAX),
            compiled_digest: WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]),
            status: 255,
            accepted_at_ms: u64::MAX,
        };
        journal.put_run_header(&header).expect("put should succeed with extreme values");

        let loaded = journal.run_header(run).expect("get should succeed");
        let found = loaded.expect("header should exist");
        assert_eq!(found.run, run);
        assert_eq!(found.status, 255);
        assert_eq!(found.accepted_at_ms, u64::MAX);
    }

    #[test]
    fn put_run_header_convenience_wrapper_works() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(300);
        let header = make_header(run);
        crate::put_run_header(&journal, &header).expect("convenience wrapper should succeed");

        let loaded = journal.run_header(run).expect("get should succeed").expect("should exist");
        assert_eq!(loaded.run, run);
    }
}
