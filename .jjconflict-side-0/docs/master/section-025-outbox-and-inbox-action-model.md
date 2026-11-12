---
section: 25
title: "Outbox and Inbox Action Model"
parent: velvet-ballistics-MASTER.md
---

## 25. Outbox and Inbox Action Model

External side effects leave only through durable outbox records.

```rust
pub struct OutboxAppend {
    pub ticket: ActionTicket,
    pub action: ActionId,
    pub input_digest: Digest,
    pub idempotency_key_digest: IdempotencyKeyDigest,
    pub capability_mask: CapabilityMask,
}

pub struct InboxAppend {
    pub ticket: ActionTicket,
    pub completion: ActionCompletion,
    pub output_digest: Digest,
}
```

Rules:

```text
No ActionScheduled event, no dispatch.
No durable ticket, no completion.
Duplicate completion with same digest is ignored.
Duplicate completion with different digest is replay divergence.
Non-idempotent scheduled action is not re-executed during recovery.
Idempotent external action may reconcile/retry only with same key digest.
```

Crash states must be explicit:

```text
scheduled_not_dispatched
dispatched_no_completion
completion_recorded_frame_not_advanced
completion_duplicate_same_digest
completion_duplicate_different_digest
non_idempotent_pending_reconciliation
```

---

