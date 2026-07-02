# Assurance Bundle — vb-tsjnz

bead_id: vb-tsjnz
bead_title: Cargo: opt `vb_queue_semantics` into workspace lints and version (P1 bug)
source_checkout: /home/lewis/src/velvet-ballistics (coordination only)
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz
commit_or_change: xnskrsku 5ed28a5e (parent rsvywymk 1d6c017f) — `crates/vb_queue_semantics/Cargo.toml` only
updated_at: 2026-07-01T16:00:00Z

## Summary

This bead delivers a manifest-only Cargo opt-in patch. Two hunks in
`crates/vb_queue_semantics/Cargo.toml`: (a) line 3 swaps
`version = "0.1.0"` for `version.workspace = true`; (b) trailing
`[lints]\nworkspace = true` block is appended. No production source touched.
The `lib.rs` (423 lines) is held invariant under the `vb-2lu1` source-length
exception and remains clean under the inherited lint policy (proven by
the cargo check/clippy/test gates returning 0).

## Requirement Coverage

| REQ | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|-----|-----------------|---------------------|-----------------|--------|
| REQ-VBTSJNZ-001 | `[package]` line 3 = `version.workspace = true` | `jj diff -- crates/vb_queue_semantics/Cargo.toml` shows line 5 = `version.workspace = true` and line 3 removed | black-hat-review.md Phase 1 | ✅ PASS |
| REQ-VBTSJNZ-002 | trailing `[lints]\nworkspace = true` block | `jj diff` lines 12-13 append the block; file ends with the block | black-hat-review.md Phase 1, 5 | ✅ PASS |
| REQ-VBTSJNZ-003 | held invariants in `[package]` | `jj diff` only modifies lines 3 and appends; lines 1, 2, 4, 6, 7 untouched | black-hat-review.md Phase 1, 5 | ✅ PASS |
| REQ-VBTSJNZ-004 | sibling pattern parity | block shape matches 7 sister crates (`vb_cli`, `vb_compile`, `vb_core`, `vb_ipc`, `vb_runtime`, `vb_storage`, `vb_validate`) | implementation.md:50-69; codebase-map.md | ✅ PASS |
| REQ-VBTSJNZ-005 | `cargo check` green | PO-VBTSJNZ-001: `cargo check -p vb_queue_semantics --all-targets` exit 0 (1782963263-state12-cargo-check.log) | formal-verification-report.md | ✅ PASS |
| REQ-VBTSJNZ-006 | `cargo clippy -D warnings` zero-warning | PO-VBTSJNZ-002: exit 0, "No issues found"; strict Holzman also exit 0 (1782963263-state12-cargo-clippy.log, 1782963270-state12-strict-clippy.log) | formal-verification-report.md | ✅ PASS |
| REQ-VBTSJNZ-007 | workspace_tests assertions + quality gates green | PO-VBTSJNZ-003: vb_8ma2 7 passed; vb_qi37_25 2 passed, 1 ignored (pre-existing) | formal-verification-report.md | ✅ PASS |
| REQ-VBTSJNZ-008 | no scope bleed | `jj diff --stat` shows only `crates/vb_queue_semantics/Cargo.toml` (5 lines changed) | black-hat-review.md Phase 5 | ✅ PASS |
| REQ-VBTSJNZ-009 | source-length exception preserved | `jj diff -- .config/source-length-exceptions.txt` empty | black-hat-review.md Phase 4 | ✅ PASS |
| REQ-VBTSJNZ-010 | recovery-under-failure policy preserved | patch succeeded; no `#[allow(...)]` added; no lint downgrades; no rust-toolchain edits; contract artifacts untouched | black-hat-review.md Phase 2 | ✅ PASS |
| REQ-VBTSJNZ-011 | cargo metadata version equality | PO-VBTSJNZ-004: `vb_queue_semantics` version = `"0.1.0"` = `[workspace.package].version` (1782963263-state12-po004-cargo-metadata-version.log) | formal-verification-report.md | ✅ PASS |
| REQ-VBTSJNZ-012 | black-hat reviewer audit | black-hat-review.md STATUS: APPROVED | this bead | ✅ PASS |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|------------|------|---------|----------|--------|--------|
| PO-VBTSJNZ-001 | cargo-check | `cargo check -p vb_queue_semantics --all-targets` | 1782963263-state12-cargo-check.log | PASS | None |
| PO-VBTSJNZ-002 | cargo-clippy | `cargo clippy -p vb_queue_semantics --all-targets` | 1782963263-state12-cargo-clippy.log; strict: 1782963270-state12-strict-clippy.log | PASS | None |
| PO-VBTSJNZ-003 | cargo-test | `cargo test -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions` + `--test vb_qi37_25_quality_gates` | 1782963263-state12-po003a-*.log, 1782963263-state12-po003b-*.log | PASS | None |
| PO-VBTSJNZ-004 | cargo-metadata + jj-diff | `jj diff --stat`, `jj diff -- Cargo.toml`, `cargo metadata`, `jj diff -- .config/source-length-exceptions.txt` | 1782963263-state12-po004-*.log | PASS | None |

