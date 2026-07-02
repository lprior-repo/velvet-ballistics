# Proof-to-Implementation Input: vb-vzo9b

## Bridge Purpose

This document prepares the State 7 `proof-to-implementation` bridge with the mapping from planned proof obligations to Rust source references, behavior test references, and refinement harness references. The bridge agent will use this to create `rust-refinement-obligation/v1` rows IF any behavior-affecting obligations exist.

**Critical**: All three planned proof obligations (`PO-001`, `PO-002`, `PO-003`) are `behavior_affecting: false` per the bead's `Behavior: false` directive. The bridge therefore does **not** need to materialize `rust-refinement-obligation/v1` rows. This document records the source refs, behavior test surface, and the closure commands so the State 11 `formal-verifier` can run the planned commands against the post-fix fuzz body.

## Proof Obligation → Source Mapping

| Obligation ID | Verifier | Production Target | Source File | Symbol |
|---------------|----------|-------------------|-------------|--------|
| PO-001 | proptest (cargo-test) | summarize_recovery_events | `crates/vb_storage/src/recovery/replay/summary/apply.rs` | `summarize_recovery_events` (L88-129); tested in `crates/vb_storage/src/recovery/replay/summary/tests.rs` |
| PO-002 | proptest (cargo-test) | recover_runtime_frame_seed_from_events | `crates/vb_storage/src/recovery/replay/summary/derive.rs` | `recover_runtime_frame_seed_from_events` (L69); multi-run guard at `crates/vb_storage/src/recovery/replay/summary/accumulator.rs:86`; tested in `crates/vb_storage/src/recovery/replay/summary/tests.rs` |
| PO-003 | proptest (cargo-build + source-lint) | fuzz_recovery_decode | `fuzz/src/journal_target/readback.rs` | `fuzz_recovery_decode` (L183-204); defect site L196; re-exported at `fuzz/src/journal_target.rs:30-33` and `fuzz/src/lib.rs:46`; bin entry at `fuzz/Cargo.toml:241-246` (`recovery_decode`) |

## Behavior Test Surface

The independent behavior test surface for this bead is the **existing** production unit-test suite. No new `#[test]` is added; the fuzz harness is the test target. The downstream implementer (holzman-rust, state 5) does not add a `#[test]`; only rewrites the fuzz body.

| Behavior Test (pre-existing) | Path | Purpose |
|-------------------------------|------|---------|
| `summarize_recovery_events_empty_returns_exact_no_recovery_data` | `crates/vb_storage/src/recovery/replay/summary/tests.rs:285-302` | Empty-events path: returns `RecoveryError::NoRecoveryData { run: RunId::new(0) }` exactly. |
| `summarize_recovery_events_*` (multi-event) | `crates/vb_storage/src/recovery/replay/summary/tests.rs` | Multi-event same-run summary; exact field pins over `RecoveryRuntimeSummary`. |
| `summarize_recovery_events_overflow_seq_rejected` (or equivalent) | `crates/vb_storage/src/recovery/replay/summary/tests.rs` | Overflow-sentinel rejection via `RecoveryError::ReplayDivergence { detail: "overflow sentinel sequence N is not valid" }`. |
| `frame_seed_empty_events_returns_exact_no_recovery_data` | `crates/vb_storage/src/recovery/replay/summary/tests.rs:285-302` (paired with the summary variant) | Empty-events frame-seed path. |
| `frame_seed_*` (multi-event) | `crates/vb_storage/src/recovery/replay/summary/tests.rs` | Multi-event same-run frame-seed; exact field pins over the inner `RecoveryRuntimeSummary`. |

These pre-existing tests are run by `cargo test -p vb_storage --lib summarize_recovery_events` and `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events`. They are independent of the fuzz harness and unchanged by this bead.

## Refinement Harness Refs

