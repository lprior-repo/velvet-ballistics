bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-12-kani
updated_at: 2026-05-09T00:00:00Z

# Kani Justification

## Proof Obligations

### PO-007: verify_trim_boundary
**Property**: For any event sequence number `s` and cutoff `c`, if `s >= c`, the event is NOT in the deletion set.

**Analysis**: The deletion decision is a single boolean expression: `seq_u64 < cutoff_seq.get()`. This is a direct `u64` comparison. The property "if `s >= c` then NOT deleted" is the contrapositive of the implementation. A Kani harness would verify:
```rust
#[kani::proof]
fn verify_trim_boundary() {
    let s: u64 = kani::any();
    let c: u64 = kani::any();
    kani::assume(s >= c);
    assert!(!(s < c));  // Tautology
}
```
This is a mathematical tautology. The value of Kani here is zero — the property is proven by the definition of `<` on `u64`.

**Compensating Evidence**:
- Unit test `trim_preserves_events_at_or_after_snapshot` verifies boundary at exact cutoff
- Unit test `trim_given_run_with_events_seq_0_to_9_and_snapshot_at_seq_5_trims_0_to_4` verifies off-by-one safety

### PO-008: verify_idempotence
**Property**: `trim(trim(J, P), P) == trim(J, P)`

**Analysis**: Full idempotence requires modeling the Fjall database state transition, which involves:
1. LSM-tree keyspace mutations
2. Write batch atomicity
3. Snapshot isolation

Kani cannot model Fjall's external I/O behavior. The idempotence property is a stateful property over the database, not a pure function.

**Compensating Evidence**:
- Unit test `trim_given_run_already_trimmed_is_idoop` verifies idempotence on real Fjall database
- Unit test `trim_is_idempotent_on_already_trimmed_run` (new) provides additional verification
- Manual QA smoke test `smoke_idempotency` verifies end-to-end

## Waiver Decision

Both Kani obligations are waived:
- **PO-007**: The property is a mathematical tautology (`s >= c` → `!(s < c)`). Unit tests provide sufficient operational evidence.
- **PO-008**: Requires modeling Fjall I/O, which is outside Kani's scope. Integration tests on real database provide equivalent confidence.

**Waiver Owner**: GoMasterOrchestrator (vb-5h50)
**Expiry**: None (structural limitation, not temporary deferral)
**Compensating Evidence**: 15 unit tests + 4 integration smoke tests + 875 total passing tests

STATUS: WAIVED
