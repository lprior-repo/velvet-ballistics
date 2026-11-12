# Codebase Map — vb-2b4g

## Target IR Definitions

- `crates/vb_core/src/workflow/mod.rs`: `CompiledNodeKind` definitions for `Together*`, `Collect*`, `Reduce*`, and `Repeat*`.
- `crates/vb_core/src/nodes.rs`: mirrored node-family definitions.

## Runtime Oracle

- `crates/vb_core/src/engine/step.rs`: basic core interpreter returns `UnsupportedPrimitive { primitive: "not_yet_implemented" }` for target families. It is not the parity oracle for this bead.
- `crates/vb_runtime/src/engine/drive.rs`: `drive_deterministic_full` executes full runtime semantics.
- `crates/vb_runtime/src/engine/execute.rs`: dispatches target families to runtime primitives.
- `crates/vb_runtime/src/primitives/repeat.rs`: packed repeat state, attempt increment/routing, finish taint copy.
- `crates/vb_runtime/src/primitives/reduce.rs`: accumulator initialization, list first/tail binding, next iteration, finish taint copy.
- `crates/vb_runtime/src/primitives/together.rs`: sequential branch routing, accumulator list append, branch-count tracking, final append/join taint.
- `crates/vb_runtime/src/primitives/collect.rs`: paginated collection state, current-page validation, duplicate/stale/out-of-order detection, terminal page, finish taint copy.

## Generated Code Surface

- `crates/vb_codegen/src/lib.rs`: `unsupported_node_feature` currently rejects target families before emission.
- `crates/vb_codegen/src/lib.rs`: emission dispatch already routes to `emit_together_step_body`, `emit_reduce_step_body`, `emit_repeat_step_body`, and `emit_collect_step_body` if validation allows it.
- `crates/vb_codegen/src/generated_storage_helpers.rs.txt`: list helper support includes `append_list_item`, `clone_list_items`, `tail`, and `collect_page_handle`.

## Tests

- `crates/vb_codegen/src/tests.rs`: current target-family tests assert fail-closed rejection.
- Existing runtime helper functions near the top of `tests.rs` use `drive_deterministic_full` and should be preferred over `vb_core::run_until_blocked` for target-family parity.
- Existing workflow builders for target families are shallow and may require realistic slot/output wiring before they can prove parity.

## Main Risks

- Do not use `vb_core::run_until_blocked` as the oracle for target families.
- Do not count source-substring or support-owner checks as semantic parity.
- `Collect*` requires generated pagination side state to match runtime duplicate/stale/multi-page semantics.
- Current generated `TogetherJoin` and `CollectNext` are known incomplete.
