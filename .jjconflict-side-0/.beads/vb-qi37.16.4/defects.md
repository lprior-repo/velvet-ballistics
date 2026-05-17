# Defects — vb-qi37.16.4 (State 11 Black Hat)

**Bead ID:** vb-qi37.16.4
**Title:** cli/runtime: Implement durable answer command
**Date:** 2026-05-11
**Phase:** State 11 — Black Hat Review

---

## Defect 1 — CRITICAL: INV-002 (Taint Enforcement) Not Enforced

**owner_state:** 11
**rerun_from:** 11

**File:** `crates/vb_ipc/src/server/handlers.rs:264`
**Contract Clause:** INV-002 — "The slot value written by an answer must not be `Secret`-tainted unless the workflow's `ResourceContract` explicitly allows secret results."

**Current Code:**
```rust
let answer = AskAnswer {
    ticket: AskTicket {
        run: run_id,
        ask_step,
        resume_step: ask_step,
    },
    answer_slot: SlotIdx::ZERO,
    value,
    taint: Taint::Clean,   // <-- HARDCODED, bypasses INV-002
    encoded_len,
};
```

**Problem:**
- `Taint::Clean` is hardcoded instead of being classified
- `ResourceContract::allows_secret_results` is never consulted
- Any secret-tainted `SlotValue` written via the answer path is silently misclassified
- The journal event records the wrong taint classification
- INV-002 (TLA+-owned temporal safety, Verus-owned Rust-local invariant) is structurally bypassed

**Required Fix Options:**
1. Add `taint: Taint` field to `IpcPayload::AnswerAsk` and require the CLI/caller to classify before IPC call
2. Perform content scanning at the IPC boundary before constructing `AskAnswer`
3. Query `ResourceContract::allows_secret_results` at the handler and reject secret answers that aren't permitted
4. Decode the `SlotValue`, inspect its taint classification, and apply INV-002 logic

**Severity:** CRITICAL — This is a formally owned contract invariant. Treating a violated invariant as "residual risk" is not acceptable for a release-critical bead.
