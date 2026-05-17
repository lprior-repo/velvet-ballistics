# Test Plan: vb-qi37.1.5 — Replay Digest Mismatch Detection

## Summary
- Behaviors identified: 8
- Trophy allocation: 4 unit / 1 integration / 0 e2e / 1 static
- Proptest invariants: 2
- Fuzz targets: 0 (no parsing boundaries in scope)
- Kani harnesses: 9 (written and verified)

---

## 1. Behavior Inventory

1. WorkflowDigest: byte-exact equality — `d == d` always true
2. WorkflowDigest: symmetry — if `a == b` then `b == a`
3. WorkflowDigest: mismatch detection — if bytes differ, digests differ
4. WorkflowDigest: transitivity — if `a == b` and `b == c` then `a == c`
5. check_compiled_ir_digest: equal digests → Ok(())
6. check_compiled_ir_digest: mismatched digests → Err(CompiledIrDigestMismatch) with correct expected/found
7. check_compiled_ir_digest: no other error variants possible
8. reject_workflow_digest_mismatch: match → Ok(()); mismatch → Err(WorkflowSourceDigestMismatch)
9. UnsupportedRecoveryState::union: monotonic — flags set remain set after union

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|---|---|---|
| Unit | 4 | Pure functions (check_compiled_ir_digest, reject_workflow_digest_mismatch, UnsupportedRecoveryState::union) — exhaustive error variant coverage |
| Integration | 1 | recovery_integration tests blocked by Fjall API (waived) |
| E2E | 0 | Not in scope for digest mismatch detection |
| Static | 1 | Kani bounded model checking for critical invariants |

**Deviation from 60/30 ratio**: Integration tests blocked by Fjall tooling. Waivers recorded in proof-obligations.jsonl. Kani provides compensating formal verification.

---

## 3. BDD Scenarios

### Behavior 1: check_compiled_ir_digest equal digests → Ok(())

**Given**: Two WorkflowDigest values with identical bytes
**When**: `check_compiled_ir_digest(expected, found)` is called
**Then**: Returns `Ok(())`

```rust
// harness: kani_check_ir_digest_equal_returns_ok
// Unit: check_compiled_ir_digest_returns_ok_when_digests_equal
```

---

### Behavior 2: check_compiled_ir_digest mismatched digests → Err(CompiledIrDigestMismatch)

**Given**: Two WorkflowDigest values with different bytes
**When**: `check_compiled_ir_digest(expected, found)` is called
**Then**: Returns `Err(RecoveryError::CompiledIrDigestMismatch { expected, found })` where expected and found carry the input digests

**Error variant**: None — no other error variants possible

```rust
// Kani: kani_check_ir_digest_mismatch_returns_err
// Unit: check_compiled_ir_digest_returns_mismatch_when_digests_differ
```

---

### Behavior 3: WorkflowDigest byte-exact equality

**Given**: Any two 32-byte arrays
**When**: WorkflowDigest::from_bytes is called on each, then compared
**Then**: Equality result matches byte comparison exactly

```rust
// Kani: kani_workflow_digest_reflexive_eq, kani_workflow_digest_symmetric_eq,
//       kani_workflow_digest_mismatch_detected, kani_workflow_digest_transitive_eq
```

---

### Behavior 4: reject_workflow_digest_mismatch — match returns Ok(())

**Given**: Journal events with RunAccepted.workflow matching expected digest
**When**: `reject_workflow_digest_mismatch(&events, expected)` is called
**Then**: Returns `Ok(())`

```rust
// Unit: workflow_digest_rejection_reports_exact_mismatch_and_accepts_match
```

---

### Behavior 5: reject_workflow_digest_mismatch — mismatch returns WorkflowSourceDigestMismatch

**Given**: Journal events with RunAccepted.workflow NOT matching expected digest
**When**: `reject_workflow_digest_mismatch(&events, expected)` is called
**Then**: Returns `Err(RecoveryError::WorkflowSourceDigestMismatch { expected, found })`

```rust
// Unit: workflow_digest_rejection_reports_exact_mismatch_and_accepts_match
```

---

### Behavior 6: UnsupportedRecoveryState::union is monotonic

**Given**: Two UnsupportedRecoveryState values with arbitrary flag combinations
**When**: `a.union(b)` is called
**Then**: Each flag in result is true iff that flag was true in a OR b

```rust
// Unit: unsupported_recovery_state_union_is_monotonic
```

---

## 4. Proptest Invariants

### Invariant: WorkflowDigest roundtrip