Defense-in-depth (holzman-rust state 11):
`cargo clippy -p vb_queue_semantics --all-targets -- -D warnings -D unsafe_code
-D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
-D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented
-D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice
-D clippy::get_unwrap -D clippy::arithmetic_side_effects
-D clippy::as_conversions -D clippy::let_underscore_must_use` → "No issues found".

## Test Evidence

| Gate | Command | Artifact | Result |
|------|---------|----------|--------|
| Build | `cargo check -p vb_queue_semantics --all-targets` | 1782963263-state12-cargo-check.log | ✅ exit 0 |
| Lint | `cargo clippy -p vb_queue_semantics --all-targets` | 1782963263-state12-cargo-clippy.log | ✅ "No issues found" |
| Test compile | `cargo test -p vb_queue_semantics --no-run` | 1782963263-state12-cargo-test-no-run.log | ✅ exit 0 |
| Workspace tests | `cargo test -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions` | 1782963263-state12-po003a-vb_8ma2_workspace_assertions.log | ✅ 7 passed |
| Workspace tests | `cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_25_quality_gates` | 1782963263-state12-po003b-vb_qi37_25_quality_gates.log | ✅ 2 passed, 1 ignored |
| Strict Holzman clippy | `cargo clippy -- -D warnings -D unsafe_code -D clippy::*` | 1782963270-state12-strict-clippy.log | ✅ "No issues found" |

## Review Evidence

| Review | Artifact | Status | Findings |
|--------|----------|--------|----------|
| Implementation | implementation.md (state 11) | Holzman-rust completed | Manifest-only patch; 5 commands run; zero forbidden constructs |
| Formal Verification | formal-verification-report.md (state 12) | PASS | 3 explicit cargo commands exit 0; 4 PO rows PASS; 0 waivers |
| Black-Hat Review | black-hat-review.md (state 13) | APPROVED | 12 contract clauses ✅; 6 forbidden repairs absent; 4 PO parity ✅ |
| Truth Serum | truth-serum-report.md (state 14) | CLEAN | All claims backed by raw command output; no hallucinated evidence |

## Waivers And Deferred Work

| Item | Reason | Owner | Follow-up | Compensating Evidence |
|------|--------|-------|-----------|----------------------|
| (none) | — | — | — | — |

`formal-waivers.jsonl` is empty. `defects.md` is empty. Zero waivers filed.

## Non-Blocking Findings (Documented)

1. **PO-VBTSJNZ-003 package id**: planned command uses `-p workspace_tests`
   (directory name); actual package name is `velvet-ballistics-workspace-tests`.
   Cargo rejects the planned id; substantive intent verified with the actual
   name. Both test binaries pass.
2. **PO-VBTSJNZ-004 literal assertion**: planned `[ "$(jj diff --stat | wc -l)" = "1" ]`
   fails against modern jj output (2 lines); substantive check passes.
3. **`cargo fmt --check` em-dash**: pre-existing drift in
   `crates/vb_queue_semantics/Cargo.toml` stub-comment line (em-dash `—`).
   Not introduced by this bead. BLOCK_GLOBAL tracked at repo level.

## Truth Serum Audit

- report: `.beads/vb-tsjnz/truth-serum-report.md`
- status: CLEAN — see `final-evidence-decision.md` for STATUS: APPROVED.

## Ready for Landing

The bead is cleared for State 14 (Landing).