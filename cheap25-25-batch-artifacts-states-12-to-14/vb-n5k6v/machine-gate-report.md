# Machine Gate Report — vb-n5k6v

> Bead-level summary of machine-executed gates. Sourced from `.beads/vb-n5k6v/evidence/*.txt` and the formal-verification-report.md gate tables.

- bead_id: `vb-n5k6v`
- state: 12 (formal-verification) — synthesized for state-14 evidence-packaging gate consumption
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v`
- production_fix_commit: `womqwkks 84a5eb7d` (vb-n5k6v: rust-contract artifacts (orphaned edge_case_tests wiring, P1 test-only repair))
- parent_commit: `rsvywymk 1d6c017f` (AGENTS.md round10 forward-port)

## Cargo Gates (User-Narrowed Scope)

| Gate | Command | Exit | Evidence |
|------|---------|------|----------|
| Rust unit tests — `edge_case` (CC-WIRE-004) | `PROPTEST_CASES=1 cargo test -p vb_storage --lib edge_case` | 0 | `evidence/cargo-test-edge-case.txt` — `cargo test: 26 passed, 1530 filtered out`; `dispatch/state-12-formal-verifier/command-logs/cargo_test_vb_storage_lib_edge_case.log` (raw, 2.4 KB; SHA-256 `8fb5ca90d2b5f2526df3d376d252cc86b836dae40f10e2c0feab0748a56daeab`) |
| Rust unit tests — full lib (CC-WIRE-005) | `PROPTEST_CASES=1 cargo test -p vb_storage --lib` | 0 | `evidence/post-wire-test-count.txt` — `cargo test: 1556 passed (1 suite, 1.36s)`; `dispatch/state-12-formal-verifier/command-logs/cargo_test_vb_storage_lib.log` (raw, 124.3 KB; SHA-256 `3ec4e1f9609f9f6592769f8d12adc95d93ca7cb3c8205653e19982d1b1c4a26f`) |
| Rust unit tests — pre-wire baseline | `PROPTEST_CASES=1 cargo test -p vb_storage --lib` (at parent commit `rsvywymk`) | 0 | `evidence/pre-wire-test-count.txt` — `cargo test: 1530 passed (1 suite, 0.95s)` (2026-07-01 direct-execution capture) |
| Cargo check (build-only) | `PROPTEST_CASES=1 cargo check -p vb_storage --tests` | 0 | `evidence/cargo-check-vb-storage-tests.txt` — `cargo build (1 crates compiled) Finished `dev` profile ... in 1.05s`; `dispatch/state-12-formal-verifier/command-logs/cargo_check_vb_storage_tests.log` (raw, 72 B; SHA-256 `bb4fb9f557cc03354a3b4f724e3c34dcb33d49b89cde353cb67511e662ae9e28`) |
| Cargo clippy (source target, strict) | `PROPTEST_CASES=1 cargo clippy -p vb_storage --lib -- -D warnings` | 0 | `evidence/cargo-clippy-vb-storage-lib.txt` — `cargo clippy: No issues found`; `dispatch/state-12-formal-verifier/command-logs/cargo_clippy_vb_storage_lib_strict.log` (raw, 72 B; SHA-256 `a5f4c585ee974ca44916ac30a98bbc189e067a7e0a6bc6d2e8d6bc525be724af`) |
| Cargo clippy (test target, strict) | `PROPTEST_CASES=1 cargo clippy -p vb_storage --tests -- -D warnings` | 101 (FAIL_GLOBAL pre-existing) | `dispatch/state-12-formal-verifier/command-logs/cargo_clippy_vb_storage_tests_strict.log` (raw, 102.2 KB; SHA-256 `103582215be01d4d3ad90d28dcf805a1df8374353e3d2ef9f7ca022c84dbc6e4`); 240 errors total. Parent baseline (`cargo_clippy_vb_storage_tests_strict_PARENT.log`, SHA-256 `963f96b7bfd0e645f6cde56a7c164ee9bad36676211757db03c43e973e5564ed`): 236 errors. Delta: +4 E0453 in `edge_case_tests.rs:4,6,7,8` from the file's pre-existing `#![allow(...)]` block (file content unchanged); identical pattern to 16 sibling declarations. Per AGENTS.md "test clippy is not strict", this is FAIL_GLOBAL pre-existing, not introduced by vb-n5k6v. |
| Cargo build (workspace) | `cargo check --workspace --all-targets --all-features` | 0 | `evidence/cargo-check-workspace.txt` — `cargo build (139 crates compiled) Finished `dev` profile ... in 9.04s` |
| Cargo test (workspace, no-run) | `cargo test --workspace --no-run` | 1 (pre-existing failure) | `evidence/cargo-test-workspace-no-run.txt` — E0624 errors in `vb_compile/tests/*` calling `WorkflowSource::new` from `tests/common/mod.rs`. Pre-existing on parent commit `rsvywymk`; not in vb-n5k6v blast radius. |
| Rust unit tests — regression (close_propagates_persist_errors) | `cargo test -p vb_storage --lib close_propagates_persist_errors` | 0 | `evidence/close-propagates-test.txt` — 1 passed, 1555 filtered out |
| Rust unit tests — regression (persist_strict) | `cargo test -p vb_storage --lib persist_strict` | 0 | `evidence/persist-strict-tests.txt` — 5 passed, 1551 filtered out |
| Rust unit tests — regression (append_strict) | `cargo test -p vb_storage --lib append_strict` | 0 | `evidence/append-strict-tests.txt` — 25 passed, 1531 filtered out |
| Cargo fmt --check | `cargo fmt --check` | 1 (FAIL_GLOBAL pre-existing) | `evidence/cargo-fmt-check.txt` — drift in `edge_case_tests.rs:627,632` and `vb_core/src/lib.rs:26` etc. Pre-existing on parent commit `rsvywymk`; the 4 lines added by vb-n5k6v are fmt-clean (match the 16-sibling pattern). |