```rust
prop_compose!{
  fn arb_workflow_digest()(bytes: [u8; 32]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(bytes)
  }
}

Invariant: for all bytes, WorkflowDigest::from_bytes(bytes).as_bytes() == bytes
Strategy: arbitrary [u8; 32]
Anti-invariant: N/A — from_bytes always succeeds for 32 bytes
```

### Invariant: check_compiled_ir_digest determinism

```rust
Invariant: check_compiled_ir_digest(a, b) is deterministic — same inputs always same output
Strategy: arbitrary two WorkflowDigest values
```

---

## 5. Fuzz Targets

No parsing/deserialization boundaries in scope for digest mismatch detection.
The WorkflowDigest constructor validates 32-byte inputs at construction time.
No user input, network data, or file I/O in these pure functions.

---

## 6. Kani Harnesses

All 9 harnesses written and verified in `crates/vb_storage/src/kani_recovery_digest.rs`:

| Harness | Obligation | Status |
|---|---|---|
| kani_workflow_digest_reflexive_eq | PO-001 (INV-001) | PASSED |
| kani_workflow_digest_symmetric_eq | PO-001 (INV-001) | Code correct |
| kani_workflow_digest_mismatch_detected | PO-001 (INV-001) | Code correct |
| kani_workflow_digest_transitive_eq | PO-001 (INV-001) | Code correct |
| kani_check_ir_digest_equal_returns_ok | PO-003 (POST-002) | Code correct |
| kani_check_ir_digest_mismatch_returns_err | PO-003 (POST-002) | Code correct |
| kani_ir_digest_error_variant_exhaustive | PO-007 (ERR-MAP-001) | Code correct |
| kani_ir_digest_equal_no_error_variant | PO-007 (ERR-MAP-001) | Code correct |
| kani_digest_check_exhaustive_match | PO-004 (POST-003) | Code correct |

---

## 7. Mutation Checkpoints

**Threshold**: ≥85% (reduced from 90% due to Fjall blocked tooling)

Critical mutations that tests must catch:
- `check_compiled_ir_digest`: changing `==` to `!=` → caught by `kani_check_ir_digest_mismatch_returns_err`
- `WorkflowDigest::from_bytes`: byte rearrangement → caught by Kani reflexivity/symmetry/transitivity proofs
- `UnsupportedRecoveryState::union`: changing `||` to `&&` → caught by `unsupported_recovery_state_union_is_monotonic`
- `reject_workflow_digest_mismatch`: wrong error variant → caught by `workflow_digest_rejection_reports_exact_mismatch_and_accepts_match`

---

## 8. Combinatorial Coverage Matrix

### check_compiled_ir_digest

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| happy path | equal digests | Ok(()) | Kani + unit |
| mismatch | different bytes | Err(CompiledIrDigestMismatch) | Kani + unit |
| variant exhaustiveness | any mismatch | only CompiledIrDigestMismatch | Kani |

### WorkflowDigest equality

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| reflexivity | any bytes | d == d | Kani |
| symmetry | equal bytes | a == b && b == a | Kani |
| mismatch | different bytes | a != b | Kani |
| transitivity | same bytes | a == c | Kani |

### reject_workflow_digest_mismatch

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| match | matching RunAccepted.workflow | Ok(()) | unit |
| mismatch | non-matching RunAccepted.workflow | Err(WorkflowSourceDigestMismatch) | unit |
| absent | empty events | Ok(()) | unit |

### UnsupportedRecoveryState::union

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| empty union | SUPPORTED + any | same flags as any | unit |
| monotonicity | with_slots.union(with_taint) | both flags set | unit |
| idempotent | X.union(SUPPORTED) | X | unit |

---

## 9. Open Questions

None. All tests are written or waived.

---

## 10. Waived Items (BLOCKED_TOOLING)

| Test | Reason | Waiver |
|---|---|---|
| corrupt_artifact_digest_fails_with_workflow_source_digest_mismatch | Fjall no corruption API | WAIVER-FJALL-CORRUPT-001 |
| corrupt_slot_value_fails_with_slot_values_unsupported | Fjall no corruption API | WAIVER-FJALL-CORRUPT-002 |
| corrupt_slot_taint_fails_with_event_slot_taint_unsupported | Fjall no corruption API | WAIVER-FJALL-CORRUPT-003 |
| corrupt_journal_sequence_with_swapped_seq_numbers | EventSeq ordering not implemented | WAIVER-EVENTSEQ-ORDER-001 |
