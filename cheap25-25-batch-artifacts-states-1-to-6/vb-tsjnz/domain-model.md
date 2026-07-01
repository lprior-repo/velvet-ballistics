# Domain Model — vb-tsjnz

- bead_id: `vb-tsjnz`
- title: Cargo: opt `vb_queue_semantics` into workspace lints and version (P1 bug)
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`
- lane: Rust-local + policy
- scope: metadata-only `Cargo.toml` patch; no runtime / API / type changes
- capture: 2026-07-01

## Ubiquitous Language

This bead's domain is **workspace metadata governance**, not application logic.
The terminology is therefore TOML-and-Cargo, not Rust-runtime:

| Term | Meaning in this bead |
| --- | --- |
| **Workspace** | The root `Cargo.toml` at the repo root, declaring `[workspace.members]`, `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints.rust]`, and `[workspace.lints.clippy]`. Single source of truth for shared metadata. |
| **Member Manifest** | A `Cargo.toml` under `crates/<name>/` referenced from `[workspace.members]`. Editable by agent; here, `crates/vb_queue_semantics/Cargo.toml`. |
| **Inheritance Field** | A `[package]` (or other) key written as `field.workspace = true` to bind to the workspace table. The `.workspace` form is Cargo's typed inheritance marker. |
| **Lint Block** | A `[lints]\nworkspace = true` opt-in table that imports the root `[workspace.lints.rust]` and `[workspace.lints.clippy]` deny sets en bloc. |
| **Lint Policy** | The pair of deny tables at workspace root. Authoritative for every member that opts in. |
| **Drift** | A member manifest that diverges from the workspace inheritance pattern (literal `version` instead of `version.workspace`, missing `[lints]` block, or harder-to-spot cargo behaviours). |
| **Sibling Pattern** | The shared formatting convention used by `vb_cli`, `vb_compile`, `vb_core`, `vb_ipc`, `vb_runtime`, `vb_storage`, `vb_validate`, and `workspace_tests` — `version.workspace = true` near top of `[package]`, `[lints]\nworkspace = true` as the final table in the file. |
| **Metadata-Only Patch** | A change that touches only Cargo manifest metadata (fields, blocks, inheritance markers). By construction it cannot alter runtime semantics; it can alter build-system acceptance. |
| **Build-Acceptance Surface** | The set of source constructs the workspace lint policy accepts. Adding a workspace `[lints]` opt-in expands this surface for `vb_queue_semantics` from `#![deny(...)]` headers only to the full workspace deny set. |

## Actors and Roles

| Actor | Role |
| --- | --- |
| **Workspace Author** | Maintainer of root `Cargo.toml`. Source of `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints.*]`. Not edited by this bead. |
| **Member Author** | Owner of `crates/vb_queue_semantics/Cargo.toml`. Edited by this bead. |
| **Cargo Build Engine** | Resolves `[package]` against `[workspace.package]`; resolves `[lints] workspace = true` against the workspace deny tables. Sets enforcement level. |
| **Holzman-Rust Implementer** | Applies the patch and runs the build gates. |
| **Black-Hat Reviewer** | Reads the diff and re-runs `cargo metadata` + `cargo clippy`. |

## Value Objects (the metadata entities)

VO-1 **PackageVersion**:

- Workspace-level: a single string `"0.1.0"` declared once in `[workspace.package].version`.
- Member-level: an inheritance marker `version.workspace = true`.
- Forbidden at member level: a literal `version = "X.Y.Z"` (drift event).

VO-2 **PackageEdition**:

- Workspace-level: `"2024"`.
- Member-level: `edition.workspace = true`. Already correct in target crate. Out of scope to change.

VO-3 **PackageLicense**:

- Workspace-level: `"MIT OR Apache-2.0"`.
- Member-level: `license.workspace = true`. Already correct in target crate. Out of scope to change.

VO-4 **LintsBlock**:

- Shape: a TOML table header `[lints]` followed immediately by `workspace = true` on the next line.
- Forbidden variants: nested sub-tables, alternative keys, per-axis selective opt-in. The shape must match every sibling crate verbatim.

