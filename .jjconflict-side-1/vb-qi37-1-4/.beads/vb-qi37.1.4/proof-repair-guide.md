# Proof Repair Guide — vb-qi37.1.4

## For proof-writer: actions required before proof can be approved

---

## R-001: Add Verus annotations for Rust-local invariants (CRITICAL — blocks approval)

**Target files**:
- `crates/vb_runtime/src/recovery.rs`
- `crates/vb_storage/src/recovery/recover.rs`

**Obligations**: PO-001, PO-002, PO-003, PO-004, PO-005, PO-006, PO-007, PO-008, PO-009

**Required annotations** in `crates/vb_runtime/src/recovery.rs`:

```rust
// For PO-001: INV-RC-001 (slot_values check)
spec fn spec_reject_unsupported_slot_values(unsupported: UnsupportedRecoveryState) -> bool {
    !unsupported.slot_values
}

// For PO-002: INV-RC-002 (slot_taint check)
spec fn spec_reject_unsupported_slot_taint(unsupported: UnsupportedRecoveryState) -> bool {
    !unsupported.slot_taint
}

// For PO-003: INV-RC-003 (action_payloads check) — THIS IS THE PRIMARY GAP
spec fn spec_reject_unsupported_action_payloads(unsupported: UnsupportedRecoveryState) -> bool {
    !unsupported.action_payloads
}

// For PO-004: INV-RC-004 (pending_actions check)
spec fn spec_reject_unsupported_pending_actions(unsupported: UnsupportedRecoveryState, pending_actions: &Seq<PendingAction>) -> bool {
    !unsupported.pending_actions || pending_actions.len() == 0
}

// Combined gate — mirrors RejectUnsupportedState in TLA+ spec
spec fn spec_reject_unsupported_live_frame_state(unsupported: UnsupportedRecoveryState, pending_actions: &Seq<PendingAction>) -> bool {
    spec_reject_unsupported_slot_values(unsupported)
        && spec_reject_unsupported_slot_taint(unsupported)
        && spec_reject_unsupported_action_payloads(unsupported)   // GAP: was missing
        && spec_reject_unsupported_pending_actions(unsupported, pending_actions)
}

// For PO-008: POST-RC-001 (hydration postcondition)
spec fn spec_hydrate_postcondition(seed: RecoveryFrameSeed, result: HydrateResult) -> bool {
    result.is_ok() ==> spec_reject_unsupported_live_frame_state(seed.unsupported, &seed.pending_actions)
}

// For PO-009: POST-RC-004 (action_payloads branch present in source)
spec fn spec_action_payloads_in_conditional(frame: RunFrame) -> bool {
    // verify action_payloads is checked in reject_unsupported_live_frame_state source
    true  // to be replaced with actual source branch existence proof
}

// Proof that action_payloads is checked
proof fn proof_reject_unsupported_action_payloads(unsupported: UnsupportedRecoveryState)
    requires spec_reject_unsupported_action_payloads(unsupported) {
    // lemma: action_payloads flag is checked
}

// Similar proof fns for PO-002, PO-004, PO-005 (slot_values, slot_taint, pending_actions, action_payloads_guarded)
proof fn proof_reject_unsupported_slot_values(unsupported: UnsupportedRecoveryState)
    requires spec_reject_unsupported_slot_values(unsupported) { }

proof fn proof_reject_unsupported_slot_taint(unsupported: UnsupportedRecoveryState)
    requires spec_reject_unsupported_slot_taint(unsupported) { }

proof fn proof_reject_unsupported_pending_actions(unsupported: UnsupportedRecoveryState, pending_actions: Seq<PendingAction>)
    requires spec_reject_unsupported_pending_actions(unsupported, &pending_actions) { }

proof fn proof_action_payloads_guarded(frame: RunFrame)
    requires frame.unsupported.action_payloads == true {
    // prove branch exists in reject_unsupported_live_frame_state
}

proof fn proof_hydrate_ok_implies_all_supported(seed: RecoveryFrameSeed, result: HydrateResult)
    requires result == Ok(frame) {
    // prove hydration_ok = TRUE implies all 4 unsupported flags are FALSE
}
```

**Required annotations** in `crates/vb_storage/src/recovery/recover.rs`:

```rust
// For PO-006: INV-RC-008 (action ABI digest check)
spec fn spec_verify_action_abi_digest(digest: Digest, check: DigestCheck, header: RecordHeader) -> bool {
    match check {
        DigestCheck::Full => digest == header.action_abi_digest,
        DigestCheck::Skip => true,
    }
}

proof fn proof_action_abi_mismatch_detected(digest: Digest, check: DigestCheck, header: RecordHeader)
    requires digest != header.action_abi_digest && check == DigestCheck::Full {
    // prove verify_digests returns Err(ActionAbiMismatch)
}

// For PO-007: INV-RC-009 (policy digest check)
spec fn spec_verify_policy_digest(digest: Digest, check: DigestCheck, header: RecordHeader) -> bool {
    match check {
        DigestCheck::Full => digest == header.policy_digest,
        DigestCheck::Skip => true,
    }
}

proof fn proof_policy_digest_mismatch_detected(digest: Digest, check: DigestCheck, header: RecordHeader)
    requires digest != header.policy_digest && check == DigestCheck::Full {
    // prove verify_digests returns Err(PolicyDigestMismatch)
}
```

**Run command**: `verus crates/vb_runtime/src/recovery.rs` and `verus crates/vb_storage/src/recovery/recover.rs` in full workspace context (after running code generator to produce `runtime/chunk_001.rs`).

**Expected evidence**: Verus report showing 0 errors for all proof fns.

---

## R-002: Run integration tests (HIGH — validates behavioral gap closure)

**Commands**:
```bash
cargo test -p vb_storage --test recovery_integration -- --nocapture
cargo test -p vb_runtime -- recovery -- --nocapture
```

**Obligations**: PO-012, PO-013, PO-014, PO-015, PO-016

**Expected evidence**: Test output showing:
- PO-012: `action_payloads=true` causes `hydrate_run_frame` to return `InvalidRecoveryHydration` (FAIL before fix, PASS after)
- PO-013: `DigestCheck::Full` wired for action ABI digests (FAIL before fix, PASS after)
- PO-014: `DigestCheck::Full` wired for policy digests (FAIL before fix, PASS after)
- PO-015: replay_events output contains RunResumed, RunRetried, RunAnswered when present
- PO-016: all 4 unsupported flag tests pass

---

## R-003: Add Kani harness and re-run (MEDIUM)

**Target file**: `crates/vb_storage/src/kani_codec.rs`

**Required harness** for PO-017:
```rust
#[kani::proof]
fn proof_recovery_frame_seed_roundtrip() {
    // Construct arbitrary RecoveryFrameSeed
    let seed = kani::any::<RecoveryFrameSeed>();
    // Encode via encode_record
    let encoded = encode_record(&seed);
    // Decode via decode_record
    let decoded = decode_record(&encoded).unwrap();
    // Assert equality
    assert_eq!(seed, decoded);
}
```

**Run command**: `cargo kani -p vb_storage --harness proof_recovery_frame_seed_roundtrip --unwind 3` with extended timeout.

**Expected evidence**: Kani report showing no failures.

---

## R-004: Fix proof-writer-report.md misleading language

**File**: `.beads/vb-qi37.1.4/proof-writer-report.md`

**Change**: Summary table Verus row from "**BLOCKED_TOOLING**" with "PASS" header → "**UNEXECUTED**" (no Verus annotations in source; compilation errors due to missing workspace deps)

---

## R-005: Fix TLA+ spec vacuity (MEDIUM)

**File**: `specs/tla/RecoveryReplay.tla`

**Change 1**: Remove `EventuallyHydatedOrRejected` or replace with meaningful liveness:
```
\* Replace tautological: <>(\/ hydration_ok = TRUE \/ hydration_ok = FALSE)
\* With meaningful progress claim (if protocol guarantees eventual decision):
EventuallyHydratedOrRejected == <> (hydration_ok = TRUE \/ rejected = TRUE)
```

**Change 2**: Add `NoSpuriousActionPayloads` to `specs/tla/RecoveryReplay.cfg` INVARIANTS section if it represents a safety property.

---

## Dependency Order

```
1. R-004 (fix report language)     — no dependencies
2. R-005 (fix TLA+ spec)           — no dependencies
3. R-001 (Verus annotations)        — requires workspace build + generated chunks
4. R-002 (integration tests)        — requires R-001 + implementation of fix
5. R-003 (Kani harness)            — no dependencies
```

---

## Tooling Prerequisites for R-001 (Verus)

Before Verus can run, the workspace must be buildable:
1. Run code generator to produce `vb_runtime/src/runtime/chunk_001.rs`
2. Ensure `vb_core` and `vb_storage` crates are resolvable
3. Run `verus` from workspace root with full dependency context

---

*proof-repair-guide: complete actions for proof-writer state 7*
