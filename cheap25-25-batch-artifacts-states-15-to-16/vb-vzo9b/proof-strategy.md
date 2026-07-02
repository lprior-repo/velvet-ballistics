# Proof Strategy: vb-vzo9b Recovery Decode Exact-Field Pin

## Bead

- **bead_id**: vb-vzo9b
- **title**: Tests: replace multi-run recovery disjunction with exact slots (P1 bug)
- **scope**: `fuzz/src/journal_target/readback.rs` lines 183-204 (`fuzz_recovery_decode`)
- **production_touched**: false (no `crates/**` files, no `Cargo.toml`)
- **behavior_affecting (bead-level)**: false — test-only repair; production behavior is unchanged
- **state**: 4 (proof-planner)
- **base**: State 3 contract `C-vb-vzo9b`, 6 proof seeds `PS-vb-vzo9b-1..6`, 8 traceability rows

## Strategy Summary

The defect is a disjunctive `assert!(run_summary.run == run || run_summary.run == RunId::new(0))` at `fuzz/src/journal_target/readback.rs:196`. The OR-disjunction accepts a sentinel value (`RunId::new(0)`) that is bound to the empty-events `RecoveryError::NoRecoveryData` variant in production and is therefore never a valid `RecoveryRuntimeSummary.run` for non-empty events.

The fix replaces the disjunctive assertion with an `assert_eq!(run_summary, expected_recovery_runtime_summary)` that pins all 11 fields of `RecoveryRuntimeSummary` simultaneously (the struct already derives `Debug, Clone, Copy, PartialEq, Eq` at `crates/vb_storage/src/recovery/types.rs:546`).

This is a **test-only** repair. Production code (`summarize_recovery_events`, `recover_runtime_frame_seed_from_events`, `RecoveryRuntimeSummary`, `RecoveryHydration`, multi-run guards, and overflow sentinel guards) is read-only context. The blast radius is one fuzz body (22 lines including signature) and its re-export chain (`fuzz/src/journal_target.rs`, `fuzz/src/lib.rs`, `fuzz/src/bin/recovery_decode.rs`).

No new types, no new error variants, no `unsafe`, no `unwrap`/`expect`/`panic` outside the desired `assert_eq!` panic. The behavior under change is **test** behavior, not production behavior. Per the bead description (`Behavior: false`), proof obligations are planned with `behavior_affecting: false`.

## Risk Profile

| Risk Class | Source Tags | In Scope? | Notes |
|------------|-------------|-----------|-------|
| `equality` (field_sensitivity) | behavior-test, exact-pin, sentinel-collision | yes | `assert_eq!` over all 11 fields of `RecoveryRuntimeSummary`; sentinel `RunId::new(0)` rejection via single struct equality |
| `rejection` (typed-error-rail) | behavior-test, typed-error-rail | yes (carried by C-3) | empty-events branch unchanged: `assert_typed_recovery_error` |
| `noop` (blast-radius-control) | behavior-test, noop | yes (carried by C-4) | frame-seed call unchanged |
| source-lint (forbidden-pattern) | behavior-test, forbidden-pattern, blast-radius-control, source-lint | yes | grep over `readback.rs` for reintroduction of `assert!(... || ...)`; build-gate for compile |
| bounded_state / arithmetic_overflow / index_safety / panic_freedom / illegal_state / refinement / concurrency_interleaving / cancellation_safety / shutdown_drain / temporal_liveness / temporal_safety / ub_safety / hostile_input / ordering / parse_canonicalization | (none) | no | Production is unchanged; no new state machine surface, no arithmetic change, no index operation, no panic surface, no refinement introduction, no concurrency, no unsafe, no hostile-input boundary added. |

The default Rust behavior profile (`verus`, `kani`, `flux-rs`, `proptest`) is **not** mandated for this bead because:

1. The defect is in test code (`fuzz/src/journal_target/readback.rs`), not production.
2. The contract cluster is `recovery_decoder_exact_field_pin` — the test contract is being tightened, not the production contract.
3. The proof-seeds explicitly note that proptest/cargo-test is sufficient because the production derivation is constant in this fuzz payload shape; no Verus/Kani/Flux refinement is needed.
4. Per `proof-seeds.jsonl` notes for PS-vb-vzo9b-1: "Pure exact-pin claim over an existing Copy + PartialEq + Eq struct. Proptest over the seed (digest, run, seq) triple is straightforward but redundant with fuzz coverage; cargo-test exact-pin is sufficient. No Verus/Kani/Flux refinement needed because the production derivation is constant in this fuzz payload shape."

The bead explicitly restricts the lane profile to **cargo-test** and **source-lint** (per `STATE.md`/`dispatch` and the user's directive: `Lanes: cargo-test, source-lint. 2-3 obligations.`).

## Lane Strategy

### Required Lanes

| Lane | Tool | Purpose |
|------|------|---------|
| **cargo-test** | `cargo test -p vb_storage --lib <unit-name>` | Targeted unit-test gates on the production recovery functions called by the fuzz harness. Must remain green before/after the fuzz assertion rewrite. |
| **cargo-build** | `cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` | Compile gate on the rewritten fuzz body. The new `assert_eq!(run_summary, expected_recovery_runtime_summary)` must compile against `RecoveryRuntimeSummary`'s `Debug, Clone, Copy, PartialEq, Eq` derive set. Note: the fuzz/ directory is a separate Cargo workspace with package name `velvet-ballistics-fuzz`; the contract C-7 entry `cargo build -p fuzz --bin recovery_decode` is corrected to use `--manifest-path` so the build runs against the correct package from any workdir. |
| **source-lint** | `grep -rn 'assert!([^)]* || ' fuzz/src/journal_target/readback.rs` (must return zero matches) | Source-lint gate against the reintroduction of the disjunctive pattern or any of the C-8 forbidden patterns. |

### Non-Applicable Default-Profile Lanes

| Verifier | Reason | Evidence |
|----------|--------|----------|
| `verus` | Defect is in test code, not production. No new Rust-local invariant to model. Existing production `summarize_recovery_events` is unchanged. | `proof-seeds.jsonl` PS-vb-vzo9b-1 note; `contract.md` C-5 (production read-only); `delivery-scope.jsonl` row 7 (`crates/vb_storage/src/recovery/replay/summary/apply.rs` touched=false). |
| `kani` | Defect is in test code, not production. No new bounded symbolic claim. | `proof-seeds.jsonl` PS-vb-vzo9b-1, -3, -6 notes; `delivery-scope.jsonl` row 28 (`kani` required=false). |
| `flux-rs` | Defect is in test code, not production. No new refinement type introduction. | `delivery-scope.jsonl` row 30 (`flux` required=false). |
| `loom` | Test code is single-threaded (no async, no threads, no atomics, no channels). | `fuzz/src/journal_target/readback.rs` has no `tokio`, `crossbeam`, `std::sync::*`, or `Send`/`Sync` boundary in scope; fuzz harness is a synchronous function reading stdin bytes. |
| `miri` | All touched files carry `#![forbid(unsafe_code)]`; zero `unsafe` blocks, no FFI, no raw pointers, no MaybeUninit, no provenance-sensitive operations in the fuzz harness or the production recovery surface called by it. | `fuzz/src/journal_target/readback.rs` uses safe Rust only (no `unsafe` keyword); `crates/vb_storage/src/recovery/replay/summary/apply.rs` uses safe Rust only. |
| `cargo-fuzz` | The fuzz harness body is the target of the change, not a separate fuzz target. The fuzz harness is built via `cargo build -p fuzz --bin recovery_decode` and run via libfuzzer. The repair does not introduce a new fuzz target or boundary; it tightens the assertions inside the existing one. `cargo build -p fuzz --bin recovery_decode` covers the build, and the assertion-strengthening is covered by the source-lint grep + cargo-test gates. | `fuzz/Cargo.toml:241-246` registers `recovery_decode` bin; the closure commands in `contract.md` C-7 explicitly list `cargo build -p fuzz --bin recovery_decode` and the two `cargo test` invocations, not a fuzz run. |

### TLA+

Removed per the global proof-planner skill mandate (temporal workflows use loom + proptest). No temporal surface in scope.

## Trust Markers

This bead introduces **no** trust markers. The `assert_eq!(run_summary, expected_recovery_runtime_summary)` is a plain Rust assertion over a struct that derives `PartialEq + Eq + Copy + Debug`. No `assume`, no `axiom`, no `admit`, no `external_body`, no `#[trusted]`, no `#[ignore]`, no `extern_spec`, no `opaque`, no model reduction.

The trusted-base-plan.md notes are therefore structural (documenting pre-existing trusted components the plan depends on) rather than obligation-driven.

## Waiver Candidates

No behavior-affecting waiver candidates. The proof obligations are all `behavior_affecting: false` because this is a TEST-ONLY repair and production code is unchanged. Per `references/waiver-planning-guide.md`, the only waivable obligations are non-behavior trust markers; we have none.

A single `waiver-candidate/v1` row is emitted for the empty waiver set as a structural placeholder (analogous to `vb-b8i8f`'s `WC-001`).

## Implementation Bridge Preparation

This bead has **no `behavior_affecting: true` proof obligations**, so the bridge (`proof-to-implementation`) does not need to materialize `rust-refinement-obligation/v1` rows. The bridge input (`proof-to-implementation-input.md`) records:

- The three planned proof obligations (PO-001, PO-002, PO-003).
- The exact production source symbols each obligation's `target` references (for traceability and review):
  - PO-001 → `crates/vb_storage/src/recovery/replay/summary/apply.rs::summarize_recovery_events`
  - PO-002 → `crates/vb_storage/src/recovery/replay/summary/derive.rs::recover_runtime_frame_seed_from_events`
  - PO-003 → `fuzz/src/journal_target/readback.rs::fuzz_recovery_decode` (compile gate) + grep over the same file (forbidden-pattern gate)
- The fuzz body rewrite's exact code shape (one literal `assert_eq!` per the contract C-1).
- The independent behavior test surface is the production unit tests themselves; the rewrite does not add a new `#[test]`.

The downstream implementer (holzman-rust, state 5) reads the bridge input to apply the `readback.rs:196` rewrite. The closure gate is `cargo build -p fuzz --bin recovery_decode && cargo test -p vb_storage --lib summarize_recovery_events && cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` (C-7).

## Self-Audit Checklist

- [x] Every (requirement_id, contract_clause, proof_seed_id, verifier) tuple in the default profile has exactly one lane decision; default-profile verifiers are `not_applicable` with `non_applicability_evidence_refs` containing SHA-256 hashes of `contract.md`, `delivery-scope.jsonl`, and `codebase-map.md`.
- [x] Every `required` lane decision has at least one paired `proof-obligation/v1` ID, and the obligation exists in `proof-obligations.planned.jsonl`.
- [x] No `blocked_tooling` row advances past State 4.
- [x] All `decision_reason` strings cite concrete `risk_tags` and avoid the weak vocabulary.
- [x] All `not_applicable` rows have a typed `limitation_kind`.
- [x] No two rows duplicate `(requirement_id, contract_clause, proof_seed_id, verifier)` with conflicting `applicability`.
- [x] No behavior-affecting waiver candidates.
- [x] All proof obligations have `behavior_affecting: false` (test-only repair per bead directive).
- [x] All proof obligations have non-empty `expected_evidence` with concrete tool markers (`test result: ok`, `Compiling recovery_decode ... Finished`).
- [x] All proof obligations have absolute `workdir` paths.
- [x] All proof obligations have non-empty `target` in `path::symbol` form.
- [x] `status: planned` for every obligation (planner never claims `PASS`).
- [x] `mode: verify-proof` for behavior-equivalent gates; cargo-test gates run actual unit tests, not smoke-only commands.
- [x] `owner_state: 4`, `rerun_from: 4` on every obligation.