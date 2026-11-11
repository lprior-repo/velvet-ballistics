# vb-b5f.2.1 Contract

The master phase 1 scalar API requires `WorkflowId::as_u32(self) -> u32` and `RunId::as_u64(self) -> u64`; each is a public `const`, `#[must_use]` getter that forwards to the existing `get()` method.

Both aliases preserve the existing `WorkflowId(u32)` and `RunId(u64)` representations, constructors, `get()` API, serde behavior, and postcard wire bytes; no macro-wide changes, casts, truncation, or persisted-format changes are in scope.

Given IDs constructed with raw values 0, 1, and the respective integer maximum, each alias returns that independently expected raw value, and the existing `get()` returns the same value.

Compile-time constants call each alias and compare its result with the same ID's existing `get()` result; compile-time construction proves the methods are const-compatible.

For both ID types and all three boundary values, postcard encoding of the typed ID equals encoding the raw integer, and decoding the typed bytes returns the original typed ID.

Acceptance mapping: `scenario_workflow_id_alias_preserves_boundary_values_and_const_use` covers WorkflowId boundaries and const use; `scenario_run_id_alias_preserves_boundary_values_and_const_use` covers RunId; `scenario_workflow_id_wire_bytes_remain_unchanged` and `scenario_run_id_wire_bytes_remain_unchanged` cover typed/raw encoding parity and typed decoding.

The source change is limited to two small forwarding methods in `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs`; focused native nextest and existing `vb_core` ID regression tests are the scoped evidence. Sol CODE, separate Sol TEST, and integrated phase acceptance remain pending.
