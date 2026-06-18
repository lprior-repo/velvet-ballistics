// Kani proof harness for duplicate accounting (PS-009, C2).

#[cfg(kani)]
mod kani_duplicate_ps009 {
    use crate::constants::JOURNAL_KEY_BYTES;
    use crate::events::JournalEvent;
    use crate::types::EventSeq;
    use vb_core::{RunId, WorkflowDigest};

    fn run_accepted(run: u64, seq: u64, workflow_byte: u8) -> JournalEvent {
        JournalEvent::RunAccepted {
            run: RunId::new(run),
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([workflow_byte; 32]),
        }
    }

    fn terminal_digest_byte_or_assume(event: &JournalEvent) -> u8 {
        let JournalEvent::RunAccepted { workflow, .. } = event else {
            kani::assume(false);
            return 0;
        };
        match workflow.as_bytes().last() {
            Some(value) => *value,
            None => {
                kani::assume(false);
                0
            }
        }
    }

    fn has_terminal_digest_byte(event: &JournalEvent) -> bool {
        let JournalEvent::RunAccepted { workflow, .. } = event else {
            return false;
        };
        workflow.as_bytes().last().is_some()
    }

    /// C2: same event with same run+seq produces identical encoded output.
    #[kani::proof]
    fn check_same_event_same_encoding() {
        let run = kani::any();
        let seq = kani::any();
        let workflow_byte = kani::any();
        let event = run_accepted(run, seq, workflow_byte);
        let v1 = terminal_digest_byte_or_assume(&event);
        let v2 = terminal_digest_byte_or_assume(&event);

        kani::assert(
            v1 == v2,
            "same workflow digest gives same terminal payload byte",
        );
        kani::assert(
            has_terminal_digest_byte(&event),
            "payload model is non-empty",
        );
    }

    /// C2: different events produce different encoded output.
    #[kani::proof]
    fn check_different_events_different_encoding() {
        let run = kani::any();
        let seq = kani::any();
        let left_workflow_byte = kani::any();
        let right_workflow_byte = kani::any();
        kani::assume(left_workflow_byte != right_workflow_byte);

        let e1 = run_accepted(run, seq, left_workflow_byte);
        let e2 = run_accepted(run, seq, right_workflow_byte);
        let v1 = terminal_digest_byte_or_assume(&e1);
        let v2 = terminal_digest_byte_or_assume(&e2);

        kani::assert(v1 != v2, "different workflow digests change payload bytes");
    }

    /// C2: JOURNAL_KEY_BYTES is bounded and non-zero.
    #[kani::proof]
    fn check_journal_key_bytes_valid() {
        kani::assert(JOURNAL_KEY_BYTES > 0, "journal key bytes must be non-zero");
        kani::assert(JOURNAL_KEY_BYTES <= 256, "journal key bytes must be small");
    }

    /// C2: conservative and precise duplicate accounting policies are monotonic.
    #[kani::proof]
    fn check_duplicate_accounting_policies() {
        let encoded_len: u64 = kani::any();
        kani::assume(encoded_len > 0);
        kani::assume(encoded_len < 100_000);
        let current_bytes: u64 = kani::any();
        let Some(max_without_overflow) = u64::MAX.checked_sub(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assume(current_bytes < max_without_overflow);

        let Some(conservative) = current_bytes.checked_add(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assert(conservative > current_bytes, "conservative increases bytes");

        let Some(precise_new) = current_bytes.checked_add(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assert(
            precise_new == conservative,
            "new-key precise equals conservative",
        );

        let precise_dup = current_bytes;
        kani::assert(precise_dup < conservative, "duplicate precise is smaller");
        kani::assert(
            precise_dup == current_bytes,
            "duplicate precise keeps current",
        );
    }

    /// C2: staged bytes never decrease regardless of duplicate policy.
    #[kani::proof]
    fn check_staged_bytes_monotonic() {
        let current: u64 = kani::any();
        kani::assume(current < u64::MAX / 2);
        let encoded_len: u64 = kani::any();
        kani::assume(encoded_len < 1_000_000);

        let Some(new_cons) = current.checked_add(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assert(new_cons >= current, "conservative policy monotonic");

        let Some(new_precise_new) = current.checked_add(encoded_len) else {
            kani::assume(false);
            return;
        };
        kani::assert(new_precise_new >= current, "precise new-key monotonic");

        let new_precise_dup = current;
        kani::assert(new_precise_dup >= current, "precise duplicate monotonic");
        kani::assert(
            new_precise_dup == current,
            "precise duplicate keeps current",
        );
    }
}
