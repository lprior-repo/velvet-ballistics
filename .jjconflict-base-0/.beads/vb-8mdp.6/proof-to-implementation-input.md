# Proof-to-Implementation Input — Idempotency Hydration

## Bead: vb-8mdp.6

Maps approved proof claims to Rust source/test/harness obligations. Bridge agent uses this to emit exact Rust implementation tasks.

---

## 1. Kani Harness Obligations

### 1.1 New Harnesses Required

| Harness Name | Target Function | Source File | Obligations | Evidence Command |
|-------------|-----------------|-------------|-------------|------------------|
| `kani_key_determinism` | `compute_action_idempotency_key` | `crates/vb_core/src/action.rs` | PO-VB-IDEM-001a | `cargo kani -p vb_storage --harness kani_key_determinism` |
| `kani_divergent_ticket_evidence` | `mark_scheduled_ticket_effect` | `crates/vb_storage/src/recovery/types.rs` | PO-VB-IDEM-002a, PO-VB-IDEM-008a | `cargo kani -p vb_storage --harness kani_divergent_ticket_evidence` |
| `kani_validate_key_ingredients` | `validate_idempotency_key_ingredients` | `crates/vb_core/src/action.rs` | PO-VB-IDEM-003a, PO-VB-IDEM-009a | `cargo kani -p vb_storage --harness kani_validate_key_ingredients` |
| `kani_hydrate_run_frame_atomic` | `hydrate_run_frame` | `crates/vb_storage/src/recovery/hydrate.rs` | PO-VB-IDEM-004a | `cargo kani -p vb_storage --harness kani_hydrate_run_frame_atomic` |
| `kani_envelope_digest_mismatch` | `mark_completed_envelope_effect` | `crates/vb_storage/src/recovery/types.rs` | PO-VB-IDEM-005a | `cargo kani -p vb_storage --harness kani_envelope_digest_mismatch` |
| `kani_seq_after_snapshot` | `hydrate_snapshot_tail_seq_after_snapshot` | `crates/vb_storage/src/recovery/hydrate.rs` | PO-VB-IDEM-006a | `cargo kani -p vb_storage --harness kani_seq_after_snapshot` |
| `kani_non_idempotent_blocked` | `mark_scheduled_ticket_effect` | `crates/vb_storage/src/recovery/types.rs` | PO-VB-IDEM-007a | `cargo kani -p vb_storage --harness kani_non_idempotent_blocked` |
| `kani_snapshot_tail_preconditions` | `hydrate_snapshot_tail_preconditions` | `crates/vb_storage/src/recovery/hydrate.rs` | PO-VB-IDEM-010a | `cargo kani -p vb_storage --harness kani_snapshot_tail_preconditions` |
| `kani_hydrate_events_preconditions` | `hydrate_events_preconditions` | `crates/vb_core/src/action.rs` | PO-VB-IDEM-011a | `cargo kani -p vb_core --harness kani_hydrate_events_preconditions` |
| `kani_action_ticket_has_valid_key` | `action_ticket_has_valid_key` | `crates/vb_core/src/action.rs` | PO-VB-IDEM-012a | `cargo kani -p vb_core --harness kani_action_ticket_has_valid_key` |
| `kani_apply_tail_events_seq_order` | `apply_tail_events` | `crates/vb_storage/src/recovery/hydrate_support.rs` | PO-VB-IDEM-013a | `cargo kani -p vb_storage --harness kani_apply_tail_events_seq_order` |
| `kani_envelope_evidence_divergence` | `mark_completed_envelope_effect` | `crates/vb_storage/src/recovery/types.rs` | PO-VB-IDEM-014a | `cargo kani -p vb_storage --harness kani_envelope_evidence_divergence` |
| `kani_already_resolved_envelope` | `mark_completed_envelope_effect` | `crates/vb_storage/src/recovery/types.rs` | PO-VB-IDEM-015a | `cargo kani -p vb_storage --harness kani_already_resolved_envelope` |
| `kani_dimensions_positive` | `hydrate_dimensions_positive` | `crates/vb_storage/src/recovery/hydrate.rs` | PO-VB-IDEM-016a | `cargo kani -p vb_storage --harness kani_dimensions_positive` |
| `kani_verify_idempotency_missing_key` | `verify_idempotency` | `crates/vb_core/src/action.rs` | PO-VB-IDEM-017a | `cargo kani -p vb_core --harness kani_verify_idempotency_missing_key` |
| `kani_is_resolved` | `is_resolved` | `crates/vb_storage/src/recovery/types.rs` | PO-VB-IDEM-018a | `cargo kani -p vb_storage --harness kani_is_resolved` |
| `kani_require_scheduled_ticket` | `require_scheduled_ticket` | `crates/vb_storage/src/recovery/types.rs` | PO-VB-IDEM-020a | `cargo kani -p vb_storage --harness kani_require_scheduled_ticket` |

### 1.2 Existing Harnesses to Extend

