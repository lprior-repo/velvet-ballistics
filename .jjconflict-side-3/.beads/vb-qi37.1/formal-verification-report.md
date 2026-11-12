# Formal Verification Report: vb-qi37.1

STATUS: APPROVED

## Inputs

- `.beads/vb-qi37.1/proof-obligations.jsonl`: valid JSONL.
- `.beads/vb-qi37.1/traceability-matrix.jsonl`: valid JSONL.
- `.beads/vb-qi37.1/delivery-scope.jsonl`: valid JSONL.
- `.beads/vb-qi37.1/contract-verification-review.md`: `STATUS: APPROVED`.

## Command Evidence

- TLA+: `JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp TMPDIR=target/tmp tlc -metadir target/tmp/tlc-review-rerun-metadir-2 -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla`; exit 0; `Model checking completed. No error has been found`; `10740192 states generated`; `8405208 distinct states found`; depth `7`.
- Verus: `mkdir -p target/tmp && TMPDIR=target/tmp verus verification/verus/recovery_verification.rs`; exit 0; `verification results:: 17 verified, 0 errors`.
- Scoped tests/manual QA: storage, runtime, workspace recovery, and recovery proptest commands passed as recorded in `machine-gate-report.md`.
- Static scan/source gates: `moon run :fmt`, `moon run :lint-src`, `moon run :check`, `moon run :source-length`, `moon run :test`, and `moon run :bench-build` passed.

## Waivers

- Optional action ABI and policy digest mismatch obligations remain waived downstream until production exposes action ABI/policy digest input, lookup, and comparison surfaces.
- Optional Kani/Flux/Loom/Miri/fuzz/theorem/dependency lanes remain approved non-required waivers from the contract and proof-plan artifacts.

## Rollup Blockers

- `moon ci` failed before running tasks because Git could not resolve `main` in this jj workspace.
- `moon run :verify-proof` failed because `scripts/rust-verification-gauntlet.sh` is malformed as shell. Exact verifier commands were run directly and passed.

## Decision

- Every required bead-local obligation is accounted as `PASS` in `verification-ledger.jsonl`.
- Every non-required optional obligation is accounted as `WAIVED` with approved waiver metadata.
