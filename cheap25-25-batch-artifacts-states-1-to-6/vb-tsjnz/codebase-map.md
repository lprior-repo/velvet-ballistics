# Codebase Map — vb-tsjnz

- bead_id: vb-tsjnz
- title: Cargo: opt `vb_queue_semantics` into workspace lints and version (P1 bug)
- priority: P1
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz
- jj_workspace: cheap25-vb-tsjnz
- jj_change: xnskrsku 5ed28a5e (empty, parent rsvywymk 1d6c017f)
- captured_at: 2026-07-01
- scout_owner: explore (State 2)
- controller: femdation

## Bead Summary

The `vb_queue_semantics` workspace member crate currently diverges from the
rest of the workspace on two Cargo metadata axes:

1. Its package `version` is hardcoded as `version = "0.1.0"` rather than
   inheriting from `[workspace.package].version` via `version.workspace = true`.
2. It has no `[lints]` section, so it does not opt into the
   `[workspace.lints.rust]` / `[workspace.lints.clippy]` declarations.

Every other workspace member (vb_cli, vb_compile, vb_core, vb_ipc, vb_runtime,
vb_storage, vb_validate, workspace_tests) uses both `version.workspace = true`
and `[lints]\nworkspace = true`. Bringing `vb_queue_semantics` into alignment
closes a P1 governance drift and keeps lint parity consistent.

## In-Scope Files (Authoritative — read before edit)

| Path | Role | Notes |
| --- | --- | --- |
| `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/vb_queue_semantics/Cargo.toml` | **primary edit target** (11 lines) | Lines 1-7 `[package]`: `version = "0.1.0"` (line 3) is the only hardcoded version; needs `version.workspace = true`. Missing `[lints]\nworkspace = true` block entirely. |
| `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/Cargo.toml` | workspace root manifest (2.6 KB) | Defines `[workspace.package]` with `version = "0.1.0"`, `edition = "2024"`, `license = "MIT OR Apache-2.0"`. `vb_queue_semantics` is listed at line 7 in `members`. `[workspace.lints.rust]` and `[workspace.lints.clippy]` are present and authoritative. |

## Reference Files (Patterns to Match)

The scout must match the inlining pattern used by every other member:

- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/vb_cli/Cargo.toml` (lines 3-5 use `version.workspace = true`; line 38 has `[lints]\nworkspace = true`).
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/vb_compile/Cargo.toml` (line 5 `version.workspace = true`; line 26 `[lints]\nworkspace = true`).
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/vb_core/Cargo.toml` (line 5 `version.workspace = true`; line 34 `[lints]\nworkspace = true`).
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/vb_ipc/Cargo.toml` (line 5 `version.workspace = true`; line 23 `[lints]\nworkspace = true`).
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/vb_runtime/Cargo.toml` (line 5 `version.workspace = true`; terminal `[lints]\nworkspace = true` block).
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/vb_storage/Cargo.toml` (line 5 `version.workspace = true`; terminal `[lints]\nworkspace = true`).
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/vb_validate/Cargo.toml` (line 5 `version.workspace = true`; line 20 `[lints]\nworkspace = true`).

## Test / Gate Coupling (No New Tests Required)

The bead is a Cargo metadata patch. Existing gates will exercise the change:

- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/scripts/check-workspace-assertions.rs` — already lists `("crates/vb_queue_semantics", "vb_queue_semantics")` (line 44) as an `EXPECTED_PACKAGE_NAMES` row and `crates/vb_queue_semantics` in `EXPECTED_MEMBERS` (line 11). The script does **not** assert version or lints, so this gate stays green either way.
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs` — references `crates/vb_queue_semantics` at lines 12 and 57; copies the assertion script into a temp workspace and exercises it. Will continue to pass.
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/crates/workspace_tests/tests/vb_qi37_25_quality_gates.rs` line 14 references `("crates/vb_queue_semantics", "vb_queue_semantics")` — package-name smoke only.
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/.moon/tasks/all.yml` lines 396 and 399 — `cargo machete` and supply-chain scans iterate over the workspace members list, so the crate is already covered.
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/xtask/src/forbidden_scan.rs` line 18 — crate listed in forbidden/unaffected scan allow-list (no change required).
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/.config/source-length-exceptions.txt` line 323 — pre-existing 427-line exception for `crates/vb_queue_semantics/src/lib.rs` under bead `vb-2lu1`. **Out of scope** for vb-tsjnz; do not touch.
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz/scripts/generate_queue_state_verus_helpers.py` — Verus helper generator reads `crates/vb_queue_semantics/src/lib.rs` constants/helpers; unaffected by Cargo.toml changes.

