# Verifier Lane Matrix: vb-uwxct

## Lane Applicability Matrix

| Proof Seed | Contract Clause | cargo-test | kani | source-lint | Verus | Flux-rs | Loom | Miri | cargo-fuzz |
|------------|-----------------|------------|------|-------------|-------|---------|------|------|------------|
| ps-vb-uwxct-000 (anchor: production encoder contract) | C0 | required (ref) | — | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| ps-vb-uwxct-001 (lex-ordering) | C1 | required | — | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| ps-vb-uwxct-002 (seq roundtrip) | C2 | required | — | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| ps-vb-uwxct-003 (always-17-bytes) | C3 | required | — | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| ps-vb-uwxct-004 (correct-prefix) | C4 | required | — | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| ps-vb-uwxct-005 (different-runs-prefix) | C5 | required | — | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| ps-vb-uwxct-006 (same-run-diff-seq) | C6 | required | — | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| ps-vb-uwxct-007 (kani harness typed Err) | C7 | — | required | required | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |

## Applicability Legend

- **required**: Mandatory verifier lane for this seed; bound to at least one proof obligation.
- **not_applicable**: Lane does not apply; concrete evidence and `non_applicability_evidence_refs` recorded in `verifier-lane-decisions.jsonl`.
- **—** (in kani column for C1..C6, in cargo-test column for C7): the seed does not require this lane, so the cell is intentionally blank; the lane is recorded as `not_applicable` with evidence in the JSONL.

## Non-Applicability Evidence Summary

| Lane | Reason | Evidence Ref |
|------|--------|--------------|
| Verus (all seeds) | No production Rust code is changed in this test-only repair. Creating a Verus proof obligation would be VACUUM (GOD RULE 2) — there is no production `exec fn` to bind to. | `crates/vb_storage/src/keys.rs:480-496` is reference-only per `contract.md` §2; `delivery-scope.jsonl` row `vb-uwxct.scope.contract.cluster` action=`reference_only`; `proof-planner` skill "Production Binding Plan" mandates STRONG/WEAK_MIRROR/WEAK_EXTERN — none apply here. |
| Flux-rs (all seeds) | No production Rust refinement target is changed. | Same as Verus; `crates/vb_storage/src/keys.rs:480-496` action=`reference_only`. |
| Loom (all seeds) | No concurrency surface is introduced or removed. The repair only narrows proptest `u64` ranges to `0u64..u64::MAX` (a static data-domain shrink) and replaces one `match` arm in a `#[cfg(kani)]` harness. | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` is a `proptest!` block with no `tokio`, `crossbeam`, `Arc<Mutex>`, or `std::sync::mpsc` usage; `crates/vb_storage/src/kani_typed_partitioned_ids.rs:1 #![cfg(kani)]` is a single-threaded symbolic harness. |
| Miri (all seeds) | No unsafe surface is introduced. All touched files retain `#![forbid(unsafe_code)]`. | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:2 #![forbid(unsafe_code)]`; `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` is a `proptest!` block with safe Rust only. |
| cargo-fuzz (all seeds) | No new parser/codec surface is introduced. The production encoder is unchanged; the test-only repair does not add a new decode/encode path. | `crates/vb_storage/src/keys.rs:480-496` is reference-only; existing `fuzz/` targets already exercise the encoder path; no new fuzzer harness is added. |
| kani (seeds 001-006) | The six proptests are not Kani targets. Kani applies only to the harness in `kani_typed_partitioned_ids.rs` (seed 007). | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` is a `proptest!` block, not a Kani harness; the Kani harness is at `crates/vb_storage/src/kani_typed_partitioned_ids.rs:111-115` and binds only to seed 007. |
| cargo-test (seed 007) | The Kani harness is `#[cfg(kani)]`-gated and is not executed by `cargo test` (it is build-stripped). Kani has its own harness probe. | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:1 #![cfg(kani)]`; the harness compiles under kani only and `cargo test` skips it. |

## Lane Co-Existence Notes

- cargo-test and kani do not conflict: the six proptests live in a `proptest!` block
  at `restate_journal_tail_scan_fallback_tests.rs:1305-1450`; the Kani harness lives
  in a separate `#[cfg(kani)]` file. Both are executed in the same `moon ci` cycle
  without compile-time interference.
- source-lint is a cross-cutting gate: `bash scripts/forbidden-scan.sh` and
  `cargo clippy --workspace --all-targets -- -D warnings` cover all
  seven specimens. The repair must not introduce new `unwrap`/`expect`/`panic`/
  `assert!(false)`/`[T]::last()`/unchecked indexing.
- Kani's `cargo kani` invocation is the package-level Kani probe (not `--lib` or
  `--test`); per the runbook in `AGENTS.md` and the `kani-list.sh` script
  (`scripts/kani-list.sh`), the harness is registered via `kani-list.json` and
  executed by harness name.

## Required-Lane Counts

| Lane | Required lanes (per seed) | Total |
|------|---------------------------|-------|
| cargo-test | 7 (one per seed, plus anchor C0) | 7 |
| kani | 1 (seed 007) | 1 |
| source-lint | 8 (all seeds) | 8 |
| Verus | 0 (test-only repair, no production change) | 0 |
| Flux-rs | 0 | 0 |
| Loom | 0 | 0 |
| Miri | 0 | 0 |
| cargo-fuzz | 0 | 0 |

## Cross-Reference

- `verifier-lane-decisions.jsonl` — one row per (seed, verifier) tuple with
  `applicability` and `non_applicability_evidence_refs`.
- `proof-coverage-matrix.md` — requirement-to-obligation traceability table.
- `proof-obligations.planned.jsonl` — concrete obligation rows with command,
  expected_evidence, owner_state, rerun_from, status, and waiver (if any).