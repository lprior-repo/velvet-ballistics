# Bead vb-tphef: Typed Policy Foundation

## Contract

Implement core typed policy types in `vb_core` and an actor-owned pure live policy evaluator in `vb_runtime`. This is a foundation slice, not end-to-end enforcement.

### vb_core: Core Policy Types

**PolicyDigest:** 32-byte artifact digest (transparent wrapper, no defaults/fabrication).

**DecisionCode:** Allowed, Denied, Suspended.

**PolicyLayer:** Live, Admission, Emergency.

**PinnedAdmissionPolicy:** `Artifact(PolicyDigest)` or `Unverified` (explicit unverified, never fabricated zero).

**CurrentLivePolicy:** `Unconfigured` (default-deny, explicit state) or `Configured { digest: PolicyDigest, rule: PolicyRule }`.
- Bounded: single rule for this slice. No Vec/lookup complexity.

**PolicyRule:** `{ action: ActionId, code: DecisionCode }` - one action maps to one decision code. Uses `vb_core::ActionId` newtype, not raw u16.

**DecisionRecord:** Immutable record with:
- Full 8-field ActionTicket (run, step, seq, action, run_attempt, attempt, idempotency_key, capacity)
- Pinned admission identity
- Current live identity (Unconfigured vs Configured digest)
- Input digest (computed over canonical bytes + taint + all ticket fields, domain-separated)
- Taint
- Outcome + layer
- Typed emergency influence enum (NotEvaluated, Active, Inactive)
- Typed compatibility influence enum (NotEvaluated, Active(code))

**RuntimePolicy:** Preserved verbatim from base commit (Strict/Journaled/Relaxed) for durability semantics.

### vb_runtime: Actor-Owned Live Policy Evaluator

**LivePolicy:**
- Constructor takes NO admission pin (that's per-run). Initializes: live=Unconfigured, overlay=inactive.
- `configure(digest, rule)`: fallible, sets live configuration.
- `activate_emergency_deny()`: monotonic, no clear/reset API.
- `evaluate(ticket, pinned_admission, input_bytes, taint) -> DecisionRecord`:
  - Computes input digest from canonical postcard encoding of (input_bytes + taint + all 8 ticket fields) with domain separation
  - Emergency overlay active → Denied at Emergency layer
  - Unconfigured → Denied at Live layer
  - Configured matching rule → rule's code at Live layer
  - Configured non-matching → Denied at Live layer

No Arc/Mutex/RwLock/atomic/interior mutability. Plain owned state with &mut self configuration.

### Tests (Core)

Seven original RuntimePolicy tests from base commit, preserved verbatim.

### Tests (Runtime)

New focused tests: unconfigured deny, explicit permit/deny/suspend, overlay remains restrictive, decision record preserves full identities/digests, changing ticket field/taint changes digest, missing vs artifact admission distinguishable.

## Acceptance Criteria

- Scoped compile passes for vb_core and vb_runtime
- Scoped tests pass for both crates' policy modules
- Strict source lint passes: `cargo clippy -p vb_core -p vb_runtime --lib -- -D warnings`
- fmt check passes for modified files only
- No edits to existing test files or unrelated code
- No broad formatting of existing files
- Modified files <= 300 lines each
- Modified functions <= 25 logical lines each
- All tests use Result, not unwrap/expect
