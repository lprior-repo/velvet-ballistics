# Hazard Analysis — vb-5bqmr

**Bead**: vb-5bqmr — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)

Hazards are listed in priority order. Each hazard names: the trigger, the typed impact on runtime behavior, the detection lane(s), the mitigation, and any model bounds required for proof.

## H-001 — Silent downgrade of future-version payloads (the actual bug)

- **Category**: parser/codec, user-visible behavior.
- **Trigger**: A `SlotWrittenEvent.extra` whose first four bytes equal `b"VBSE"` and whose 5th byte is NOT `0x01` (e.g., a producer that emits `b"VBSE\x02…"`).
- **Pre-fix behavior**: `decode_slot_written_extra` returns `Ok(LegacyFrameExtra(bytes))`, which is then interpreted by `legacy_frame_extra_recovered_slot_taint` as `unsupported=true` with `taint=Taint::Secret`. The durable taint metadata from the future writer is silently discarded.
- **Post-fix behavior**: `decode_slot_written_extra` returns `Err(SlotWrittenExtraError::VersionMismatch { found })`. Recovery emits `RecoveryError::CorruptSlotTaint { slot }`; collect emits `EngineError::CollectExtraHydrationFailed { kind: VersionMismatch, .. }`.
- **Severity**: P1 (data-integrity); fail-closed.
- **Mitigation**: New `VersionMismatch` arm at the decoder + new `VersionMismatch` arm at the collect translation.
- **Lane**: Rust-local implementation (Kani + proptest) and parser/codec (fuzz — see `red-queen-strategy.md` §M3, out of scope here).

## H-002 — Magic-then-truncated input misclassified

- **Category**: parser/codec (corner case).
- **Trigger**: `bytes.len() < MAGIC.len() + 1` but `bytes[..MAGIC.len()] == MAGIC` is impossible to satisfy. The only path to "magic-but-truncated" is `bytes.len() == MAGIC.len() == 4`, which is `< 5`, so the new `version = bytes[4]` read is gated by `split_first` after the length check.
- **Behavior**: The decoder falls into the legacy arm. This is correct: the input cannot meaningfully claim to be a versioned envelope because no version byte is present.
- **Severity**: N/A (by construction).
- **Mitigation**: The `bytes.len() >= MAGIC.len() + 1` gate; proptest against inputs in `0..=4` bytes long with `bytes[..4] == MAGIC`.

## H-003 — `RecoveryError` exhaustiveness check (`recovery_unit_tests.rs:1149`) breaks

- **Category**: test/compilation hazard.
- **Trigger**: A new `RecoveryError` variant is added. The compile-time test `_exhaustive_match` at `recovery_unit_tests.rs:1149` would have to be updated.
- **Mitigation**: This bead does NOT widen `RecoveryError` (per the contract decision in `domain-model.md` §6). The compile-time exhaustiveness check is preserved unchanged.
- **Severity**: N/A (avoided by design).

## H-004 — `CollectExtraHydrationFailureKind` exhaustiveness breaks downstream callers

- **Category**: test/compilation hazard.
- **Trigger**: A new arm `VersionMismatch` is added to `CollectExtraHydrationFailureKind`. Existing `match` expressions that do not use `_ =>` would no longer compile.
- **Mitigation**: This bead assumes downstream callers either:
  - Use a `_ =>` arm (acceptable for a `#[non_exhaustive]` enum), OR
  - Are updated in lockstep.
- **Severity**: low; verify by running `cargo build --all-targets` over `vb_runtime` and `vb_core`.

## H-005 — BDD regression: legacy path (`recovery_bdd_tests.rs:3158-3211`)

- **Category**: regression / behavior.
- **Trigger**: The new decoder body could accidentally classify `b"\x01\x02\x03\x04"` differently.
- **Detection**: existing BDD scenario `typed_rejection_hydrate_from_events_slot_taint_fails_closed` uses `extra: Some(vec![0x01, 0x02, 0x03, 0x04])`. After this bead, the new decoder's first guard `bytes.split_at_checked(4)` succeeds and the magic check fails; the function returns `Ok(LegacyFrameExtra(bytes))`. NO behavior change.
- **Severity**: must remain green.

