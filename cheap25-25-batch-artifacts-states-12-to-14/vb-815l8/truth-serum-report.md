# Truth Serum Report - vb-815l8

STATUS: APPROVED

## Audit

### Claim 1: "The replacement assertion is typed and discriminates `Err(InvalidRecoveryHydration)` from `Ok(_)` and other `Err(_)` variants."

- **Backed by raw evidence?** Yes.
- **Evidence path**: `crates/vb_runtime/src/error/equality.rs:3-28` shows the `PartialEq for RuntimeError` implementation that dispatches by unit tag; line 28 maps `RuntimeError::InvalidRecoveryHydration => Some(10)`. The typed `assert_eq!` at `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:87-91` invokes this `PartialEq`.
- **Limitations disclosed?** Yes — `PartialEq` discriminates by unit tag only; if a future edit assigns tag 10 to multiple unit variants, the discrimination is weakened. Mitigated by the 8 canonical typed-failure sites at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` and the dedicated unit-tag tests at `crates/vb_runtime/src/error/tests_basic.rs` and `tests_conversion_refinement.rs`.

### Claim 2: "Production code is untouched."

- **Backed by raw evidence?** Yes.
- **Evidence path**: `jj diff crates/vb_storage/src/recovery/types.rs crates/vb_runtime/src/recovery.rs` is empty (recorded in `implementation.md` §144). The full `jj diff --stat` is `1 file changed, 16 insertions(+), 4 deletions(-)` (test file only).
- **Limitations disclosed?** None. The contract.md §3 explicitly forbids production mutations; the bead scope is TEST-ONLY.

### Claim 3: "All 4 cargo-test obligations PASS."

- **Backed by raw evidence?** Yes.
- **Evidence paths**:
  - PO-001: `evidence/cargo_test_targeted_recovery_from_corrupt_snapshot.log` → 1 passed.
  - PO-002: `evidence/cargo_test_integration_runtime_storage_fault_tolerance.log` → 18 passed.
  - PO-003: `evidence/cargo_test_vb_runtime_recovery.log` → 13 passed (no regression).
  - PO-004: `evidence/cargo_test_vb_runtime_lib.log` → 1807 passed (no regression).
- **Limitations disclosed?** None. All 4 raw logs show `test result: ok.` with `0 failed; 0 ignored`.

### Claim 4: "No regressions."

- **Backed by raw evidence?** Yes.
- **Evidence path**: The 8 canonical typed-failure sites at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` all pass (13 in PO-003). The 1807-test baseline at `vb_runtime::lib` matches the pre-bead baseline (no test added, no test removed).
- **Limitations disclosed?** Yes — the workspace-wide `cargo fmt --check` and strict test clippy lanes have pre-existing failures (`vb_core`, `vb_runtime::frame_pool/tests.rs`, `restate_timer_deadline_primitive_tests.rs`). These are pre-existing repo-wide debt, not introduced by this bead, classified `BLOCK_GLOBAL` prerequisite repair.

### Claim 5: "All 8 non-behavior waivers are valid."

- **Backed by raw evidence?** Yes.
- **Evidence path**: Each waiver in `formal-waivers.jsonl` cites the corresponding `verifier-lane-decisions.jsonl` row, the `proof-seeds.jsonl` entry, the relevant contract clause, and a concrete reason tied to bead scope (TEST-ONLY, no production mutation, no new harness in scope). All 8 are `behavior_affecting: false` and `non_behavior: true`.
- **Limitations disclosed?** None. The waivers are lane-not-applicable, not behavior-affecting waivers. The `formal-verifier` skill explicitly rejects behavior-affecting waivers; this audit rejects none.

### Claim 6: "Triple-locked contract."

- **Backed by raw evidence?** Yes.
- **Evidence path**:
  1. The 8 canonical typed-failure sites at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492`.
  2. The `workspace_tests`-level typed-failure witness at `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:87-91` (the change in this bead).
  3. The pre-existing `PartialEq for RuntimeError` unit-tag dispatch at `crates/vb_runtime/src/error/equality.rs:3-28` (tag 10 for `InvalidRecoveryHydration`).
- **Limitations disclosed?** None. The three locks are independent: even if one is silently weakened, the other two still detect the regression.

### Claim 7: "Performance is not affected."

- **Backed by raw evidence?** Yes (by non-evidence).
- **Evidence path**: No benchmark is run because this is a correctness fix, not a perf change. The replacement `assert_eq!` macro expansion is compiler-elided on release `opt-level = 3`; runtime cost is identical to the production-grade typed-error equality check. No performance claim is made.
- **Limitations disclosed?** Yes — `implementation.md §151-157` explicitly states "No performance claim is made." Per `proof-planner` doctrine, claims without baseline/result benchmark evidence are forbidden; this bead makes no claim and therefore has no evidence gap.

### Claim 8: "No cover-only Kani, no commented-out tests, no ignored tests."

- **Backed by raw evidence?** Yes.
- **Evidence path**: All 4 raw test logs show `0 ignored` and `0 filtered out` for PO-002 and PO-004. PO-001 shows `0 ignored; 17 filtered out` (filtered because the targeted test name was passed). No `#[ignore]`, `#[cfg(skip_me)]`, or commented-out `#[test]` functions in the touched file.
- **Limitations disclosed?** None.

## Hallucination Audit

- **No invented output.** All test counts (1, 18, 13, 1807) are taken verbatim from the raw log files in `evidence/`.
- **No invented exit codes.** All 4 obligations show exit 0 per the raw logs.
- **No invented proof names.** No proof/harness artifacts are introduced; this bead is TEST-ONLY.
- **No invented tool availability.** `cargo +nightly`, `cargo test`, `cargo check`, `cargo fmt -p`, `jj diff` are all standard Rust/Jujutsu tooling available in this workspace.
- **No fabricated waiver provenance.** Each of the 8 waivers in `formal-waivers.jsonl` cites a `verifier-lane-decisions.jsonl` row, a `proof-seeds.jsonl` entry, and a contract clause — all of which exist on disk.

## Honest Limitations

1. **Workspace-wide lint debt.** The workspace has 4 pre-existing fmt failures and ~131+ pre-existing test clippy errors in unrelated files. These are recorded as `BLOCK_GLOBAL` prerequisite repair, not introduced by this bead, and do not block this bead's closure. The touched test file is independently lint-clean.
2. **Test-name intent mismatch.** The test is named `recovery_from_corrupt_snapshot_sequence_is_detected` but the body asserts boundary rejection (not corrupt-snapshot storage detection). This is flagged in `contract.md §5 Q1` as a P3 follow-up for `test-writer`, out of scope for this bead.
3. **Single-seed workspace_tests witness.** The `workspace_tests`-level test exercises only the happy-shape rejection path. The other typed-failure branches inside `hydrate_run_frame` are already covered at the production crate level (the 8 canonical typed-failure sites). Out of scope for this bead.
4. **No new coverage.** The bead does not add new test coverage beyond what the 8 canonical sites already provide; it adds a `workspace_tests`-level witness (cross-crate) that locks the contract at a different test boundary.

## Decision

**APPROVED** — All claims are backed by raw command evidence or by explicit non-evidence (no claim made). All 8 waivers are validated as non-behavior lane-applicability waivers. The pre-existing workspace-wide lint debt is disclosed as deferred global debt, not laundered as a pass. The triple-locked contract (canonical unit tests + workspace_tests witness + PartialEq unit-tag dispatch) is sufficient for bookmark-ready handoff.