## Source Inventory (For Risk Scoring Only)

- `crates/vb_queue_semantics/src/lib.rs` (423 lines) — `forbid(unsafe_code)` header is already present, plus per-line `#![deny(...)]` for `unused_must_use`, `unreachable_pub`, `rust_2018_idioms`. Adding the workspace lints will additionally enforce: `unsafe_code = "forbid"`, `unwrap_used = "forbid"`, `expect_used = "forbid"`, `panic = "forbid"`, `unimplemented = "forbid"`, `todo = "forbid"`, `dbg_macro = "forbid"`, `string_slice = "forbid"`, `get_unwrap = "forbid"`, `arithmetic_side_effects = "deny"`, `as_conversions = "deny"`, `let_underscore_must_use = "deny"`, `await_holding_lock = "deny"`, `indexing_slicing = "deny"`, plus the standard `correctness`/`suspicious`/`perf`/`complexity` clippy groups. **Risk:** the existing 423-line file was authored without those denies; if any `unwrap`/`expect`/panic-shaped pattern exists the new lint set will fail the build. The scout did not find `unwrap(`, `expect(`, `panic!`, `todo!`, `unimplemented!`, or `dbg!` in the source via Grep — but downstream **holzman-rust** must re-verify before claiming green.

## Risk Tags

- `cargo-metadata`: P1 governance drift; missing workspace inheritance.
- `lint-policy`: Adds zero-tolerance clippy denies (unwrap_used, expect_used, panic, todo, unimplemented, dbg_macro, arithmetic_side_effects, as_conversions, indexing_slicing, etc.) to a crate that was authored under only its `#![deny(...)]` header set.
- `dependency`: NONE — no new dep, no version bump on existing deps.
- `public-api`: NONE — package version becomes `0.1.0` (matches workspace) instead of an isolated `0.1.0`; semver-equivalent.
- `verifier-lanes`: NONE — no Kani/Verus/Flux/TLA+/Loom/fuzz obligations attach to a Cargo metadata-only patch.
- `tests-required`: NONE new — existing workspace assertions and `cargo check -p vb_queue_semantics` already cover the crate.

## Open Questions / Hand-off Notes

1. The bead text says "Add it to workspace.lints and ensure version is aligned with the workspace version." Confirmed workspace version is `0.1.0` (matches current hardcoded value), so version alignment is a no-op numerically but a metadata-hygiene gain (inheritance).
2. **UNCONFIRMED:** Whether the existing 423-line `lib.rs` is clean against the workspace lints the bead will turn on. The scout found no obvious deny-trip patterns via Grep, but a final pre-merge `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` and `cargo check -p vb_queue_semantics --all-targets` is required downstream.
3. **OUT OF SCOPE:** Source-length exception, Verus helper generator, `vb_queue_semantics/src/lib.rs` refactor — not part of vb-tsjnz.

## Downstream Owner Recommendation

- **Implementation (rust-contract → holzman-rust):** two-line Cargo.toml edit on `crates/vb_queue_semantics/Cargo.toml`.
- **Test:** `cargo check -p vb_queue_semantics --all-targets` + `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` + existing `cargo test -p workspace_tests --test vb_8ma2_workspace_assertions` and `cargo test -p workspace_tests --test vb_qi37_25_quality_gates`.
- **Proof lanes:** NONE — no Verus/Kani/Flux/Loom obligations for Cargo metadata.
- **Review (black-hat):** verify both axes (version inheritance + lints inheritance) by reading the diff and re-running `cargo metadata` + `cargo clippy`.

## Verification Gate (Mandatory)

```bash
pwd -P
# /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz
test -s ".beads/vb-tsjnz/codebase-map.md"
test -s ".beads/vb-tsjnz/delivery-scope.jsonl"
jq -c . ".beads/vb-tsjnz/delivery-scope.jsonl" >/dev/null
```