VO-5 **MemberManifest**:

- The editable file. Bounded to lines 1-N where N is currently 11. After this bead, lines 1-N' where N' is 13 and `[lints]\nworkspace = true` occupies lines 12-13.

VO-6 **PublishFlag**:

- Member declares `publish = false`. Out of scope to change. Held invariant for downstream reviewer parity with `xtask/src/forbidden_scan.rs`.

## Aggregate / Single Edit Unit

Edit unit = `crates/vb_queue_semantics/Cargo.toml` (single file). The patch is:

1. **Replace** line 3 with `version.workspace = true`.
2. **Append** a `[lints]` table whose sole key is `workspace = true` after line 11 (after the existing `[dependencies]` table, matching the ordering convention used by `vb_cli`, `vb_core`, `vb_compile`, `vb_ipc`, `vb_runtime`, `vb_storage`, `vb_validate`).

No other fields move. No dependencies added. No features toggled. No `[[bin]]`/`[[test]]`/`[[bench]]` blocks added.

## Commands and Events

| Command | Trigger | Outcome on success |
| --- | --- | --- |
| `replace_version_line` | After scout marks `version = "0.1.0"` as drift | `version.workspace = true` on line 3 |
| `append_lints_block` | After replace | Two new lines (`[lints]` and `workspace = true`) after the last existing line |
| `cargo check -p vb_queue_semantics --all-targets` | Post-patch build acceptance | exit 0 |
| `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` | Post-patch lint acceptance | exit 0 |
| `cargo test -p workspace_tests --test vb_8ma2_workspace_assertions` | Workspace assertion gate | passes |
| `cargo test -p workspace_tests --test vb_qi37_25_quality_gates` | Quality-gate assertion | passes |

## Policies (Hard Invariants)

POL-1 **Inheritance-first**: A member that declares a literal `version` while `[workspace.package].version` exists is a P1 governance drift.
POL-2 **Lint parity**: Once `[workspace.lints.*]` is declared, every member MUST opt in via `[lints]\nworkspace = true`. No silent opt-out.
POL-3 **Sibling literalism**: The shape of the `[lints]` block SHALL be character-identical to the sibling crates' block (single key, single value, two lines).
POL-4 **No new dependency, no new feature, no new bin/test/bench/lib target**. The patch is metadata-only.
POL-5 **Pre-existing source-length exception `vb-2lu1` at `.config/source-length-exceptions.txt:323` SHALL remain untouched**; the 427-line cap on `crates/vb_queue_semantics/src/lib.rs` is a separate concern.

## Forbidden States (made unrepresentable by the patch)

F-1 `crates/vb_queue_semantics/Cargo.toml::[package].version = "0.1.0"` after this bead lands.
F-2 `crates/vb_queue_semantics/Cargo.toml` without a `[lints]` table after this bead lands.
F-3 `crates/vb_queue_semantics/Cargo.toml::[package]` containing `version.workspace = false` (illegal — inherits-or-not is a Cargo invariant; cannot be false while `[workspace.package]` exists).
F-4 `crates/vb_queue_semantics/Cargo.toml::[lints]` containing per-axis overrides like `rust = ...` (sibling pattern is `[lints]\nworkspace = true` only).

## Open Domain Questions

None. The bead is bounded: the scout enumerated the reference patterns; sibling crates give the canonical shape; workspace version happens to match the hardcoded value, so the patch is semver-equivalent on the version axis while gaining inheritance on the metadata axis.

## Out-of-Scope (Recorded to Block Re-Reach)

OOS-1 `vb_queue_semantics/src/lib.rs` source-length refactor — owned by `vb-2lu1`.
OOS-2 `vb_queue_semantics/src/lib.rs` content refactor or split — not requested.
OOS-3 Verus helper generator scope (`scripts/generate_queue_state_verus_helpers.py`) — read-only with respect to Cargo.toml.
OOS-4 Adding new clippy lint levels — would change workspace policy; separate bead.