| Harness Name | Current Coverage | Extension Needed |
|-------------|------------------|------------------|
| `crates/vb_storage/src/kani_recovery_hydrate.rs` | Partial hydration coverage | Add harnesses for all 17 new obligations |

### 1.3 Kani Arbitrary Instances Required

```rust
// In vb_core — for kani_hydrate_events_preconditions and kani_action_ticket_has_valid_key
impl kani::Arbitrary for RunId { ... }
impl kani::Arbitrary for SeqNo { ... }
impl kani::Arbitrary for ActionId { ... }

// In vb_storage — for kani_recovery_hydrate harnesses
impl kani::Arbitrary for ActionTicket { ... }
impl kani::Arbitrary for RunFrame { ... }
impl kani::Arbitrary for JournalEvent { ... }
impl kani::Arbitrary for SlotIdx { ... }
impl kani::Arbitrary for Taint { ... }
```

---

## 2. Proptest Obligations

### 2.1 New Property Tests Required

| Test Name | Target | Property | Command |
|-----------|--------|----------|---------|
| `test_key_computation_deterministic` | `compute_action_idempotency_key` | f(run, seq, action) == f(run, seq, action) for all inputs | `cargo test -p vb_core -- test_key_computation_deterministic` |
| `test_canonical_key_validates` | `action_ticket_has_valid_key` | canonical key returns true, non-canonical returns false | `cargo test -p vb_core -- test_canonical_key_validates` |

### 2.2 Strategy

```rust
proptest! {
    #[test]
    fn test_key_computation_deterministic(run in any::<RunId>(), seq in any::<SeqNo>(), action in any::<ActionId>()) {
        let key1 = compute_action_idempotency_key(run, seq, action);
        let key2 = compute_action_idempotency_key(run, seq, action);
        prop_assert_eq!(key1, key2);
    }

    #[test]
    fn test_canonical_key_validates(ticket in action_ticket_strategy()) {
        let canonical_key = compute_action_idempotency_key(ticket.run, ticket.seq, ticket.action);
        let mut ticket_with_canonical = ticket.clone();
        ticket_with_canonical.idempotency_key = canonical_key;
        prop_assert!(action_ticket_has_valid_key(&ticket_with_canonical));

        let mut ticket_with_wrong_key = ticket.clone();
        ticket_with_wrong_key.idempotency_key = canonical_key.wrapping_add(1);
        prop_assert!(!action_ticket_has_valid_key(&ticket_with_wrong_key));
    }
}
```

---

## 3. TLA+ Obligations

### 3.1 Existing Specs to Extend

| Spec | Current Coverage | Extension Needed |
|------|------------------|------------------|
| `verification/tla/IdempotencySafety.tla` | Key determinism, tracker independence, digest invariants | Add `DigestInvariant` for all completions with same (action, step) |
| `verification/tla/RecoveryHydration.tla` | Hydration atomicity, dimension bounds | Already covers PO-VB-IDEM-004b, PO-VB-IDEM-016b |

### 3.2 TLC Commands

```bash
# IdempotencySafety — all invariants
java -jar tla2tools.jar verification/tla/IdempotencySafety.tla -config verification/tla/IdempotencySafety.cfg

# RecoveryHydration — atomicity and dimension bounds
java -jar tla2tools.jar verification/tla/RecoveryHydration.tla -config verification/tla/RecoveryHydration.cfg
```

---

## 4. Verus Obligations

### 4.1 Existing Proofs to Extend

| Proof File | Current Coverage | Extension Needed |
|------------|------------------|------------------|
| `verification/verus/vb_rpch_action_replay_tracker.rs` | `is_resolved` monotonicity | Extend to cover `mark_completed_envelope_effect` already-resolved path |
| `verification/verus/idempotency_replay_tracker.rs` | Tracker surface refinement | Already covers PS-VB-IDEM-002, PS-VB-IDEM-007, PS-VB-IDEM-018 |

### 4.2 Verus Commands

```bash
cargo verus --verify verification/verus/vb_rpch_action_replay_tracker.rs
cargo verus --verify verification/verus/idempotency_replay_tracker.rs
```

---

## 5. Cargo/Dependency Check Obligations

```bash
# Verify vb_core has no vb_storage dependencies
cargo check -p vb_core 2>&1 | grep -i vb_storage || echo 'PASS: no vb_storage deps'

# Verify vb_storage compiles with vb_core
cargo check -p vb_storage 2>&1 | grep -i error || echo 'PASS: vb_storage compiles'
```

---

## 6. Test Obligations from Traceability Matrix

