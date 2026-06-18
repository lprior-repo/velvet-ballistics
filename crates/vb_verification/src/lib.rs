//! Standalone Kani verification crate for vb-rpch recovery functions.
//!
//! This crate exists because Kani doesn't compile #[cfg(kani)] modules in
//! dependencies. By placing harnesses here (in a leaf crate), we ensure
//! vb_core and vb_storage are compiled with kani cfg set.

#![forbid(unsafe_code)]

#[cfg(kani)]
mod kani_harnesses {
    use vb_core::{RunId, StepIdx, WorkflowDigest};
    use vb_storage::recovery::RunSnapshot as VbRunSnapshot;
    use vb_storage::recovery::hydrate::{hydrate_run_frame, hydrate_run_frame_from_events};
    use vb_storage::{EventSeq, JournalEvent};

    // Newtype wrapper to bypass orphan rule: we can impl Arbitrary for our own types
    #[derive(kani::Arbitrary)]
    struct ArbitraryRunSnapshot {
        run_id: u64,
        seq_no: u64,
        workflow_digest: [u8; 32],
        slots_len: usize,
        taint_len: usize,
    }

    impl ArbitraryRunSnapshot {
        fn to_vb_snapshot(&self) -> VbRunSnapshot {
            VbRunSnapshot {
                run: RunId::new(self.run_id),
                seq: EventSeq::new(self.seq_no),
                workflow: WorkflowDigest::from_bytes(self.workflow_digest),
                slots: Vec::with_capacity(self.slots_len),
                taint: Vec::with_capacity(self.taint_len),
            }
        }
    }

    // Proof: hydrate_run_frame returns Err when snapshot.run != run_id
    #[kani::proof]
    #[kani::unwind(5)]
    fn hydrate_run_frame_precond_run_id_mismatch() {
        let arbitrary_snapshot: ArbitraryRunSnapshot = kani::any();
        let snapshot = arbitrary_snapshot.to_vb_snapshot();
        let run_id: RunId = kani::any();

        // Precondition: snapshot.run != run_id
        kani::assume(snapshot.run != run_id);

        // Create a single-element tail to avoid Vec<JournalEvent> issue
        let tail_event = JournalEvent::RunAccepted {
            run: run_id,
            seq: EventSeq::new(1),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        let tail_events = vec![tail_event];

        let result = hydrate_run_frame(&snapshot, &tail_events, run_id);

        // Postcondition: must return Err
        kani::assert(
            result.is_err(),
            "hydrate_run_frame must return Err when snapshot.run != run_id",
        );
    }

    // Proof: hydrate_run_frame_from_events returns Err on empty events
    #[kani::proof]
    #[kani::unwind(5)]
    fn hydrate_run_frame_from_events_precond_empty() {
        let events: Vec<JournalEvent> = Vec::new();
        let run_id: RunId = kani::any();

        let result = hydrate_run_frame_from_events(&events, run_id);

        // Postcondition: must return Err for empty events
        kani::assert(
            result.is_err(),
            "hydrate_run_frame_from_events must return Err on empty events",
        );
    }

    // Proof: hydrate_run_frame accepts valid matching run_id (no panic)
    #[kani::proof]
    #[kani::unwind(5)]
    fn hydrate_run_frame_postcond_ok() {
        let arbitrary_snapshot: ArbitraryRunSnapshot = kani::any();
        let snapshot = arbitrary_snapshot.to_vb_snapshot();
        let run_id: RunId = kani::any();

        // Create valid tail events with matching run_id
        let tail_events = vec![
            JournalEvent::RunAccepted {
                run: run_id,
                seq: EventSeq::new(1),
                workflow: WorkflowDigest::from_bytes([0u8; 32]),
            },
            JournalEvent::StepStarted {
                run: run_id,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                attempt: 1u16,
            },
        ];

        // The function should handle valid input without panic
        // (Result may be Ok or Err depending on other factors)
        let _ = hydrate_run_frame(&snapshot, &tail_events, run_id);
    }
}

#[cfg(not(kani))]
mod not_kani {
    // No-op stubs for non-kani builds
}
