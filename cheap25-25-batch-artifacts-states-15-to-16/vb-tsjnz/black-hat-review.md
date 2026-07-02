# Black-Hat Review — vb-tsjnz

bead_id: vb-tsjnz
bead_title: Cargo: opt `vb_queue_semantics` into workspace lints and version (P1 bug)
phase: 13
updated_at: 2026-07-01T15:55:00Z
attempt: 1

## Review Scope

This bead delivers a manifest-only Cargo opt-in patch:
`crates/vb_queue_semantics/Cargo.toml` swaps `version = "0.1.0"` for
`version.workspace = true` and appends a trailing `[lints]\nworkspace = true`
block. No production source code was modified. The `lib.rs` (423 lines,
flagged under bead `vb-2lu1`) is held out of scope.

## Phase 1: Contract & Bead Parity

### Requirements Coverage (contract.md REQ-VBTSJNZ-001 .. REQ-VBTSJNZ-012)

| REQ | Contract Clause | Evidence | Status |
|-----|-----------------|----------|--------|
| REQ-VBTSJNZ-001 | `[package]` line 3 = `version.workspace = true` | `jj diff -- crates/vb_queue_semantics/Cargo.toml` line 5 shows new entry; line 3 removed | ✅ |
| REQ-VBTSJNZ-002 | trailing `[lints]\nworkspace = true` block | `jj diff` lines 12-13 append the block; file ends with the block | ✅ |
| REQ-VBTSJNZ-003 | held invariants in `[package]` (edition.workspace, license.workspace, publish=false) | `jj diff` only modifies lines 3 and appends; lines 1, 2, 4, 6, 7 untouched | ✅ |
| REQ-VBTSJNZ-004 | sibling pattern parity | block shape matches `vb_cli`, `vb_compile`, `vb_core`, `vb_ipc`, `vb_runtime`, `vb_storage`, `vb_validate` (verified by scout in `codebase-map.md` and by string compare in `implementation.md:50-69`) | ✅ |
| REQ-VBTSJNZ-005 | `cargo check` green | PO-VBTSJNZ-001 exit 0; evidence 1782963263-state12-cargo-check.log | ✅ |
| REQ-VBTSJNZ-006 | `cargo clippy -D warnings` zero-warning | PO-VBTSJNZ-002 exit 0; "No issues found"; strict Holzman gate also exit 0 | ✅ |
| REQ-VBTSJNZ-007 | workspace_tests assertions + quality gates green | PO-VBTSJNZ-003: `vb_8ma2_workspace_assertions` 7 passed; `vb_qi37_25_quality_gates` 2 passed, 1 ignored (pre-existing) | ✅ |
| REQ-VBTSJNZ-008 | no scope bleed (no new deps/features/target tables) | `jj diff` shows only the package-version line and the trailing `[lints]` block | ✅ |
| REQ-VBTSJNZ-009 | `.config/source-length-exceptions.txt` line 323 untouched | `jj diff -- .config/source-length-exceptions.txt` empty | ✅ |
| REQ-VBTSJNZ-010 | recovery-under-failure policy preserved | patch succeeded; no `#[allow(...)]` added; no lint downgrades; no rust-toolchain edits | ✅ |
| REQ-VBTSJNZ-011 | cargo metadata reports version equality | `cargo metadata` reports `vb_queue_semantics` version = "0.1.0" = workspace.package.version | ✅ |
| REQ-VBTSJNZ-012 | black-hat reviewer audit (this document) | this file | ✅ |

**Parity Assessment**: All 12 contract clauses are satisfied with raw evidence.
No clause is open, no clause is waived, no clause requires follow-up.

## Phase 2: Forbidden-Repair Audit (contract.md lines 113-124)

Each forbidden repair is checked mechanically:

