#![forbid(unsafe_code)]
#![cfg(test)]
//! Negative digest-mismatch tests for the modern ActionScheduledTicket /
//! ActionCompletedEnvelope recovery path.
//!
//! Wave 1 (vb-wy33p.2) added CLI integration tests plus tests in
//! `tests.rs` that exercise `verify_digests` with `FullDigestEvidence`.
//! Those tests cover the high-level request shape, but they do not
//! drive `recover_full_journal` directly with the modern envelope /
//! ticket event kinds and the slice-of-tuples parameters that the
//! CLI replay hands to recovery.
//!
//! These tests fill that gap by exercising `recover_full_journal`
//! end-to-end with a real `FjallJournal`:
//!
//! 1. Modern envelope with mismatched action-ABI digest →
//!    `ActionAbiMismatch { action_id }` (precise typed error).
//! 2. Modern envelope with matching action-ABI digest → `Ok(_)`.
//! 3. Modern envelope present + non-empty `expected_policy_digests`
//!    without a `RunAdmission` record → `PolicyDigestMismatch`
//!    (GAP-3 sentinel: digests cannot be verified).
//! 4. Ticket only (no envelope) + non-empty `expected_policy_digests`
//!    without `RunAdmission` → `PolicyDigestMismatch`.
//! 5. Modern envelope + `expected_policy_digests` that does not
//!    reference the envelope's step → `PolicyDigestMismatch`.
//! 6. Modern envelope + `RunAdmission` present + partial policy digest
//!    coverage → `Ok(_)` (admission is the verification anchor).
//! 7. The `action_id` carried in `ActionAbiMismatch` equals the
//!    `ActionId` baked into the `ActionCompletedEnvelope.ticket`.

mod tests {
    use crate::recovery::{ActionReplayTracker, RecoveryError, recover_full_journal};
    use crate::{DurableActionOutcome, EventSeq, FjallJournal, JournalEvent};
    use vb_core::RuntimePolicy;
    use vb_core::action::{ActionTicket, compute_action_idempotency_key};
    use vb_core::value::{SlotValue, Taint};
    use vb_core::{ActionId, CapabilitySet, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};

    fn sample_digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    fn action_ticket(run: RunId, step: StepIdx, action: ActionId) -> ActionTicket {
        let seq = SeqNo::ZERO;
        ActionTicket {
            run,
            step,
            seq,
            action,
            attempt: 1,
            idempotency_key: compute_action_idempotency_key(run, seq, action),
            capacity: 1,
        }
    }

    fn scheduled_ticket_event(
        run: RunId,
        seq: EventSeq,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        action_abi_digest: WorkflowDigest,
    ) -> JournalEvent {
        JournalEvent::ActionScheduledTicket {
            run,
            seq,
            ticket,
            action_abi_digest,
            input,
            output,
        }
    }

    fn completed_envelope_event(
        run: RunId,
        seq: EventSeq,
        ticket: ActionTicket,
        output: SlotIdx,
        value: SlotValue,
        taint: Taint,
        action_abi_digest: WorkflowDigest,
    ) -> JournalEvent {
        let encoded = postcard::to_allocvec(&value).expect("slot value encodes");
        let encoded_len = u32::try_from(encoded.len()).expect("encoded length fits u32");
        let value_digest = *blake3::hash(&encoded).as_bytes();
        JournalEvent::ActionCompletedEnvelope {
            run,
            seq,
            ticket,
            action_abi_digest,
            output,
            outcome: DurableActionOutcome::Ready,
            value: encoded,
            encoded_len,
            taint,
            value_digest,
        }
    }

    fn run_admission_event(
        run: RunId,
        seq: EventSeq,
        artifact_digest: WorkflowDigest,
    ) -> JournalEvent {
        JournalEvent::RunAdmission {
            run,
            seq,
            artifact_digest,
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        }
    }

