#![allow(clippy::as_conversions, clippy::cast_possible_truncation)]

use proptest::prelude::*;

proptest! {
    #[test]
    fn ppi_001_deterministic_replay_invariant(
        run_val in 1u64..=1000u64,
        step_count in 1u16..=5u16,
        seed_val in 0u8..=99u8,
    ) {
        // PO-010: deterministic replay property registered under the planned
        // `proptests::ppi_001_deterministic_replay_invariant` cargo-test filter.
        use crate::recovery::recover_runtime_summary;
        use crate::{EventSeq, FjallConfig, FjallJournal, JournalEvent};
        use tempfile::TempDir;
        use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};

        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([seed_val; 32]);
        let mut events = Vec::new();
        events.push(JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        });

        let mut seq = 1u64;
        for step_idx in 0..step_count {
            events.push(JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(seq),
                step: StepIdx::new(step_idx),
                attempt: 1,
            });
            seq = seq.saturating_add(1);
            events.push(JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(seq),
                step: StepIdx::new(step_idx),
                output: SlotIdx::ZERO,
            });
            seq = seq.saturating_add(1);
        }

        let Ok(dir1) = TempDir::new() else {
            prop_assert!(false, "tempdir one creation failed");
            return Ok(());
        };
        let Ok(journal1) = FjallJournal::open(dir1.path(), Some(FjallConfig::default())) else {
            prop_assert!(false, "journal one open failed");
            return Ok(());
        };
        for event in &events {
            let append = journal1.append_strict(event);
            prop_assert!(append.is_ok(), "journal one append failed: {append:?}");
        }
        let summary1 = recover_runtime_summary(&journal1, run);

        let Ok(dir2) = TempDir::new() else {
            prop_assert!(false, "tempdir two creation failed");
            return Ok(());
        };
        let Ok(journal2) = FjallJournal::open(dir2.path(), Some(FjallConfig::default())) else {
            prop_assert!(false, "journal two open failed");
            return Ok(());
        };
        for event in &events {
            let append = journal2.append_strict(event);
            prop_assert!(append.is_ok(), "journal two append failed: {append:?}");
        }
        let summary2 = recover_runtime_summary(&journal2, run);

        prop_assert_eq!(summary1.is_ok(), summary2.is_ok());
        if let (Ok(h1), Ok(h2)) = (summary1, summary2) {
            let s1 = h1.summary();
            let s2 = h2.summary();
            prop_assert_eq!(s1.run, s2.run);
            prop_assert_eq!(s1.steps_started, s2.steps_started);
            prop_assert_eq!(s1.steps_succeeded, s2.steps_succeeded);
            prop_assert_eq!(s1.terminal, s2.terminal);
            prop_assert_eq!(s1.slots_written, s2.slots_written);
        }
    }
}
