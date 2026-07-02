# Verification Layers — vb-rpch

## Boundary

- **Verus-owned kernel**: Pure Rust-local recovery invariants — `UnsupportedRecoveryState::union`, `ActionReplayTracker` monotonicity, `DigestCheck` hierarchy, dimension bounds (u16), seed construction, replay attempt filtering, frame hydration postconditions
- **TLA+ temporal model**: Journal replay sequence ordering, snapshot-plus-tail causal consistency, incomplete-run discovery, non-idempotent blocking, digest verification order — `specs/RecoveryReplay.tla`
- **Theorem projection**: None required (Verus sufficient for all Rust-local pure obligations)
- **Runtime shell**: Fjall journal I/O, wall-clock durability, snapshot encoding/decoding, `RunFrame` construction from seed
- **External systems excluded from formal proof**: Fjall internal LSM-tree storage, OS filesystem sync, async task scheduling

---

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layers | Notes |
|---|---|---|---|
| PRE-001 (hydrate_run_frame preconditions) | verus | kani | Dimension bound checks, seq ordering |
| PRE-002 (hydrate_run_frame_from_events preconditions) | verus | kani | Non-empty events, dimension bounds |
| PRE-003 (check_workflow_source_digest) | verus | tla-plus | Verus models journal event filtering |
| PRE-004 (recover_runtime_summary/frame_seed preconditions) | verus | kani | Non-empty events invariant |
| PRE-005 (replay preconditions) | verus | kani | Non-null tracker, seq ordering |
| POST-001 (workflow digest result) | verus | tla-plus | Verus models digest comparison; TLA+ models journal event loop |
| POST-002 (IR digest result) | verus | tla-plus | Same split as POST-001 |
| POST-003 (verify_digests all-level) | verus | tla-plus | GAP-3: ActionAbiMismatch/PolicyDigestMismatch not reachable |
| POST-004 (summary counts) | verus | tla-plus | Counts derived from events; TLA+ models ExtractTerminal |
| POST-005 (frame seed fields) | verus | tla-plus | Verus models seed field invariants; TLA+ models ReplayEvent |
| POST-006 (hydrate_run_frame result) | verus | kani | RunFrame reconstruction postconditions |
| POST-007 (hydrate_run_frame_from_events result) | verus | kani | Same as POST-006 |
| POST-008 (incomplete runs) | tla-plus | kani | TLA+ models DiscoverIncomplete |
| POST-009 (replay_events result) | verus | tla-plus | Verus models attempt filtering; TLA+ models ReplayEvent |
| POST-010 (tracker is_resolved) | verus | kani | Monotonicity invariant |
| INV-001 (RecoveryError distinctness) | static-scan | test | Enum variants are exhaustive and distinct |
| INV-002 (UnsupportedRecoveryState union) | verus | proptest | Algebraic properties |
| INV-003 (seed dimension invariants) | verus | kani | step_count > 0, slot_count > 0 |
| INV-004 (tracker monotonic) | verus | kani | Once resolved, always resolved |
| INV-005 (DigestCheck hierarchy) | verus | test | Enum ordering invariant |
| INV-006 (only incomplete runs) | tla-plus | kani | TLA+ models DiscoverIncomplete |
| ERR-Journal | test | static-scan | Integration test + clippy |
| ERR-WorkflowSourceDigestMismatch | test | tla-plus | BDD B-001b |
| ERR-CompiledIrDigestMismatch | test | tla-plus | BDD B-001c |
| ERR-ActionAbiMismatch | waiver | — | GAP-3: not reachable, vb-ty9 pending |
| ERR-PolicyDigestMismatch | waiver | — | GAP-3: not reachable, vb-ty9 pending |
| ERR-NonIdempotentActionBlocked | test | tla-plus | BDD B-007 |
| ERR-ReplayDivergence | test | kani | BDD B-008 |
| ERR-NoRecoveryData | test | — | BDD B-004 |
| ERR-CorruptSnapshot | test | — | BDD B-005, B-012 |
| ERR-TerminalStateMismatch | waiver | — | DEFERRED_GLOBAL: no public API parameter |
| ERR-FrameDimensionOverflow | test | kani | BDD B-011 |
| GAP-3 ActionAbiMismatch | waiver | — | vb-ty9 |
| GAP-3 PolicyDigestMismatch | waiver | — | vb-ty9 |
| Durability Strict profile | test | performance | BDD + moon run :perf |
| Durability Journaled profile | test | — | BDD coverage |
| Durability Relaxed profile | test | — | BDD coverage |

---

## Verus Scope

### Rust Target: `crates/vb_storage/src/recovery/types.rs` + `crates/vb_storage/src/recovery/replay/core.rs` + `crates/vb_storage/src/recovery/hydrate.rs`