## H-006 — BDD regression: corrupt v1 envelope (`recovery_bdd_tests.rs:2507-2536` mirror)

- **Category**: regression / behavior.
- **Trigger**: The new decoder body could accidentally flip `DecodeFailed` to `VersionMismatch` for an existing `[VBSE\x01, 255, 255, 255]` test case.
- **Detection**: existing helper `corrupt_slot_taint_envelope()` in `crates/vb_storage/src/recovery/tests.rs:2332` and the helper mirror at `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:57-61` exercise the corrupt-v1 arm (`[VBSE\x01, 255, 255, 255]`).
- **Mitigation**: After this bead, `b"VBSE\x01\xff\xff\xff"` has magic + version match, so it follows the postcard path and returns `Err(DecodeFailed)`. NO behavior change.
- **Severity**: must remain green.

## H-007 — Magic constant drift between hoisted `MAGIC` and legacy `PREFIX`

- **Category**: parser/codec / constant drift.
- **Trigger**: A future maintainer changes `SLOT_WRITTEN_EXTRA_MAGIC` or `SLOT_WRITTEN_EXTRA_VERSION` but not `SLOT_WRITTEN_EXTRA_PREFIX`.
- **Mitigation**: The new `const` expression for `SLOT_WRITTEN_EXTRA_PREFIX` is compositionally derived from the two hoisted constants. A compile-time equality test asserts `SLOT_WRITTEN_EXTRA_PREFIX == &[b'V', b'B', b'S', b'E', 0x01]` and a runtime invariant test asserts `SLOT_WRITTEN_EXTRA_PREFIX.as_ptr()` cannot drift from `SLOT_WRITTEN_EXTRA_MAGIC.as_ptr()`. Concretely: a `#[test] fn prefix_constant_matches_composition()` in `slot_extra.rs::tests` asserts the equality.
- **Severity**: medium; mitigated by compile-time const + dedicated test.

## H-008 — Allocation on legacy path (negative invariant)

- **Category**: parser/codec / performance.
- **Trigger**: A naive rewrite adds a defensive allocation (e.g., `bytes.to_vec()`) in the legacy arm.
- **Mitigation**: The new body uses `bytes.split_at_checked(MAGIC.len())` and `rest.split_first()`, both zero-allocation. Kani `cover!` on a counter that records zero allocations for the legacy arm across all `len ∈ [0, MAGIC.len() + 1)` and all `magic-prefix-mismatch` inputs.
- **Severity**: N/A; the negative invariant is preserved by construction.

## H-009 — `tracing::warn!` invocations introduced — performance regression?

- **Category**: performance / observability.
- **Trigger**: Two new warn-level traces, one per `VersionMismatch` arm (recovery + collect paths).
- **Impact**: Each warn call is gated by the `VersionMismatch` arm, which (by design) is reachable only on hostile or upgraded inputs. Hot paths (v1 happy path, legacy bytes) emit zero warns.
- **Severity**: negligible; expected to be unreachable in production.

## H-010 — Concurrency hazards

- **Category**: concurrency.
- **Trigger**: None — the function is sync, pure, total, and short. Multiple threads may decode concurrently; this is by construction safe because the function holds no interior mutability or thread-local state.
- **Severity**: N/A.

## H-011 — Unsafe / undefined behaviour

- **Category**: unsafe / UB.
- **Trigger**: A naive rewrite introduces `unsafe` (e.g., for an unchecked slice read).
- **Mitigation**: `slot_extra.rs:1` already enforces `#![forbid(unsafe_code)]`. The new body uses only safe APIs (`split_at_checked`, `split_first`).
- **Severity**: N/A (forbidden by lint).

## H-012 — Bounded state and arithmetic overflow

- **Category**: arithmetic overflow.
- **Trigger**: A naive rewrite computes `MAGIC.len() + 1` at runtime; result is `5`, no overflow possible.
- **Mitigation**: The new constants are `const` and have statically-known values; `MAGIC.len() + 1` is `5` by definition.
- **Severity**: N/A.

## H-013 — API stability for downstream crates

