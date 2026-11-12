# Contract Specification: vb-qi37.26.1

## Context
- **Feature:** Fix vb_ipc typed handler compile errors (E0308) blocking workspace-tests compilation.
- **Bead ID:** vb-qi37.26.1
- **Prerequisite for:** vb-qi37.26
- **Root cause:** A module split/restore cycle left stale String-based code in `crates/vb_ipc/src/server/handlers.rs` where strongly-typed enum variants were expected (`EdgeType`, `PassFail`, `GateKind`, `NodeKind`, `TaintPathStatus`).
- **Fix status:** Already applied in commit `0ebc5270`; current code compiles cleanly.

## Domain Terms
- **Typed enum:** One of `EdgeType`, `PassFail`, `GateKind`, `NodeKind`, `TaintPathStatus` defined in `crates/vb_ipc/src/payloads.rs`. These are `Serialize`/`Deserialize` enums with wire-format naming via `serde(rename_all)`.
- **String-based code:** Stale code that passed `&str` or `String` where an enum variant was required, causing E0308 mismatches.
- **Orphaned file:** A `.rs` file under `crates/vb_ipc/src/server/handlers/` that is not referenced by any `mod.rs` and therefore not compiled.

## Assumptions
- The repository uses a pinned nightly Rust toolchain (see `docs/rust-governance.md`).
- `cargo check` is sufficient for compilation validation; linking is not required for this contract.
- The orphaned handler files (`command.rs`, `event.rs`, `query.rs`, `session.rs`) are intentionally excluded from the build at this time and must not be deleted or wired in as part of this bead.

## Open Questions
- None. The fix is already applied and verified.

## Preconditions
- PRE-001: The workspace checkout must be clean with no uncommitted changes that could affect compilation.
- PRE-002: The Rust toolchain must match the repository's pinned nightly version.

## Postconditions
- POST-001: `cargo check -p vb_ipc` exits with code `0` and zero errors.
- POST-002: `cargo check -p velvet-ballistics-workspace-tests --tests` exits with code `0` and zero errors.
- POST-003: `cargo clippy -p vb_ipc -- -D warnings` exits with code `0` (source lint zero tolerance).
- POST-004: No new `unsafe`, `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` are introduced in the changed code.

## Invariants
- INV-001: **Type Consistency** -- `handlers.rs` must reference enum variants (e.g., `crate::EdgeType::Branch`) rather than string literals (e.g., `"branch"`) for all typed IPC payload fields.
- INV-002: **Compilation Isolation** -- Orphaned files in `crates/vb_ipc/src/server/handlers/` must remain unreferenced by the module tree and must not break compilation.
- INV-003: **Safety Preservation** -- The fix must not introduce any `unsafe` blocks or panicking APIs (`unwrap`, `expect`, `panic`, `todo`, `unimplemented`).

## Error Taxonomy
- `Error::CompileE0308` -- Type mismatch between `String`/`&str` and an enum variant. This is the error being fixed.
- `Error::ClippyWarning` -- Any clippy warning treated as an error by `-D warnings`.
- `Error::SafetyRegression` -- Introduction of `unsafe`, `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` in the fix.
- `Error::OrphanedFileLeak` -- An orphaned file is accidentally wired into the module tree and causes new compile errors.

## Contract Signatures
This is a compile-fix bead; there are no new public APIs. The contract is expressed as compilation gates:
- `fn verify_vb_ipc_compiles() -> Result<(), CompileError>`
- `fn verify_workspace_tests_compiles() -> Result<(), CompileError>`
- `fn verify_no_safety_regression() -> Result<(), SafetyRegression>`

## Verus-Owned Clauses
- None. This bead is purely compilation/type-correctness. No new pure Rust-core logic is introduced that requires Verus proof.

## TLA+-Owned Clauses
- None. No temporal workflow, protocol, scheduler, or lifecycle behavior is modified.

## Theorem-Owned Clauses
- None. No theorem kernel is required for a type-mismatch compile fix.

## Non-goals
- Deleting or refactoring the orphaned handler files.
- Changing any functional behavior of the IPC handlers.
- Adding new tests or benchmarks.
- Verifying runtime behavior (this is a compile-only fix).
- Performance claims (no hot paths were modified).
