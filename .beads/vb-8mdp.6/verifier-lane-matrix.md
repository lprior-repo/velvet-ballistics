# Verifier Lane Matrix — Idempotency Hydration

## Bead: vb-8mdp.6

Maps each proof seed to its assigned verifier lanes.

| Proof Seed | Kani | TLA+ | Verus | Proptest | Miri | Flux | Loom | Fuzz | Decision Rationale |
|------------|------|------|-------|----------|------|------|------|------|-------------------|
| PS-VB-IDEM-001 | ✅ | ✅ | — | ✅ | ❌ NA | — | — | ❌ NA | Kani exhausts collision space; TLA+ proves determinism; proptest generates random triples |
| PS-VB-IDEM-002 | ✅ | — | ✅ | — | ❌ NA | — | — | — | Kani verifies tracker key independence; Verus refines state machine |
| PS-VB-IDEM-003 | ✅ | ✅ | — | — | — | ⚠️ WAIVED | — | — | Kani + TLA+ cover taint validation; Flux waived pending dedicated effort |
| PS-VB-IDEM-004 | ✅ | ✅ | — | — | — | — | ❌ NA | — | Kani verifies error paths; TLA+ models atomicity |
| PS-VB-IDEM-005 | ✅ | ✅ | — | — | — | — | — | — | Kani verifies digest comparison; TLA+ invariant |
| PS-VB-IDEM-006 | ✅ | ✅ | — | — | — | — | — | — | Kani verifies seq ordering; TLA+ invariant |
| PS-VB-IDEM-007 | ✅ | — | ✅ | — | — | — | ❌ NA | — | Kani verifies blocking; Verus proves is_resolved monotonicity |
| PS-VB-IDEM-008 | ✅ | — | — | — | — | — | — | — | Kani verifies divergence detection |
| PS-VB-IDEM-009 | ✅ | — | — | — | — | ⚠️ WAIVED | — | — | Same as PS-VB-IDEM-003 — Flux waived |
| PS-VB-IDEM-010 | ✅ | — | — | — | — | — | — | — | Kani verifies preconditions combination |
| PS-VB-IDEM-011 | ✅ | — | — | — | — | — | — | — | Kani verifies empty/non-empty |
| PS-VB-IDEM-012 | ✅ | — | — | ✅ | — | — | — | — | Kani verifies key comparison; proptest generates tickets |
| PS-VB-IDEM-013 | ✅ | — | — | — | — | — | — | — | Kani verifies seq ordering in apply_tail_events |
| PS-VB-IDEM-014 | ✅ | — | — | — | — | — | — | — | Kani verifies envelope divergence |
| PS-VB-IDEM-015 | ✅ | — | — | — | — | — | — | — | Kani verifies already-resolved path |
| PS-VB-IDEM-016 | ✅ | ✅ | — | — | — | — | — | — | Kani verifies dimension bounds; TLA+ models |
| PS-VB-IDEM-017 | ✅ | — | — | — | — | — | — | — | Kani verifies MissingKey error |
| PS-VB-IDEM-018 | ✅ | — | ✅ | — | — | — | — | — | Kani verifies is_resolved; Verus refines |
| PS-VB-IDEM-019 | ✅ (cargo) | — | — | — | — | — | — | — | cargo check enforces boundary |
| PS-VB-IDEM-020 | ✅ | — | — | — | — | — | — | — | Kani verifies exact match |

## Legend

- ✅ = Required lane
- ❌ NA = Not applicable (concrete evidence cited)
- ⚠️ WAIVED = Waiver candidate with rationale
- — = Not required for this proof seed

## Waiver Summary

| Proof Seed | Waiver Reason |
|------------|---------------|
| PS-VB-IDEM-003 | Flux refinement for slot taint validation is a separate effort. Kani covers taint rejection paths (SecretInKey, RandomInKey, TimeInKey). TLA+ models no-secret-in-key invariant. |
| PS-VB-IDEM-009 | Same as PS-VB-IDEM-003. |

## Not-Applicable Summary

| Proof Seed | Reason |
|------------|--------|
| PS-VB-IDEM-001 (Miri) | Wrapping arithmetic is defined behavior. No raw pointers or undefined behavior. |
| PS-VB-IDEM-002 (Miri) | No unsafe code in ActionReplayTracker. All HashMap/HashSet operations are safe Rust. |
| PS-VB-IDEM-007 (Loom) | Single-threaded sequential recovery. ActionReplayTracker is not shared across threads. |
| PS-VB-IDEM-004 (Loom) | Hydration is single-threaded. No concurrent state modifications. |
| PS-VB-IDEM-001 (Fuzz) | Kani exhausts bounded input space. Fuzzing adds no value over exhaustive checking for a pure function with bounded inputs. |

## Existing Verification Coverage

| Artifact | Description | Coverage |
|----------|-------------|----------|
| `verification/tla/IdempotencySafety.tla` | TLA+ model of idempotency and replay safety | PS-VB-IDEM-001, 002, 003, 005, 006, 007, 008, 014 |
| `verification/tla/RecoveryHydration.tla` | TLA+ model of hydration protocol | PS-VB-IDEM-004, 010, 016 |
| `verification/verus/idempotency_replay_tracker.rs` | Verus refinement proofs | PS-VB-IDEM-002, 007, 018 |
| `verification/verus/vb_rpch_action_replay_tracker.rs` | Verus state machine proofs | PS-VB-IDEM-007, 018 |
| `verification/flux/vb_rpch_flux_r8.rs` | Flux ActionReplayTracker surface | PS-VB-IDEM-002 (partial) |
| `verification/flux/vb_rpch_flux_r9.rs` | Flux ActionReplayTracker surface | PS-VB-IDEM-002 (partial) |
| `crates/vb_storage/src/kani_recovery_hydrate.rs` | Kani harnesses | PS-VB-IDEM-001 (partial), 004, 005, 006, 007, 008, 010, 011, 012, 013, 014, 015, 016, 017, 018, 020 |