| Forbidden Repair | Check | Result |
|------------------|-------|--------|
| Lowering priority of any workspace lint | `jj diff -- Cargo.toml` (workspace root) is empty; `grep -E 'level = (allow|warn)' Cargo.toml` shows no allow/warn entries added | ✅ NOT PRESENT |
| Removing any workspace lint | workspace lint table unchanged | ✅ NOT PRESENT |
| Adding `#[allow(...)]` to source | `jj diff` shows no `lib.rs` modification; `grep '#\[allow' crates/vb_queue_semantics/src/lib.rs` returns nothing new | ✅ NOT PRESENT |
| Editing `.config/source-length-exceptions.txt` to remove `vb-2lu1` entry | `jj diff -- .config/source-length-exceptions.txt` empty | ✅ NOT PRESENT |
| Editing `rust-toolchain.toml` to bypass the patch | `rust-toolchain.toml` not in the diff | ✅ NOT PRESENT |
| Editing contract artifacts retroactively | `implementation.md`, `contract.md`, `proof-obligations.planned.jsonl` are not in the diff (`jj diff` only touches `crates/vb_queue_semantics/Cargo.toml`) | ✅ NOT PRESENT |

## Phase 3: Holzman Rust Compliance

This patch is manifest-only — there are no Rust expressions, no functions,
no loops, no allocations, no type-states, no panics, no `unsafe`, no
`unwrap`/`expect`, no `todo`/`unimplemented`. Holzman Rules 1-10 reduce to:

- **Rule 6 (smallest scope)**: satisfied — only two hunks; no scope creep.
- **Rule 10 (warnings and analysis)**: satisfied — `cargo check`, `cargo clippy`,
  `cargo clippy --strict`, and `cargo test --no-run` all exit 0 with the
  workspace lint policy fully inherited. No regression introduced.
- **All other rules**: N/A (no Rust code touched).

The `lib.rs` (held invariant) is also clean under the inherited policy —
that is the substantive meaning of PO-VBTSJNZ-001/002/PO-VBTSJNZ-strict-clippy
all returning 0: if any forbid/deny pattern existed in the existing
`lib.rs`, the inheritance would have promoted it to an error.

## Phase 4: Process-Hazard Audit

### Cargo metadata version equality (REQ-VBTSJNZ-011)

```
$ cargo metadata --no-deps --format-version 1 \
    | jq -r '.packages[] | select(.name=="vb_queue_semantics") | .version'
0.1.0

$ grep '^\[workspace.package\]' -A 4 Cargo.toml
[workspace.package]
edition = "2024"
license = "MIT OR Apache-2.0"
version = "0.1.0"
```

`vb_queue_semantics` resolves to `"0.1.0"`. Workspace root
`[workspace.package].version` is `"0.1.0"`. Equality holds. REQ-VBTSJNZ-011 ✅.

### Diff bounded to two hunks (REQ-VBTSJNZ-008 / REQ-VBTSJNZ-012)

```
$ jj diff --stat
crates/vb_queue_semantics/Cargo.toml | 5 ++++-
1 file changed, 4 insertions(+), 1 deletion(-)
```

Exactly 1 file modified, 5 lines changed in that file (1 removed on line 3,
4 added: `version.workspace = true` on line 5 + blank line + `[lints]\nworkspace = true`).
This is the shape mandated by `contract.md` lines 76-77.

### Source-length exception preserved (REQ-VBTSJNZ-009)

```
$ jj diff -- .config/source-length-exceptions.txt
(no output — file unchanged)
```

The pre-existing `vb-2lu1` source-length exception at line 323 is intact.

### Test surface (REQ-VBTSJNZ-007)

```
$ cargo test -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions
cargo test: 7 passed (1 suite, 0.14s)

$ cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_25_quality_gates
cargo test: 2 passed, 1 ignored (1 suite, 0.11s)
```

The 1 ignored test is pre-existing in the test file (per `vb_qi37_25_quality_gates.rs`)
and is not introduced by this bead. The 7 + 2 = 9 active tests all pass.

## Phase 5: Diff Boundary Audit

The patch is byte-for-byte bounded:

