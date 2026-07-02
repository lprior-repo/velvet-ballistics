# Contract — vb-tsjnz

- bead_id: `vb-tsjnz`
- title: Cargo: opt `vb_queue_semantics` into workspace lints and version (P1 bug)
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`
- capture: 2026-07-01
- status: DRAFT (pre-rust-contract issue)

This file is the **normative contract** for the patch. The supporting artifacts
(domain-model, type-contracts, workflow-model, error-taxonomy, boundary-map,
hazard-analysis, proof-seeds.jsonl, traceability-matrix.jsonl) are the
auxiliary documentation.

## Scope

A two-axis metadata-only edit to `crates/vb_queue_semantics/Cargo.toml`:

1. **Axis A — Inheritance of `[workspace.package].version`.**
2. **Axis B — Inheritance of `[workspace.lints.rust]` and `[workspace.lints.clippy]`.**

No other fields, dependencies, features, target tables, or source files are
modified by this bead.

## REQ-VBTSJNZ-001 — `version.workspace = true`

The `[package]` block of `crates/vb_queue_semantics/Cargo.toml` SHALL contain
`version.workspace = true` on line 3 and SHALL NOT contain a literal
`version = "..."` entry.

## REQ-VBTSJNZ-002 — `[lints]\nworkspace = true`

The file `crates/vb_queue_semantics/Cargo.toml` SHALL contain a `[lints]`
table whose only entry is `workspace = true`, and that table SHALL be the
final block in the file.

## REQ-VBTSJNZ-003 — Held invariants in `[package]`

The fields `edition.workspace = true` and `license.workspace = true` SHALL
remain unchanged. The field `publish = false` SHALL remain unchanged.

## REQ-VBTSJNZ-004 — Sibling pattern parity

The shape of the edited `[lints]` block MUST be character-identical to the
shape used by `vb_cli/Cargo.toml:37-38`, `vb_compile/Cargo.toml:25-26`,
`vb_core/Cargo.toml:33-34`, `vb_ipc/Cargo.toml`, `vb_runtime/Cargo.toml:44`,
`vb_storage/Cargo.toml`, and `vb_validate/Cargo.toml`.

The literal-shape contract:

```toml
[lints]
workspace = true
```

## REQ-VBTSJNZ-005 — `cargo check` green under workspace lints

After the patch, `cargo check -p vb_queue_semantics --all-targets` SHALL exit
0 with no errors and no warnings promoted to errors.

## REQ-VBTSJNZ-006 — `cargo clippy` zero-warning under workspace lints

After the patch,
`cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` SHALL exit 0.

## REQ-VBTSJNZ-007 — Workspace-test gates green

After the patch, all of the following SHALL exit 0:

- `cargo test -p workspace_tests --test vb_8ma2_workspace_assertions`
- `cargo test -p workspace_tests --test vb_qi37_25_quality_gates`

## REQ-VBTSJNZ-008 — No scope bleed

The patch MUST NOT add or remove any of: `[dependencies]` entries,
`[dev-dependencies]` block, `[features]` block, `[[bench]]`, `[[test]]`,
`[lib]`, `[[bin]]`. The Edit is confined to lines 3 (replace) and tail
(append `[lints]\nworkspace = true`).

## REQ-VBTSJNZ-009 — Exception file preserved

The pre-existing source-length exception for `crates/vb_queue_semantics/src/lib.rs`
recorded at `.config/source-length-exceptions.txt` line 323 under bead
`vb-2lu1` SHALL remain untouched.

## REQ-VBTSJNZ-010 — Recovery under failure

If REQ-VBTSJNZ-005 or REQ-VBTSJNZ-006 fails:

1. Holzman-rust MUST NOT modify the workspace lint policy.
2. Holzman-rust MUST NOT add `#[allow(...)]` to `vb_queue_semantics` source.
3. Holzman-rust MUST report `Failed::LintFailure` to the controller; the
   source cleanup becomes a follow-up bead owned by the original author of
   `lib.rs`. The current bead fails; no patch lands; no waiver is filed
   against the workspace policy.

## REQ-VBTSJNZ-011 — Cargo TOML inheritance resolves

`cargo metadata --no-deps --format-version 1` MUST report
`packages[name=vb_queue_semantics].version = "0.1.0"` identical to
`workspace_members.packages[].version` (= workspace root
`[workspace.package].version = "0.1.0"`).

## REQ-VBTSJNZ-012 — Black-hat reviewer audit

After landing, a black-hat reviewer MUST verify:

- The diff is exactly two hunks: (a) line 3 replaced; (b) trailing two lines
  appended.
- No other file in the workspace is touched by the patch.
- `cargo metadata` reports version equality (REQ-VBTSJNZ-011).
- `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` is green.

## Forbidden Repairs (Cross-Cutting)

These moves are forbidden even if they would make the build green:

- Lowering the priority of any workspace lint.
- Removing any workspace lint.
- Adding `#[allow(...)]` to source.
- Editing `.config/source-length-exceptions.txt` to remove the `vb-2lu1` entry.
- Editing `rust-toolchain.toml` to bypass the patch.
- Editing contract artifacts retroactively.

These are restatements of the "no Loop Oscillations" rule.

## Verification (Statement of Completion)

The bead is **complete** when ALL of the following hold simultaneously:

- `crates/vb_queue_semantics/Cargo.toml` line 3 is `version.workspace = true`.
- The file ends with `[lints]\nworkspace = true`.
- `cargo check -p vb_queue_semantics --all-targets` exits 0.
- `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` exits 0.
- Both `cargo test -p workspace_tests --test vb_8ma2_workspace_assertions`
  and `cargo test -p workspace_tests --test vb_qi37_25_quality_gates` exit 0.
- `.config/source-length-exceptions.txt` line 323 is unchanged.
- `jj diff` shows the patch is contained to `crates/vb_queue_semantics/Cargo.toml`.

## Cross-References

- Domain language and entities: `domain-model.md`
- Type-level TOML contracts: `type-contracts.md`
- Workflow lifecycle and gates: `workflow-model.md`
- Error catalogue: `error-taxonomy.md`
- Boundary map: `boundary-map.md`
- Hazard catalogue: `hazard-analysis.md`
- Proof seeds: `proof-seeds.jsonl`
- Traceability matrix: `traceability-matrix.jsonl`