## Verus / Kani / Flux / Loom / Fuzz / TLA+ Gates

| Lane | Status | Evidence |
|------|--------|----------|
| Verus | NOT REQUIRED | `verifier-lane-decisions.jsonl` row `vld-vb-n5k6v-decl-001-verus`: "No production-bound exec fn to verify. The 3-line `#[cfg(test)] #[path = "..."]` mod declaration is a Rust module-resolution construct, not an exec fn; no requires/ensures seam exists. Verus mirror-only proof would violate the no-vacuum-Verus rule." `scripts/check-verus-production-binding.sh` is not invoked. |
| Kani | NOT REQUIRED | `vld-vb-n5k6v-decl-001-kani`: "No bounded state/control-flow proof target. The module declaration has no symbolic input domain; kani::any() is not applicable to a static module-resolution construct." |
| Flux | NOT REQUIRED | No refinement type target. |
| Loom | NOT REQUIRED | 4 concurrent tests follow default-Rust threading precedent in `journal/tests.rs:2598+` and `recovery/tests.rs`; `FjallJournal::append_*` is `&self`; `JournalWriterQueue` wraps `Mutex<InnerState>` at `queue/writer.rs:33`. |
| Fuzz | NOT REQUIRED | No hostile-input surface; the 26 tests are concrete-value behavior tests. |
| TLA+ | REMOVED | Per `proof-strategy.md`, TLA+ is removed from the skill; temporal workflows use loom + proptest. |

## Verifier Lane Decisions (105 rows)

| Lane | Applicability | Count |
|------|---------------|-------|
| proptest (default-Rust) | required | 3 (PO-WIRE-DECL-001, PO-WIRE-RUN-004, PO-WIRE-DELTA-005) + 6 folded seeds (PS-WIRE-LINT-010, PS-WIRE-CONC-011, PS-WIRE-CODEC-012, PS-WIRE-PERSIST-013, PS-WIRE-BATCH-014, PS-WIRE-QUEUE-015) |
| verus | not_applicable | 3 rows (VLR-001-verus, VLR-002-verus, VLR-003-verus) |
| kani | not_applicable | 3 rows |
| flux | not_applicable | 3 rows |
| loom | not_applicable | 3 rows |
| fuzz | not_applicable | 3 rows |
| proptest (folded into default-rust) | subsumed | 3 rows |

All 105 verifier-lane-review rows carry `reviewer_disposition: "accepted"` per `verifier-lane-review.jsonl`.

## Source-Target Lint Clean

The wire declaration (4 lines in `lib.rs:183-186`) and the production fix (4 lines in `journal/append.rs:36-39`) introduce zero source-target clippy diagnostics. `cargo clippy -p vb_storage --lib -- -D warnings` exits 0 with "No issues found". This is the strict Holzman rule 10 gate (source lint zero-tolerance).

## Workspace Build Clean

`cargo check --workspace --all-targets --all-features` exits 0 with 139 crates compiled. The workspace test build (`cargo test --workspace --no-run`) fails on pre-existing `vb_compile/tests/*` E0624 errors, which are not in vb-n5k6v's blast radius (the bead touches only `vb_storage/src/lib.rs` and `vb_storage/src/journal/append.rs`).