**Not applicable.** This bead has zero `behavior_affecting: true` obligations. Per `proof-schemas.md` (rust-refinement-obligation/v1), refinement obligations are required only for behavior-affecting proof claims. Since this is a TEST-ONLY repair, the bridge does not materialize `rust-refinement-obligation/v1` rows.

## Required Code Change (Implementation)

The downstream implementer (holzman-rust, state 5+) applies **one** change at `fuzz/src/journal_target/readback.rs:196`:

```rust
// BEFORE (defect):
assert!(run_summary.run == run || run_summary.run == vb_core::RunId::new(0));

// AFTER (C-1 exact pin over all 11 fields):
let expected_recovery_runtime_summary = vb_storage::recovery::RecoveryRuntimeSummary {
    run,
    first_seq: seq,
    last_seq: seq,
    workflow: Some(digest),
    steps_started: 0,
    steps_succeeded: 0,
    actions_scheduled: 0,
    actions_resolved: 0,
    suspensions: 0,
    slots_written: 0,
    terminal: None,
};
assert_eq!(run_summary, expected_recovery_runtime_summary);
```

The local construction of `expected_recovery_runtime_summary` is byte-equivalent to the values derived by `summarize_recovery_events` for a single `RunAccepted` event with `seq = EventSeq::new(1)` (see `codebase-map.md §2.3` field table and `contract.md` C-1).

The frame-seed call site (lines 201-203) is byte-identical pre/post fix per contract C-4; the implementer MUST NOT modify it.

The empty-events branch is byte-identical pre/post fix per contract C-3; the implementer MUST NOT modify it.

## Forbidden Patterns (C-8) — MUST NOT APPEAR POST-FIX

| Pattern | Why forbidden |
|---------|---------------|
| `assert!(... \|\| ...)` over `RecoveryRuntimeSummary` fields | Reintroduces the disjunctive defect. |
| `matches!(summary, RecoveryRuntimeSummary { run, .. })` | Only checks `run`; misses the other 10 fields. |
| Field-by-field `assert!(...)` chain (e.g., 11 separate `assert!`) | Brittle, easy to drop a field. |
| `let _summary = ...;` | Coverage-only fuzz target; defeats behavior checking. |
| `dbg!(...)` instead of `assert_eq!` | Failure mode is silent. |
| `unwrap()` / `expect()` on `RecoveryResult` | Disallowed by Holzman Rust; also wrong type (used on the typed-error rail). |

## Closure Gate (C-7)

The State 11 `formal-verifier` runs the three commands in order:

```bash
# 1. Compile gate (note: fuzz/ is a separate workspace; use --manifest-path)
cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml

# 2. Production unit-test gate (1)
cargo test -p vb_storage --lib summarize_recovery_events

# 3. Production unit-test gate (2)
cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events
```

The forbidden-pattern grep gate (PO-003's secondary command) is run by the formal verifier as part of PO-003's evidence collection; it is not a separate `verification-ledger/v1` row.

`moon ci` is canonical but not strictly required for this bead (deferred to landing per bead workflow, `delivery-scope.jsonl`).

## Bridge Contractor Notes

- State 7 `proof-to-implementation` does NOT create `rust-refinement-obligation/v1` rows because all three proof obligations are `behavior_affecting: false`.
- The bridge DOES record the source refs above for traceability and for the State 11 verifier.
- `mapping_status: planned` is allowed at State 7; must be `materialized` by State 11 (implementation) and `verified` by State 12 (closure). For this bead, "materialized" means: the fuzz body rewrite is applied; "verified" means: the three closure commands return exit 0 with the expected markers in their stdout.
- The downstream implementer reads the proof obligations and applies the one change at `readback.rs:196`.
- No public API addition, no Cargo.toml change, no production code change is required.
- Pre-existing tests in `crates/vb_storage/src/recovery/replay/summary/tests.rs` and `crates/vb_storage/src/recovery/vb_h6ix_tests.rs` MUST remain green and MUST NOT be modified by this bead.