- **Category**: public API.
- **Trigger**: Adding `SlotWrittenExtraError::VersionMismatch { found }` to a `#[non_exhaustive]` enum. Downstream `match` arms that do not use `_ =>` would break — but only if downstream code wrote an exhaustive `match`. The crate's own downstream is contained to `vb_runtime` and `vb_storage::recovery::*`; both new call sites use explicit `match`.
- **Mitigation**: `#[non_exhaustive]` already in place; documented in the bead as additive. `cargo build --all-targets` for `vb_runtime` and `vb_storage` confirms no compile-time regression.
- **Severity**: low.

## H-014 — Encoding symmetry assumption broken

- **Category**: parser/codec.
- **Trigger**: A future engineer writes a v2 encoder and forgets to update the decoder.
- **Mitigation**: This bead makes that error case explicit (`Err(VersionMismatch)` instead of silent `Ok(LegacyFrameExtra)`), so the asymmetry becomes loud at the boundary rather than silent.
- **Severity**: N/A (forward-compat hardener).

## H-015 — Cross-crate mirror helper drift

- **Category**: test/compilation.
- **Trigger**: `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:57-61` mirrors the corrupt-envelope helper. The new bead should add a sibling helper `unknown_version_envelope()` at the same location.
- **Mitigation**: Add the helper as part of the test-planner deliverable. Verify both crate roots build green.
- **Severity**: low (the mirror test will likely re-use the corrupt-payload helper unless an unknown-version path is exercised).

## H-016 — Master §47 lattice preserve invariant

- **Category**: lattice / safety.
- **Trigger**: A future rewrite drops the `legacy_frame_extra_recovered_slot_taint` "unsupported=true" classification.
- **Mitigation**: This bead does not change the lattice. `legacy_frame_extra_recovered_slot_taint` retains `unsupported: true` and `taint: Taint::Secret`. The DecodeFailed path remains `Err(CorruptSlotTaint{slot})`.
- **Severity**: low (preserved by construction; verified by `cargo build --all-targets`).

## H-017 — Recursive / re-entrant decode

- **Category**: control flow.
- **Trigger**: A pathological rewrite calls `decode_slot_written_extra` from within itself or a callback that mutates the source buffer.
- **Mitigation**: The function takes `&[u8]` by value and reads only; no callback. The caller does NOT mutate the source while the borrow is alive (borrow checker enforces).
- **Severity**: N/A.

## Hazard summary table

| ID | Severity | Lane needed (proposed) | Mitigation in this bead |
|---|---|---|---|
| H-001 | P1 (behavior) | Kani + proptest + fuzz (fuzz out-of-scope) | VersionMismatch arm + exhaustive match at call sites |
| H-002 | N/A | proptest | length-checked split |
| H-003 | N/A (avoided) | none | no new `RecoveryError` variant |
| H-004 | low | cargo build | downstream matches exhaustively |
| H-005 | regression | BDD scenario | test_unchanged in `delivery-scope.jsonl` |
| H-006 | regression | unit | test_unchanged |
| H-007 | medium | compile-time const + #[test] | compositional const + equality test |
| H-008 | N/A | Kani `cover!` | zero-allocation legacy arm |
| H-009 | negligible | performance smoke | warn only on `VersionMismatch` |
| H-010 | N/A | none | sync, pure |
| H-011 | N/A | source-lint | `#![forbid(unsafe_code)]` |
| H-012 | N/A | none | const arithmetic |
| H-013 | low | cargo build | `#[non_exhaustive]` retained |
| H-014 | forward | Kani | explicit rejection |
| H-015 | low | unit + workspace test | sibling helper added |
| H-016 | low | proptest | lattice path preserved |
| H-017 | N/A | none | borrow-only |

## Modeling / proof-plant hints

These hints are SEEDS for the proof-planner; final lane decisions belong to that agent.

- **Rust-local implementation**: Kani bounds over `len ∈ [0..64]` covering the three discriminator arms (v1, unknown-version, legacy).
- **Hostile input**: this is a parser/codec surface. `cargo-fuzz` would be ideal but is out of scope; `proptest` is the in-scope alternative.
- **Concurrency**: N/A.
- **Unsafe / provenance**: N/A; `#![forbid(unsafe_code)]` covers.
- **Performance**: branch-prediction only; proptest `cover!` on allocation counter.
- **Release**: API-additive only; no version bump.