| Test Obligation | Source File | Requirement |
|----------------|-------------|-------------|
| `test_key_computation_deterministic` | vb_core/action.rs tests | VB-IDEM-HYDR-001 |
| `test_canonical_key_matches_ticket` | vb_core/action.rs tests | VB-IDEM-HYDR-001 |
| `test_tracker_key_is_action_step` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-002 |
| `test_same_key_different_ticket_diverges` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-002 |
| `test_validate_key_ingredients_rejects_secret` | vb_core/action.rs tests | VB-IDEM-HYDR-003 |
| `test_validate_key_ingredients_rejects_random` | vb_core/action.rs tests | VB-IDEM-HYDR-003 |
| `test_validate_key_ingredients_rejects_time` | vb_core/action.rs tests | VB-IDEM-HYDR-003 |
| `test_verify_idempotency_key_required_with_empty_slots` | vb_core/action.rs tests | VB-IDEM-HYDR-003 |
| `test_hydrate_run_frame_no_partial_state_on_error` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-004 |
| `test_completed_envelope_digest_mismatch_diverges` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-005 |
| `test_completed_envelope_digest_match_duplicate` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-005 |
| `test_hydrate_rejects_stale_tail_events` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-006 |
| `test_seq_ordering_invariant` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-006 |
| `test_non_idempotent_action_blocked_on_replay` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-007 |
| `test_unsafe_action_recovery_fails` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-007 |
| `test_divergent_tickets_replay_divergence` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-008 |
| `test_same_ticket_duplicate` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-008 |
| `test_preconditions_run_mismatch` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-010 |
| `test_preconditions_seq_order_violation` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-010 |
| `test_preconditions_no_evidence` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-010 |
| `test_events_preconditions_empty_false` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-011 |
| `test_events_preconditions_nonempty_true` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-011 |
| `test_canonical_key_validates` | vb_core/action.rs tests | VB-IDEM-HYDR-012 |
| `test_non_canonical_key_rejected` | vb_core/action.rs tests | VB-IDEM-HYDR-012 |
| `test_apply_tail_events_seq_order` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-013 |
| `test_envelope_digest_mismatch_diverges` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-014 |
| `test_envelope_output_mismatch_diverges` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-014 |
| `test_completed_action_envelope_duplicate` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-015 |
| `test_failed_action_envelope_duplicate` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-015 |
| `test_dimensions_positive_rejects_zero_step` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-016 |
| `test_dimensions_positive_rejects_zero_slot` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-016 |
| `test_verify_idempotency_key_required_empty_slots` | vb_core/action.rs tests | VB-IDEM-HYDR-017 |
| `test_verify_idempotency_unsafe_retry` | vb_core/action.rs tests | VB-IDEM-HYDR-017 |
| `test_is_resolved_completed` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-018 |
| `test_is_resolved_failed` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-018 |
| `test_is_resolved_not_scheduled` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-018 |
| `test_require_scheduled_ticket_exact_match` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-020 |
| `test_require_scheduled_ticket_ticket_mismatch` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-020 |
| `test_require_scheduled_ticket_output_mismatch` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-020 |
| `test_require_scheduled_ticket_missing` | vb_storage/recovery/tests.rs | VB-IDEM-HYDR-020 |

---

## 7. Source Reference Map

| Rust Function | Source File:Line | Proof Obligations |
|---------------|------------------|------------------|
| `compute_action_idempotency_key` | `vb_core/src/action.rs:157` | PO-VB-IDEM-001a, PO-VB-IDEM-001b, PO-VB-IDEM-001c |
| `action_ticket_has_valid_key` | `vb_core/src/action.rs:171` | PO-VB-IDEM-012a, PO-VB-IDEM-012b |
| `validate_idempotency_key_ingredients` | `vb_core/src/action.rs:347` | PO-VB-IDEM-003a, PO-VB-IDEM-009a |
| `verify_idempotency` | `vb_core/src/action.rs:391` | PO-VB-IDEM-017a |
| `hydrate_events_preconditions` | `vb_storage/src/recovery/hydrate.rs:63` | PO-VB-IDEM-011a |
| `hydrate_dimensions_positive` | `vb_storage/src/recovery/hydrate.rs:69` | PO-VB-IDEM-016a |
| `hydrate_snapshot_tail_preconditions` | `vb_storage/src/recovery/hydrate.rs:51` | PO-VB-IDEM-010a |
| `hydrate_run_frame` | `vb_storage/src/recovery/hydrate.rs` | PO-VB-IDEM-004a, PO-VB-IDEM-004b |
| `ActionReplayTracker::new` | `vb_storage/src/recovery/types.rs:404` | (trivial) |
| `ActionReplayTracker::mark_scheduled_ticket_effect` | `vb_storage/src/recovery/types.rs:413` | PO-VB-IDEM-002a, PO-VB-IDEM-007a, PO-VB-IDEM-008a |
| `ActionReplayTracker::mark_completed_envelope_effect` | `vb_storage/src/recovery/types.rs:469` | PO-VB-IDEM-005a, PO-VB-IDEM-014a, PO-VB-IDEM-015a |
| `ActionReplayTracker::require_scheduled_ticket` | `vb_storage/src/recovery/types.rs:444` | PO-VB-IDEM-020a |
| `ActionReplayTracker::is_resolved` | `vb_storage/src/recovery/types.rs:547` | PO-VB-IDEM-018a, PO-VB-IDEM-018b |
| `apply_tail_events` | `vb_storage/src/recovery/hydrate_support.rs` | PO-VB-IDEM-013a |
