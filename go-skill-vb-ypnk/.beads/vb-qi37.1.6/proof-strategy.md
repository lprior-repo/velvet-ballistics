# Proof Strategy

## Scope

- Bead: `vb-qi37.1.6`.
- State: 4 attempt 3, proof planning refresh after repaired State 3.
- Inputs read: `STATE.md`, repaired `contract.md`, repaired `proof-obligations.jsonl`, repaired `traceability-matrix.jsonl`, `delivery-scope.jsonl`, `codebase-map.md`, `tla-spec.md`, `verification-layers.md`, State 6 rejection artifacts `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`, and prior `proof-evidence.md` only as context.
- Output matrix: `.beads/vb-qi37.1.6/proof-obligations.planned.jsonl`.
- Production code, tests, proof/model/harness/spec files, dependencies, and CI configuration were not edited.

## Discovery Evidence

- `pwd -P` from isolated workspace returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6`.
- `test -s ".beads/vb-qi37.1.6/contract.md" && test -s ".beads/vb-qi37.1.6/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.1.6/delivery-scope.jsonl"` passed.
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" "crates/vb_storage/src/recovery" "crates/vb_runtime/src/recovery.rs" "crates/vb_runtime/src/primitives/collect.rs" "crates/vb_runtime/src/primitives/wait_ask.rs" "crates/vb_runtime/src/action.rs" "crates/vb_core/src/frame.rs"` returned 693 matches in 15 files.
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" "crates/vb_storage/src/recovery" "crates/vb_runtime/src/recovery.rs" "crates/vb_runtime/src/primitives/collect.rs" "crates/vb_runtime/src/primitives/wait_ask.rs" "crates/vb_runtime/src/action.rs" "crates/vb_core/src/frame.rs" "verification"` returned 437 matches in 30 files.
- Discovery confirmed existing `verification/tla/RecoveryCrashRestart.tla`, existing `verification/verus/recovery_hydration_contracts.rs`, existing generic `kani::proof` coverage in `crates/vb_core/src/frame.rs`, and no scoped `unsafe` boundary beyond `#![forbid(unsafe_code)]` declarations.
- Prior evidence remains context only: Verus previously passed locally, TLC was blocked by missing `tla2tools.jar`, and `moon run :verify-proof` was blocked by gauntlet script parsing failure. This refreshed plan does not claim any pass result.

## State 6 Rejection Repairs Reflected

- `PRE-006` is now required in Verus, integration, and mutation planning rows.
- `POST-008` expected evidence now names exact typed error variants rather than a collapsed generic error class.
- TLA planning requires transition-level modeling or reviewed narrowing for durable append, retry, wait, ask, action, snapshot, crash, and recovery actions.
- TLA execution requires the `EventuallyRecoveredOrRejected` property to be checked by config or an equivalent bounded liveness configuration.
- The canonical proof gate blocker remains explicit until `moon run :verify-proof` reaches the scoped TLA/Verus lanes.
- Verus local proof remains insufficient unless State 5 adds production-shape mapping/refinement for the abstraction.

## Risk Classification

- Temporal recovery ordering: required. Crash cuts, ordered journal replay, snapshot watermark rules, latest-attempt filtering, waits, asks, actions, collect state, and lifecycle diagnostics depend on state over time.
- Rust-local invariant correctness: required. Runtime hydration must not produce a runnable frame without complete durable facts, exact taint, bounded dimensions, fallible caller-boundary handling, and typed fail-closed errors.
- Bounded state/error classification: required as a supporting lane. `PRE-006`, frame dimensions, taint downgrade absence, and exact error variants are bounded enough for Kani/proptest/integration support if Verus does not fully bind them to production-shaped types.
- Broad generated input space: required through proptest-style obligations because example tests cannot cover event stream permutations, snapshot/tail interleavings, taint combinations, and collect extra identity/corruption.
- Fuzz/adversarial raw input: waived for this bead unless later implementation changes raw byte decoding or external parser boundaries.
- Loom/concurrency: waived unless later implementation introduces new concurrent recovery/shared-memory behavior.
- Miri/unsafe UB: not applicable for the scoped recovery files as currently discovered.
- Dependency/supply-chain: not applicable unless later states change manifests or verifier dependencies.
- Theorem kernel: waived unless proof writing discovers a small algebraic kernel not expressible in TLA+ or Verus.

## Required Lanes

- TLA+: repair/author `verification/tla/RecoveryCrashRestart.tla` and configs so transition-level durable lifecycle behavior is checked, `EventuallyRecoveredOrRejected` or equivalent is active, and raw TLC evidence is recorded. Known blocker: `java -jar tla2tools.jar ...` currently cannot access `tla2tools.jar`.
- Verus: repair/author `verification/verus/recovery_hydration_contracts.rs` or a successor artifact so `PRE-006` fallible caller-boundary handling, exact typed errors, taint exactness, dimensions, monotonicity, and no partial success are proved. Add a production-shape mapping/refinement artifact before counting abstraction evidence as production-bound.
- Kani: keep as bounded support for frame dimensions, transition limits, taint downgrade absence, and exact typed error classification when Verus does not discharge the same production-shaped branch.
- Proptest: generate bounded event streams and snapshot-tail/collect cases for determinism, monotonicity, taint downgrade resistance, and collect extra identity/corruption.
- Integration: prove actual `FjallJournal` drop/reopen restart behavior and runtime boundary fail-closed behavior.
- Mutation/deep gate: ensure exact typed-error assertions catch mutation of every named `POST-008` variant and `PRE-006` partial-success branches.
- Canonical proof gate: `moon run :verify-proof` must be repaired or correctly invoked so it reaches scoped proof artifacts and records PASS/WAIVED/BLOCKED evidence without relying on unrelated global debt.

## Reviewer Focus

- Reject if `PRE-006` disappears from Verus, integration, or mutation rows.
- Reject if any required TLA row omits active liveness/property checking for recovered-or-typed-rejected outcomes.
- Reject if TLA remains only direct-initialization booleans without transition-level mapping or explicit reviewed narrowing.
- Reject if Verus evidence is treated as production-bound without mapping `SpecRecoveryInput`, `SpecRecoverySuccess`, and `SpecRecoveryError` to recovery summary, frame seed, hydration, runtime boundary, and typed errors.
- Reject if `POST-008` expected evidence does not name `NoRecoveryData`, `CorruptSnapshot`, `ReplayDivergence`, `WorkflowSourceDigestMismatch`, `CompiledIrDigestMismatch`, `NonIdempotentActionBlocked`, `FrameDimensionOverflow`, `InvalidRecoveryHydration`, and `CollectExtraHydrationFailed`.
- Reject any claimed proof pass in planner artifacts; all executable outcomes belong to later states.

## Handoff

- `proof-obligations.planned.jsonl` is the source for proof-writer/formal-verifier routing.
- Planner statuses are `planned`, `blocked_tooling`, `waived`, or `not_applicable`; no row claims PASS.
- Rerun State 4 planning if contract clauses, scope paths, proof artifacts, gate commands, or acceptance boundary changes.