    /// Builds a journal with `RunAccepted` + (optional) `RunAdmission` +
    /// a legacy `ActionScheduled` (for a different action+step) +
    /// `ActionScheduledTicket` (modern) + `ActionCompletedEnvelope`
    /// (modern) carrying the given `action_abi_digest`.
    ///
    /// Includes a legacy `ActionScheduled` event for an unrelated
    /// action+step so that `recover_full_journal`'s "missing required
    /// action schedule" sentinel (which only matches the legacy
    /// `ActionScheduled { .. }` variant, not the modern
    /// `ActionScheduledTicket`) does not fire before the per-event
    /// digest check runs. The legacy event does not participate in any
    /// digest check, so the modern envelope's action-ABI digest is the
    /// only one verified.
    fn append_modern_envelope_journal(
        journal: &FjallJournal,
        run: RunId,
        ticket: ActionTicket,
        envelope_action_abi_digest: WorkflowDigest,
        with_admission: bool,
    ) {
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(0x10),
        };
        journal
            .append_journaled(&accepted)
            .expect("setup: append RunAccepted");

        let mut next_seq: u64 = 1;
        let mut next_seq_value = || {
            let s = next_seq;
            next_seq = next_seq.saturating_add(1);
            EventSeq::new(s)
        };

        if with_admission {
            let admission = run_admission_event(run, next_seq_value(), sample_digest(0xAA));
            journal
                .append_journaled(&admission)
                .expect("setup: append RunAdmission");
        }

        // Legacy `ActionScheduled` for a different action+step. Its only
        // purpose is to satisfy `recover_full_journal`'s existence check
        // so the per-event digest verification on the modern envelope
        // actually runs.
        let legacy_scheduled = JournalEvent::ActionScheduled {
            run,
            seq: next_seq_value(),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        };
        journal
            .append_journaled(&legacy_scheduled)
            .expect("setup: append legacy ActionScheduled");

        let scheduled = scheduled_ticket_event(
            run,
            next_seq_value(),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
            envelope_action_abi_digest,
        );
        journal
            .append_journaled(&scheduled)
            .expect("setup: append ActionScheduledTicket");

