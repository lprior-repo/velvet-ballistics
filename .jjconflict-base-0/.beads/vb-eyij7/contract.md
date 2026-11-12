# Contract — `vb-eyij7`

## Decision

`velvet-ballistics-MASTER.md` is authoritative under repository `AGENTS.md`. Reconcile the actual repository to §34’s exact workspace contract; do not amend the master merely to preserve accidental current drift. Coordinate parser extraction with `vb-l5dzb` so the YAML and expression packages have one implementation owner.

## Required member set

The root workspace’s exact active member set is:

1. `crates/vb_core`
2. `crates/vb_yaml`
3. `crates/vb_validate`
4. `crates/vb_expr`
5. `crates/vb_compile`
6. `crates/vb_storage`
7. `crates/vb_runtime`
8. `crates/vb_ipc`
9. `crates/velvet_ballistics`
10. `crates/workspace_tests`
11. `fuzz`

No extra root member is allowed. The product package/binary is `velvet-ballistics`; its Rust library crate/module is `velvet_ballistics`.

## Ownership and compatibility

- Extract YAML implementation from `vb_compile` into `vb_yaml` and expression implementation into `vb_expr` according to the master function surfaces. `vb_compile` owns workflow compilation/lowering and consumes these package APIs.
- Keep `vb_validate` as the existing dedicated validation crate.
- Move the current product CLI/library/binary from `crates/vb_cli` behind `crates/velvet_ballistics`, preserving documented CLI behavior and any required Rust API compatibility.
- Remove `vb_queue_semantics` from active workspace membership unless the master is explicitly revised by an authorized contract update. Its current manifest describes a stub, not an active required architecture crate.
- Make `fuzz` a real root member and reconcile its nested `[workspace]` declaration, Cargo dependencies, target catalog, and `cargo-fuzz` commands.
- Preserve existing source/API behavior through a single canonical owner and temporary re-exports where needed; do not retain shadow implementations.

## Acceptance contract

1. A structural assertion proves exact workspace member equality against this list.
2. Cargo metadata resolves all packages and path dependencies without duplicates or unresolved edges.
3. Canonical package, library, and binary names match repository naming rules.
4. Existing YAML, expression, validation, compile, CLI, fuzz, and workspace-integration test targets remain reachable and pass in focused lanes.
5. Fuzz target listing/build follows the master fuzz conventions.
6. `moon ci` passes from the isolated workspace; all failures are classified and repaired at their owning layer.
7. No performance claim is introduced without benchmark evidence. No proof lane is closed by source/file presence alone.

## Explicitly out of scope

Changing YAML/expression semantics, runtime behavior, storage formats, IPC protocols, or performance design. Any such change requires a new behavior contract and risk-specific proof/test plan.

## Current decision risks

The exact migration ordering and compatibility window for downstream external consumers are not discoverable from the repository inputs. Preserve public paths during extraction unless a concrete consumer audit and explicit approval support their removal. Coordinate `vb-l5dzb` before any overlapping source move.
