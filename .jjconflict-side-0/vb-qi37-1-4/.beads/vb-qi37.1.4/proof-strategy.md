# Proof Strategy — vb-qi37.1.4

## Bead
- **ID**: vb-qi37.1.4
- **Title**: runtime/recovery: Fail closed on incomplete recovery
- **Phase**: 4 (proof-planning)
- **Risk**: critical (recovery safety)

---

## Scope Summary

The bead closes a gap in the runtime recovery boundary: `UnsupportedRecoveryState::action_payloads` was not checked in `reject_unsupported_live_frame_state`, allowing action payloads to be consumed from a frame that should have been rejected. The bead also wires in `DigestCheck::Full` for action ABI digests and policy digests in `verify_digests`.

**Delivery scope touches**: `vb_runtime/src/recovery.rs`, `vb_storage/src/recovery/recover.rs`, `vb_storage/src/recovery/types.rs`

---

## Risk Classification

| Risk Tag | Description | Severity |
|---|---|---|
| `recovery_safety` | Fail-closed boundary violation | **critical** |
| `persistence` | Journal durability, snapshot decode | high |
| `parser_codec` | Codec roundtrip for recovery types | high |
| `concurrency` | HashSet in ActionReplayTracker | medium |

**Decision**: All `recovery_safety` clauses require formal proof. No waivers for safety-critical obligations.

---

## Verifier Lane Selection

### Lane 1: Verus (primary — Rust-local invariants)

**Rationale**: INV-RC-001 through INV-RC-005 are Rust-local boolean conditions on `RecoveryFrameSeed::unsupported` fields. `reject_unsupported_live_frame_state` is a pure function — directly expressible in Verus with `spec_reject_unsupported_live_frame_state`.

**Targets**:
- `crates/vb_runtime/src/recovery.rs::reject_unsupported_live_frame_state` — 4 flag checks
- `crates/vb_runtime/src/recovery.rs::DurableFrameRecoveryBoundary::hydrate_run_frame` — postcondition
- `crates/vb_storage/src/recovery/recover.rs::verify_digests` — action ABI and policy digest checks

**Proof obligations**: 9 Verus proof obligations covering all fail-closed invariants and postconditions.

**Shell exclusions**: Fjall journal I/O, snapshot decode, wall-clock time, network.

### Lane 2: TLA+ (protocol — event replay lifecycle)

**Rationale**: INV-RC-007 (RunResumed/RunRetried/RunAnswered not silently dropped) is a temporal/state-machine property — best validated by TLC model checking.

**Target**: `specs/RecoveryReplay.tla` (written from `tla-spec.md`)

**Prerequisite**: `RecoveryReplay.tla` and `RecoveryReplay.cfg` must be written as actual files before TLC can run.

**Proof obligations**: 2 TLC invariants (SafeHydration, LifecycleEventsNotDropped) + 2 temporal properties.

**Bounded model**: `pending_actions ∈ 0..10`, `replay_buf ∈ 0..20`, symmetry disabled for UnsupportedState (16 combos).

### Lane 3: Integration Tests (gap detection)

**Rationale**: The primary gap (`action_payloads` not checked) is a behavioral bug in source code. Integration tests demonstrate the gap exists before fix and closes after fix.

**Targets**:
- `crates/vb_storage/tests/recovery_integration.rs`
- `crates/vb_runtime/src/recovery.rs` (in-module tests)

**Proof obligations**: 6 integration test obligations.

### Lane 4: Kani (codec roundtrip)

**Rationale**: PRE-RC-001 requires that `RecoveryFrameSeed` (including `UnsupportedRecoveryState`) roundtrips correctly through snapshot encoding/decoding.

**Target**: `crates/vb_storage/src/kani_codec.rs`

**Proof obligations**: 1 Kani obligation.

### Lane 5: Loom (concurrency)

**Rationale**: `ActionReplayTracker` uses `HashSet` internally. Concurrent access during replay needs systematic permutation testing.

**Status**: **waived** — `ActionReplayTracker` is in storage replay, non-critical path for fail-closed boundary. Compensating evidence: integration tests cover concurrent event ordering.

---

## Waiver Summary

| Obligation | Lane | Reason | Compensating Evidence |
|---|---|---|---|
| Concurrent HashSet in ActionReplayTracker | loom | Non-critical for fail-closed boundary | Integration tests on event ordering |
| Lean/Hax theorem kernel | theorem | UnsupportedRecoveryState is 4-bool struct; all clauses directly Verus-expressible | Verus proofs for all critical clauses |

---

## Proof Ordering (Critical Path)

```
1. Write RecoveryReplay.tla + RecoveryReplay.cfg  (TLA+ prerequisite)
2. Run Verus on vb_runtime/src/recovery.rs       (verus-obligations: 1–5, 8, 9)
3. Run Verus on vb_storage/src/recovery/recover.rs (verus-obligations: 6–7)
4. Run TLC on RecoveryReplay.tla                  (tla-obligations: 10–11)
5. Run integration tests                          (integ-obligations: 12–17)
6. Run Kani on kani_codec.rs                     (kani-obligation: 18)
```

---

## Key Decisions

1. **Verus owns all Rust-local clauses** — including INV-RC-006 (action_payloads not consumed), which was initially marked as Verus-owned in `verification-layers.md`.

2. **TLA+ is blocked on spec file creation** — `specs/RecoveryReplay.tla` and `specs/RecoveryReplay.cfg` must be written from `tla-spec.md` before TLC can run. The proof-writer must produce these files.

3. **Integration tests are gap-detection not gap-closure** — INTEG-RC-GAP-001/002/003 expect tests to FAIL on current source (demonstrating the gap) and PASS after the fix is applied.

4. **No Miri required** — all recovery source files use `#![forbid(unsafe_code)]`, eliminating UB risk in the proof boundary.

5. **Proptest lane not activated** — INV-RC-001 through INV-RC-004 are pure boolean combinators; exhaustive unit tests cover them adequately. Proptest would add no additional coverage for this scope.

---

## Artifact Targets

| Artifact | Path | Writer |
|---|---|---|
| RecoveryReplay.tla | `specs/RecoveryReplay.tla` | proof-writer |
| RecoveryReplay.cfg | `specs/RecoveryReplay.cfg` | proof-writer |
| verus-report.md | `.beads/vb-qi37.1.4/verus-report.md` | verus run |
| tla-report.md | `.beads/vb-qi37.1.4/tla-report.md` | tlc run |
| test-output.md | `.beads/vb-qi37.1.4/test-output.md` | cargo test run |
| kani-report.md | `.beads/vb-qi37.1.4/kani-report.md` | cargo kani run |
