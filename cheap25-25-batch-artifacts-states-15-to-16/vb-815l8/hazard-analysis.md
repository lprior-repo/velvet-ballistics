# Hazard Analysis — vb-815l8

- bead_id: vb-815l8
- scope: TEST-ONLY; one-line assertion + one-line import + comment cleanup at `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:7-13, 75-79`
- authored_at: 2026-07-01

## 1. Hazard Categories (in scope)

This bead addresses exactly one hazard class: **tautological assertions in tests** that mask production-code regressions. The P1 severity comes from the fact that the assertion cannot fail regardless of whether the runtime boundary correctly rejects the seed, correctly returns `Ok(frame)`, panics, or returns an unrelated error variant.

## 2. Hazards Directly Mitigated By This Bead

### H-001 (P1, behavior-affecting): Tautological assertion masks boundary regression

- **Source:** `crates/workspace_tests/tests/integration_runtime_fault_tolerance.rs:79` — `assert!(result.is_ok() || result.is_err())`.
- **Symptom:** Test passes regardless of boundary behavior. A regression that:
  - Silently returns `Ok(frame)` for invalid seeds (worst case — silent runtime corruption).
  - Returns `Err(RuntimeError::Core { source })` (wrong error type — runtime cannot distinguish recovery failure from core failure).
  - Panics inside `hydrate_run_frame` (worst case — process crash).
  - Becomes a no-op (test passes despite the assertion being dead code).
  
  All four regressions are silently masked by the current assertion.

- **Mitigation:** Replace with `assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration), "...")`. The replacement:
  - Discriminates `Ok(_)` from `Err(InvalidRecoveryHydration)`.
  - Discriminates between `RuntimeError` variants via unit-tag `PartialEq`.
  - Discriminates against panic outcomes (a panic fails the test by panic, not by assertion; the `cargo test` summary surfaces it).

- **Risk tags:** `user-visible-behavior`, `public_api`, `persistence`, `parser/codec`.

### H-002 (P2, documentation-affecting): Comments contradict production code

- **Source:** `integration_runtime_storage_fault_tolerance.rs:75-78`:
  - Line 75-76: "Hydration should succeed because the seed itself is valid (corrupt snapshot is a storage-layer concern; the boundary only validates the seed shape)."
  - Line 78: "A seed with step_count=0 and no workflow may still be a valid empty-run seed."
  
- **Symptom:** Future maintainer reading the comments may conclude that the boundary is permissive and that the test was intentionally weak. They may:
  - Add more `assert!(result.is_ok() || result.is_err())` patterns elsewhere.
  - Refactor the boundary to be permissive, expecting the test to keep passing.
  - Mis-document the `RecoveryResumeStatus::CannotResume` invariant in adjacent code.

- **Mitigation:** Replace both comment blocks with a comment that references the production-code invariant (`RecoveryResumeStatus::CannotResume`, `recovery.rs:41-50`) and the storage-layer `from_seed` mark (`storage/types.rs:949-957`).

- **Risk tags:** `documentation`, `public_api`, `persistence`.

### H-003 (P3, scoping-affecting): Test name implies a different test body

- **Source:** `recovery_from_corrupt_snapshot_sequence_is_detected` (`integration_runtime_storage_fault_tolerance.rs:46`) — the name implies storage-corrupt-snapshot detection, but the body asserts boundary behavior on a manually-constructed seed.

- **Symptom:** The test name is a misleading indicator of intent. The replacement assertion matches the actual body behavior, not the name's promise.

- **Mitigation:** Out of scope for this bead (renaming the test is a behavior-test rewrite, not a contract assertion fix). Flagged in `domain-model.md` §8 and `contract.md` for downstream `test-writer` review.

- **Risk tags:** `documentation`, `testing-trophy`.

## 3. Hazards NOT Mitigated By This Bead (out of scope)

### H-004 (P3): Out-of-scope tautological assertions in adjacent files

- **Source:** `codebase-map.md` §7 lists five other tautological assertions:
  - `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:{238,263,286}`
  - `crates/vb_core/src/verification/kani/kani_choose_replay.rs:337` (Kani `cover!`, not a behavior assertion)
  - `crates/vb_compile/tests/vb_xi2f_compile_source_proptest.rs:176`
  - `crates/vb_core/src/budget/tests.rs:1306`
  - `crates/vb_compile/src/kani_foreach_parity.rs:471`
  
- **Status:** Covered by other beads per `codebase-map.md` §7. NOT in scope for vb-815l8.

### H-005 (P3): Source-length exception drift

- **Source:** The test file is 359 lines (vs. the 346-line baseline in `source-length-exceptions.txt:200`). One added `use` and one replaced assertion do not change the exception status.

- **Status:** No action required for this bead. Flagged in `domain-model.md` §8 Q3.

### H-006 (P2, design-affecting): Test intent mismatch (storage-corrupt-snapshot vs. boundary-rejection)

- **Source:** The test name implies `RecoveryError::CorruptSnapshot { run, seq }` from `recover_runtime_frame_seed_from_events`, but the body exercises the runtime boundary on a manually-built seed.

- **Status:** Out of scope for this bead. Flagged in `domain-model.md` §8 Q1 for `test-writer` follow-up.

## 4. Temporal Hazards

None for this bead. The test is single-shot, deterministic, and has no scheduling, ordering, or interleaving surface.

## 5. Concurrency Hazards

None. The test does not spawn tasks, does not share state, and does not exercise any async boundary. The recovery boundary is synchronous.

## 6. Unsafe / Provenance Hazards

None. `crates/vb_runtime/src/recovery.rs:1` and the test file line 1 both have `#![forbid(unsafe_code)]`.

## 7. Hostile-Input Hazards

None. The test fixture is a controlled, manually-constructed `RecoveryFrameSeed`. There is no parser, no untrusted input, no fuzz/proptest surface introduced or exercised.

## 8. Performance / Release Hazards

None. The replacement assertion adds zero runtime overhead vs. the tautological assertion (both compile to a single boolean check + `panic!` call on failure; the typed `assert_eq!` is slightly more expensive on failure because it formats the `Debug` payload, but no failure is expected).

## 9. Refinement Hazards

None. The replacement assertion is the most-refined possible form for a unit-variant `RuntimeError` equality check.

## 10. Open Hazard Questions

None. All in-scope hazards (H-001, H-002) are mitigated by the single-line assertion replacement plus comment cleanup. H-003 is flagged but out of scope.