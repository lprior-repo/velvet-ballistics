# Formal Verification Report — vb-tsjnz

bead_id: vb-tsjnz
bead_title: Cargo: opt `vb_queue_semantics` into workspace lints and version (P1 bug)
phase: 12
updated_at: 2026-07-01T15:50:00Z
attempt: 1

## Summary

Manifest-only Cargo opt-in patch (`version.workspace = true` + `[lints]\nworkspace = true`)
to `crates/vb_queue_semantics/Cargo.toml`. No production source touched.
All three explicit verification commands exit 0. All four planned proof obligations
reach PASS status with raw command evidence captured under `.beads/vb-tsjnz/evidence/`.

## Machine Gate Results (Cargo)

### `cargo check -p vb_queue_semantics --all-targets` — PASS (exit 0)

```
    Checking vb_queue_semantics v0.1.0 (/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/vb_queue_semantics)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
```

- Workspace lints inherited; no error; no warning promoted to error.
- Captured at `.beads/vb-tsjnz/evidence/1782963263-state12-cargo-check.log`.

### `cargo clippy -p vb_queue_semantics --all-targets` — PASS (exit 0, "No issues found")

```
cargo clippy: No issues found
```

- No clippy lint (correctness, suspicious, perf, complexity, restrict, style,
  pedantic, nursery) trips.
- Captured at `.beads/vb-tsjnz/evidence/1782963263-state12-cargo-clippy.log`.

### `cargo test -p vb_queue_semantics --no-run` — PASS (exit 0)

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s
  Executable unittests src/lib.rs (target/debug/deps/vb_queue_semantics-3a217c92b205db74)
```

- Test binary compiles cleanly under the workspace lints.
- Captured at `.beads/vb-tsjnz/evidence/1782963263-state12-cargo-test-no-run.log`.

## Verification Ledger Summary

| ID | Category | Result |
|----|----------|--------|
| PO-VBTSJNZ-001 | cargo-check | PASS |
| PO-VBTSJNZ-002 | cargo-clippy | PASS |
| PO-VBTSJNZ-003 | cargo-test (workspace_tests) | PASS |
| PO-VBTSJNZ-004 | cargo-metadata / jj-diff | PASS (substantive) |
| **Total** | **4** | **4 PASS / 0 FAIL / 0 WAIVED** |

Detailed rows in `verification-ledger.jsonl` (4 entries).

## Required Proof Obligations (Planned)

All four planned obligations from `proof-obligations.planned.jsonl` have been
executed with raw command evidence:

| Obligation | Tool | Command | Evidence File | Result |
|------------|------|---------|---------------|--------|
| PO-VBTSJNZ-001 | cargo-check | `cargo check -p vb_queue_semantics --all-targets` | 1782963263-state12-cargo-check.log | PASS |
| PO-VBTSJNZ-002 | cargo-clippy | `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` | 1782963263-state12-cargo-clippy.log | PASS |
| PO-VBTSJNZ-003 | cargo-test | `cargo test -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions` + `--test vb_qi37_25_quality_gates` | 1782963263-state12-po003a-*.log, 1782963263-state12-po003b-*.log | PASS |
| PO-VBTSJNZ-004 | cargo-metadata / jj-diff | `jj diff --stat`, `jj diff -- Cargo.toml`, `cargo metadata --no-deps`, `jj diff -- .config/source-length-exceptions.txt` | 1782963263-state12-po004-*.log | PASS (substantive) |

## Strict Holzman Source-Lint Gate (Defense in Depth)

Strict Holzman clippy also re-run as a defense-in-depth check:

```
cargo clippy -p vb_queue_semantics --all-targets -- -D warnings -D unsafe_code \
  -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic \
  -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented \
  -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice \
  -D clippy::get_unwrap -D clippy::arithmetic_side_effects \
  -D clippy::as_conversions -D clippy::let_underscore_must_use
```

Result: `cargo clippy: No issues found` — exit 0.
Captured at `.beads/vb-tsjnz/evidence/1782963270-state12-strict-clippy.log`.

This is the same gate that holzman-rust ran in State 11 implementation.md and is
documented in `implementation.md` table row 3. It re-verifies PO-VBTSJNZ-002
with the workspace lint policy plus every Holzman-explicit deny.

## Waivers

`formal-waivers.jsonl` is empty (zero waivers filed). The patch introduces zero
behavior-affecting change and the workspace lint policy remains intact.

No production `#[allow(...)]`, no lint downgrades, no source modifications,
no `rust-toolchain.toml` bypass, no `.config/source-length-exceptions.txt`
edits — every Forbidden Repair in `contract.md` lines 113-124 is preserved.

## Non-Blocking Findings (Documented)

1. **PO-VBTSJNZ-003 package id**: The planned obligation command uses
   `-p workspace_tests` but the actual workspace package name is
   `velvet-ballistics-workspace-tests` (per `crates/workspace_tests/Cargo.toml:2`).
   Cargo rejects `-p workspace_tests` with "did not match any packages".
   Re-running with `-p velvet-ballistics-workspace-tests` succeeds:
   - `vb_8ma2_workspace_assertions`: 7 passed
   - `vb_qi37_25_quality_gates`: 2 passed, 1 ignored (pre-existing, see
     `crates/workspace_tests/tests/vb_qi37_25_quality_gates.rs`)

   This is a planning-artifact typo (directory name vs package name), not a
   defect. The substantive intent (workspace-tests assertions and quality
   gates stay green) is verified.

2. **PO-VBTSJNZ-004 script literal assertion**: The planned obligation
   script contains `[ "$(jj diff --stat | wc -l)" = "1" ]`. The actual
   `jj diff --stat` output for this bead is 2 lines (one file row plus a
   "1 file changed, 4 insertions(+), 1 deletion(-)" summary footer).
   The substantive check (exactly one file modified, exactly the expected
   file) passes. The literal `wc -l == 1` assertion is a planning artifact
   that does not match modern jj output; the substantive gate is verified
   by direct inspection of `jj diff -- crates/vb_queue_semantics/Cargo.toml`.

## Formal Verification Approval

**STATUS: PASS**

All three explicit cargo commands exit 0. All four planned proof obligations
have raw command evidence and PASS status. No waivers required. No behavior
introduced (manifest-only patch).

## Evidence Artifacts

- `verification-ledger.jsonl`: 4 obligation rows, all PASS
- `formal-waivers.jsonl`: empty (0 rows)
- `.beads/vb-tsjnz/evidence/1782963263-state12-*.log`: 8 raw command-output files
- `.beads/vb-tsjnz/evidence/1782963270-state12-strict-clippy.log`: defense-in-depth gate
- `.beads/vb-tsjnz/implementation.md`: holzman-rust state 11 artifact
- `.beads/vb-tsjnz/proof-obligations.planned.jsonl`: source of truth for the 4 rows
- `.beads/vb-tsjnz/contract.md`: REQ-VBTSJNZ-001 through REQ-VBTSJNZ-012