**Spec/Proof Functions**:
- `spec fn union_invariant(self, other: Self) -> bool` — union produces valid state (no contradictory flags)
- `proof fn union_commutative(a, b)` — `a.union(b) == b.union(a)`
- `proof fn union_associative(a, b, c)` — `a.union(b).union(c) == a.union(b.union(c))`
- `proof fn union_idempotent(a)` — `a.union(a) == a`
- `proof fn union_no_contradiction(a, b)` — `!(a.slot_values && b.slot_values && !(a.slot_values || b.slot_values))`
- `spec fn tracker_resolved_invariant(tracker, key)` — monotonic state machine
- `proof fn tracker_mark_completed_preserves_monotonicity(tracker, action, step)`
- `proof fn tracker_mark_failed_preserves_monotonicity(tracker, action, step)`
- `spec fn attempt_filter_invariant(events, max_attempt)` — state-affecting events only from max_attempt
- `proof fn replay_events_respects_attempt_filter(events)`
- `spec fn seed_dimension_invariants(seed: RecoveryFrameSeed) -> bool` — step_count > 0, slot_count > 0
- `proof fn seed_construction_preserves_dimensions(events)`

**Invariants**:
- `UnsupportedRecoveryState::SUPPORTED` has all four bools false
- `ActionReplayTracker::is_resolved` monotonic
- `DigestCheck` hierarchy: `WorkflowSourceOnly < WorkflowAndIr < Full`

**Trusted Boundary**:
- `vb_core::RunFrame::new`, `write_slot_with_taint`, `set_pc`, `increment_executed`, `mark_*` — trusted runtime frame construction
- Fjall journal event storage — trusted external durability boundary
- `postcard` decoding of snapshot bytes — trusted codec boundary

**Shell Exclusions**:
- Fjall journal I/O (read events, write events, snapshots)
- Wall-clock time and durability profiles (SyncAll)
- Async task scheduling

**Evidence Command**: `verus crates/vb_storage/src/recovery/types.rs crates/vb_storage/src/recovery/replay/core.rs crates/vb_storage/src/recovery/hydrate.rs`

---

## TLA+ Scope

### Module/Model Path: `specs/RecoveryReplay.tla`

**Variables**:
- `Events: Seq of JournalEvent`
- `Snapshots: [run -> Snapshot | None]`
- `Tracker: [action_step -> {"unresolved", "completed", "failed"}]`
- `TerminalFlags: [run -> {"none", "cancelled", "finished", "failed"}]`
- `DigestCheckLevel`

**Actions**: `Init`, `ReplayEvent`, `CheckDigest`, `SnapshotPlusTail`, `ExtractTerminal`, `DiscoverIncomplete`, `BlockNonIdempotent`

**Safety Invariants**:
- `ReplaySeqOrder` — seq non-decreasing per attempt, steps monotonic per attempt
- `TailCausalAfterSnapshot` — tail seq > snapshot seq
- `OnlyIncompleteRuns` — DiscoverIncomplete only returns non-terminal runs
- `NoResolvedReExecution` — resolved action+step never reappears
- `DigestVerificationOrder` — workflow before IR digest

**Temporal Properties**:
- `EventuallyTerminalOrRecoverable` — every non-terminal run reaches terminal or incomplete set
- `EventuallyAllDigestsVerified` — at Full level, all digests verified or errored

**Fairness/Deadlock Stance**:
- Weak fairness on `ReplayEvent` and `DiscoverIncomplete`
- Deadlock freedom required: model never stalls with incomplete work remaining

**Refinement Boundary**:
- Rust `recover_runtime_summary` ↔ TLA+ `ExtractTerminal` (counts extracted from same event set)
- Rust `recover_runtime_frame_seed` ↔ TLA+ `ReplayEvent` (seed fields derived from events)
- Rust `replay_events` ↔ TLA+ `ReplayEvent` (attempt filtering, tracker updates, non-idempotent blocking)
- Rust `recover_all_incomplete_runs` ↔ TLA+ `DiscoverIncomplete`
- Rust `verify_digests` ↔ TLA+ `CheckDigest`

**Evidence Command**: `tlc -config specs/RecoveryReplay.cfg specs/RecoveryReplay.tla`

---

## Waivers

| Clause | Reason | Owner | Expiry |
|---|---|---|---|
| GAP-3 ActionAbiMismatch verification | Lookup function not implemented; GAP-3 tracked in vb-ty9 | vb-ty9 owner | vb-ty9 |
| GAP-3 PolicyDigestMismatch verification | Lookup function not implemented; GAP-3 tracked in vb-ty9 | vb-ty9 owner | vb-ty9 |
| TerminalStateMismatch public API | No expected-terminal parameter in public APIs; DEFERRED_GLOBAL in BDD B-017 | vb-oewy owner | TBD |