        let completed = completed_envelope_event(
            run,
            next_seq_value(),
            ticket,
            SlotIdx::new(1),
            SlotValue::I64(42),
            Taint::Clean,
            envelope_action_abi_digest,
        );
        journal
            .append_journaled(&completed)
            .expect("setup: append ActionCompletedEnvelope");
    }

    // ------------------------------------------------------------------------
    // 1. Modern envelope + mismatched action-ABI digest → ActionAbiMismatch.
    // ------------------------------------------------------------------------

    #[test]
    fn recovery_rejects_action_abi_mismatch_on_modern_envelope() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(310);
        let action = ActionId::new(7);
        let ticket = action_ticket(run, StepIdx::new(2), action);

        // Envelope carries action_abi_digest = 0xA1.
        append_modern_envelope_journal(&journal, run, ticket, sample_digest(0xA1), false);

        // Caller expects a DIFFERENT action_ABI digest for the same action.
        let expected_action_abi_digests = [(action, sample_digest(0xA2))];
        let expected_policy_digests: [(StepIdx, WorkflowDigest); 0] = [];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(
            &journal,
            run,
            &mut tracker,
            &expected_action_abi_digests,
            &expected_policy_digests,
        );

        assert!(
            matches!(
                &result,
                Err(RecoveryError::ActionAbiMismatch { action_id: a }) if *a == action
            ),
            "modern envelope with mismatched action-ABI digest must surface ActionAbiMismatch, got {result:?}"
        );
    }

    // ------------------------------------------------------------------------
    // 2. Modern envelope + matching action-ABI digest → Ok(_).
    // ------------------------------------------------------------------------

    #[test]
    fn recovery_accepts_action_abi_match_on_modern_envelope() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(311);
        let action = ActionId::new(7);
        let ticket = action_ticket(run, StepIdx::new(2), action);

        // Envelope and expected carry the SAME action_ABI digest.
        append_modern_envelope_journal(&journal, run, ticket, sample_digest(0xA1), false);

        let expected_action_abi_digests = [(action, sample_digest(0xA1))];
        let expected_policy_digests: [(StepIdx, WorkflowDigest); 0] = [];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(
            &journal,
            run,
            &mut tracker,
            &expected_action_abi_digests,
            &expected_policy_digests,
        );

        let replayed = result.expect(
            "modern envelope with matching action-ABI digest must replay cleanly without admission",
        );
        assert!(
            replayed.iter().any(|e| matches!(
                e,
                JournalEvent::ActionCompletedEnvelope { ticket: t, .. } if t.action == action
            )),
            "replayed journal must contain the modern envelope, got {replayed:?}"
        );
    }

    // ------------------------------------------------------------------------
    // 3. Policy digest mismatch when modern envelope is ABSENT
    //    (only ActionScheduledTicket + no RunAdmission).
    // ------------------------------------------------------------------------

    #[test]
    fn recovery_rejects_policy_digest_mismatch_when_modern_envelope_absent() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(312);
        let action = ActionId::new(11);
        let ticket = action_ticket(run, StepIdx::new(3), action);

        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: sample_digest(0x10),
        };
        journal
            .append_journaled(&accepted)
            .expect("setup: append RunAccepted");

        // Ticket only — no envelope, no RunAdmission.
        let scheduled = scheduled_ticket_event(
            run,
            EventSeq::new(1),
            ticket,
            SlotIdx::new(0),
            SlotIdx::new(1),
            sample_digest(0xA1),
        );
        journal
            .append_journaled(&scheduled)
            .expect("setup: append ActionScheduledTicket");

        // Non-empty expected_policy_digests + missing RunAdmission
        // triggers the GAP-3 sentinel: digests cannot be verified.
        let expected_action_abi_digests: [(ActionId, WorkflowDigest); 0] = [];
        let expected_policy_digests = [(ticket.step, sample_digest(0xB1))];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(
            &journal,
            run,
            &mut tracker,
            &expected_action_abi_digests,
            &expected_policy_digests,
        );

        assert!(
            matches!(&result, Err(RecoveryError::PolicyDigestMismatch { .. })),
            "ticket-only journal with expected policy digests and no RunAdmission must surface \
             PolicyDigestMismatch, got {result:?}"
        );
    }

    // ------------------------------------------------------------------------
    // 4. Policy digest mismatch when modern envelope is PRESENT
    //    (envelope + ticket + no RunAdmission).
    // ------------------------------------------------------------------------

    #[test]
    fn recovery_rejects_policy_digest_mismatch_on_modern_envelope_present() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(313);
        let action = ActionId::new(13);
        let ticket = action_ticket(run, StepIdx::new(4), action);

        append_modern_envelope_journal(&journal, run, ticket, sample_digest(0xA1), false);

        let expected_action_abi_digests: [(ActionId, WorkflowDigest); 0] = [];
        let expected_policy_digests = [(ticket.step, sample_digest(0xB1))];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(
            &journal,
            run,
            &mut tracker,
            &expected_action_abi_digests,
            &expected_policy_digests,
        );

        assert!(
            matches!(&result, Err(RecoveryError::PolicyDigestMismatch { .. })),
            "modern envelope + expected policy digests + no RunAdmission must surface \
             PolicyDigestMismatch (GAP-3 sentinel), got {result:?}"
        );
    }

    // ------------------------------------------------------------------------
    // 5. Policy digest reference for an unrelated step + no RunAdmission
    //    must still fire the GAP-3 sentinel. The sentinel is keyed on the
    //    absence of RunAdmission, not on per-step matching, so this exercises
    //    the "missing-digest-for-modern-event" path explicitly.
    // ------------------------------------------------------------------------

    #[test]
    fn recovery_rejects_missing_policy_digest_for_modern_event() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(314);
        let action = ActionId::new(15);
        let envelope_step = StepIdx::new(5);
        let unrelated_step = StepIdx::new(99);
        let ticket = action_ticket(run, envelope_step, action);

        append_modern_envelope_journal(&journal, run, ticket, sample_digest(0xA1), false);

        let expected_action_abi_digests: [(ActionId, WorkflowDigest); 0] = [];
        // Caller supplies a digest for an UNRELATED step. The envelope's
        // step is not represented in the expected map.
        let expected_policy_digests = [(unrelated_step, sample_digest(0xC1))];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(
            &journal,
            run,
            &mut tracker,
            &expected_action_abi_digests,
            &expected_policy_digests,
        );

        assert!(
            matches!(&result, Err(RecoveryError::PolicyDigestMismatch { .. })),
            "modern envelope without a matching step in expected_policy_digests and no \
             RunAdmission must surface PolicyDigestMismatch, got {result:?}"
        );
    }

    // ------------------------------------------------------------------------
    // 6. With a durable RunAdmission present, partial policy digest coverage
    //    succeeds: the GAP-3 sentinel is gated on the absence of RunAdmission,
    //    and per-step policy digest checks live outside `recover_full_journal`'s
    //    replay path. Replay must therefore succeed without error.
    // ------------------------------------------------------------------------

    #[test]
    fn recovery_accepts_partial_policy_digest_match_with_admission() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(315);
        let action = ActionId::new(17);
        let envelope_step = StepIdx::new(6);
        let ticket = action_ticket(run, envelope_step, action);

        // With RunAdmission present.
        append_modern_envelope_journal(&journal, run, ticket, sample_digest(0xA1), true);

        // Partial coverage: only one of two known steps is represented.
        let expected_action_abi_digests: [(ActionId, WorkflowDigest); 0] = [];
        let expected_policy_digests = [
            (envelope_step, sample_digest(0xB1)),
            (StepIdx::new(99), sample_digest(0xB2)),
        ];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(
            &journal,
            run,
            &mut tracker,
            &expected_action_abi_digests,
            &expected_policy_digests,
        );

        let replayed = result.expect(
            "with RunAdmission present, partial expected_policy_digests coverage must replay cleanly",
        );
        assert!(
            replayed
                .iter()
                .any(|e| matches!(e, JournalEvent::RunAdmission { .. })),
            "replayed journal must contain RunAdmission, got {replayed:?}"
        );
        assert!(
            replayed.iter().any(|e| matches!(
                e,
                JournalEvent::ActionCompletedEnvelope { ticket: t, .. } if t.action == action
            )),
            "replayed journal must contain the modern envelope, got {replayed:?}"
        );
    }

    // ------------------------------------------------------------------------
    // 7. The `action_id` carried by `ActionAbiMismatch` must equal the
    //    `ActionId` baked into the envelope's ticket. This is the precise
    //    typed-error AC: the error variant preserves identity, not just kind.
    // ------------------------------------------------------------------------

    #[test]
    fn recovery_typed_error_carries_action_id_for_modern_envelope_mismatch() {
        let temp_dir = tempfile::tempdir().expect("setup: tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("setup: journal open");

        let run = RunId::new(316);
        let action = ActionId::new(23);
        let ticket = action_ticket(run, StepIdx::new(8), action);

        append_modern_envelope_journal(&journal, run, ticket, sample_digest(0xA1), false);

        // Pass a deliberately-wrong action-ABI digest for the envelope's
        // action. The error must echo the SAME action_id that the envelope
        // carries — not the first expected action in the slice.
        let expected_action_abi_digests = [(action, sample_digest(0xFE))];
        let expected_policy_digests: [(StepIdx, WorkflowDigest); 0] = [];

        let mut tracker = ActionReplayTracker::new();
        let result = recover_full_journal(
            &journal,
            run,
            &mut tracker,
            &expected_action_abi_digests,
            &expected_policy_digests,
        );

        let Err(RecoveryError::ActionAbiMismatch {
            action_id: reported,
        }) = result
        else {
            panic!(
                "modern envelope mismatch must surface ActionAbiMismatch with the envelope's \
                 action_id, got {result:?}"
            );
        };
        assert_eq!(
            reported, action,
            "ActionAbiMismatch.action_id must equal the ActionId baked into the envelope ticket"
        );
    }
}
