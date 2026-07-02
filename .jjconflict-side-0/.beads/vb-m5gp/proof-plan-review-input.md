# Proof Plan Review Input: vb-m5gp

## Review Request

Review the State 4 proof plan for bead `vb-m5gp`. The plan converts the accepted pure-refactor contract into executable gates. It intentionally does not add proof code, tests, production code, dependencies, or CI config.

Attempt 3 repair scope: fix exact obligation commands rejected in State 11 only. No production/test/proof artifacts changed.

## Contract Summary

- Pure refactor only: split `crates/vb_compile/src/lib.rs` into private modules `mod_compile_core`, `mod_compile_errors`, `mod_compile_validation`, and `mod_compile_lowering`.
- Preserve crate-root public API, signatures, cfg gates, error variants, diagnostic codes, behavior, generated artifacts, digest, and idempotency gate outcomes.
- Do not promise public `compile`, `lower`, `validation`, or `mod_compile_*` module paths.
- Do not blindly wire stale unwired scaffolding under `crates/vb_compile/src/{compile,lower,validation}`.
- No dependency, feature, config, benchmark, or performance claim change.

## Planned Required Gates

- Workspace/path/input guard.
- Dependency/config diff guard.
- `cargo +nightly fmt --all --check`.
- `cargo +nightly clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings` (strict source lint per repository governance; test clippy is not strict).
- `cargo +nightly test -p vb_compile --all-targets --all-features`.
- Selected workspace compile/error integration tests under actual package `velvet-ballistics-workspace-tests`.
- `moon ci` rollup.
- Static source review/checks for facade/private modules, no new public internal modules, acyclic module dependencies, stale scaffolding disposition, visibility leakage, forbidden constructs, and source length.
- Kani idempotency parity: `cargo kani --package vb_compile --harness idempotency_gate_parity --quiet`.

## Non-Applicable / Waived Lanes for Review

- TLA+: non-applicable because the bead has no temporal workflow/protocol/concurrency/lifecycle behavior. A TLA model would be vacuous.
- Loom: non-applicable because no concurrency primitives or scheduling behavior are in scope.
- Verus: waived only while implementation remains a pure move; semantic changes must return to State 3.
- Lean/Aeneas/Hax: non-applicable because no theorem-critical kernel is introduced.
- Miri: planned as optional deep evidence, not a required blocker, because this is a behavior-free structural move and `vb_compile` is already `unsafe_code = forbid`; waiver requires compensating clippy/test/Kani/moon evidence.

## Review Questions

1. Do all planned rows map to a contract clause or traceability entry?
2. Are any required lanes too weak for public API/behavior parity?
3. Is the TLA+ non-applicability rationale acceptable, or is there an actual temporal risk hidden in the refactor?
4. Is the Kani command sufficiently repository-supported by existing `scripts/rust-verification-gauntlet.sh` command style and in-crate harness declaration?
5. Should Miri be promoted to required for this refactor, or is optional/deep with compensating evidence acceptable?
6. Are the attempt 3 exact-command repairs acceptable: `workspace_tests` package references corrected to `velvet-ballistics-workspace-tests`, and `STATIC-001` aligned to source-only strict clippy because test clippy is not strict under repository governance?

## Reviewer Inputs

- `.beads/vb-m5gp/proof-strategy.md`
- `.beads/vb-m5gp/proof-obligations.planned.jsonl`
- `.beads/vb-m5gp/contract.md`
- `.beads/vb-m5gp/verification-layers.md`
- `.beads/vb-m5gp/traceability-matrix.jsonl`
- `.beads/vb-m5gp/tla-spec.md`
- `.beads/vb-m5gp/lean-contract.md`
- `.beads/vb-m5gp/formal-verification-report.md`
- `.beads/vb-m5gp/regression-diff.md`
- `.beads/vb-m5gp/ci-failure-category.txt`