| Hunk | Location | Operation | Content |
|------|----------|-----------|---------|
| Hunk 1 | `crates/vb_queue_semantics/Cargo.toml` line 3 | replaced | `version = "0.1.0"` → `version.workspace = true` |
| Hunk 2 | `crates/vb_queue_semantics/Cargo.toml` after line 11 | appended | blank line + `[lints]\nworkspace = true` |

- Hunk 1: field-swap preserving the sister-crate pattern (`edition.workspace`,
  `license.workspace`, `version.workspace`).
- Hunk 2: trailing `[lints]\nworkspace = true` block matching all 7 sister
  crates (`vb_cli`, `vb_compile`, `vb_core`, `vb_ipc`, `vb_runtime`,
  `vb_storage`, `vb_validate`).

No other file is touched. `lib.rs` is unchanged. `Cargo.lock` is unchanged.
`Cargo.toml` (workspace root) is unchanged. `.config/source-length-exceptions.txt`
is unchanged. `rust-toolchain.toml` is unchanged.

## Phase 6: Proof/Test/Source Parity Matrix

| Proof ID | Behavior Affecting | Test/Build Evidence | Result | Status |
|----------|--------------------|---------------------|--------|--------|
| PO-VBTSJNZ-001 | Yes | `cargo check -p vb_queue_semantics --all-targets` exit 0 | PASS | ✅ |
| PO-VBTSJNZ-002 | Yes | `cargo clippy -p vb_queue_semantics --all-targets` exit 0 ("No issues found"); strict Holzman also exit 0 | PASS | ✅ |
| PO-VBTSJNZ-003 | No | `cargo test -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions` 7 passed; `--test vb_qi37_25_quality_gates` 2 passed, 1 ignored | PASS | ✅ |
| PO-VBTSJNZ-004 | No | `jj diff --stat` shows exactly 1 file (`crates/vb_queue_semantics/Cargo.toml`); `cargo metadata` version equality; `source-length-exceptions.txt` unchanged | PASS | ✅ |

All 4 planned obligations from `proof-obligations.planned.jsonl` have
matching ledger rows in `verification-ledger.jsonl` with raw command
evidence and PASS status.

## VERDICT: APPROVED

### Summary

- All 12 contract clauses are satisfied with raw evidence.
- All 6 Forbidden Repairs are absent from the diff.
- All 4 planned proof obligations are PASS with raw command evidence.
- The diff is bounded to two hunks in one file (the contract-mandated shape).
- The workspace lint policy is fully inherited; no downgrades; no source
  `#[allow(...)]`; no toolchain edits; no exception-file edits.
- The held invariant `lib.rs` is clean under the inherited policy
  (proven by PO-VBTSJNZ-001/002 returning 0).
- No behavior is introduced (manifest-only patch); no waiver is filed.

### Non-Blocking Findings (Documented)

1. **PO-VBTSJNZ-003 package id**: The planned obligation command uses
   `-p workspace_tests` (directory name); the actual workspace package
   name is `velvet-ballistics-workspace-tests`. Cargo rejects the
   planned id with "did not match any packages". The substantive intent
   is verified with the actual package name and both test binaries pass.
   This is a planning-artifact typo, not a defect in the patch.

2. **PO-VBTSJNZ-004 script literal assertion**: The planned script
   contains `[ "$(jj diff --stat | wc -l)" = "1" ]`. The actual
   `jj diff --stat` output is 2 lines (file row + summary footer).
   The substantive check (1 file modified, exactly the expected file)
   passes. The literal assertion is a planning artifact that does not
   match modern `jj diff --stat` output; the substantive gate is
   verified by direct inspection of `jj diff`.

3. **`cargo fmt --check` pre-existing drift**: `crates/vb_queue_semantics/Cargo.toml`
   contains an em-dash (`—`) on the unchanged stub-comment line. The
   em-dash pre-dates this bead and is BLOCK_GLOBAL drift tracked at
   repo level. Not introduced by this bead.

### Defects

Zero defects. See `defects.md` (empty).

---

**STATUS: APPROVED**