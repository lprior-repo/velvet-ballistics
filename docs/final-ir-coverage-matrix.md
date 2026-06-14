# Final IR Coverage Matrix

This matrix records generated-mode parity for `vb-qi37.10.1` and the related core-engine beads. Status reflects the focused evidence captured in this worktree.

| IR surface | Interpreter behavior | Generated behavior | Evidence | Status |
|---|---|---|---|---|
# allow-removed-crate: coverage matrix references the removed codegen crate as a historical test target
| `Nop`, `SetConst`, `Copy`, `Jump`, `Finish` | Linear step execution updates pc, slots, taints, or terminal result. | Linear step functions emit checked slot reads/writes and typed pc/result outcomes. | `rtk cargo test -p vb_codegen -p vb_core -p vb_runtime -p vb_storage --all-features`: 3861 passed. | Historical covered |
# allow-removed-crate: coverage matrix references the removed codegen crate as a historical test target
| Primitive expressions | Interpreter evaluates typed primitive ops with exact errors. | Generated `eval_expr_*` emits checked stack ops and typed `DriveError` paths. | `rtk cargo test -p vb_codegen --all-features`: 299 passed. | Historical covered |
# allow-removed-crate: coverage matrix references the removed codegen crate as a historical test target
| Expression taint | Interpreter joins taint through expression values. | Generated expression functions return `(SlotValue, Taint)` and write returned taint. | `crates/vb_codegen/src/lib.rs` expression emission plus focused codegen suite. | Historical covered |
# allow-removed-crate: coverage matrix references the removed codegen crate as a historical test target
| `BuildObject` | Interpreter stores deterministic object handles and joins field taints. | Generated object tests in `vb_codegen`; full focused suite. | Historical covered |
# allow-removed-crate: coverage matrix references the removed codegen crate as a historical test target
| `BuildList` | Interpreter stores deterministic list handles and joins item taints. | Generated list store enforces record/value capacity and writes deterministic item order. | Generated list tests in `vb_codegen`; full focused suite. | Historical covered |
| Helper expressions: `contains`, `starts_with`, `ends_with`, `length`, `empty` | Interpreter applies symbol-text helpers when runtime symbol text is available; `length`/`empty` also accept symbols. | Generated mode fails closed for these helpers because `CompiledWorkflow` carries only `symbols_count`, not symbol strings or a runtime `ValueStore`, and has no static operand-type proof for `length`/`empty`. | Generated-mode helper rejection tests. | Rejected before generation |
| Helper expressions: `has`, `exists`, `sum`, `count` | Interpreter applies list membership, null existence, list sum, and list count with typed errors. | Generated mode supports `Has` as list-contains-item, `Exists` as non-null, and list `Sum`/`Count` with checked stores and typed errors. | Generated helper emission tests and focused codegen suite. | Covered |
| Helper expressions: `append`, `append_if`, `merge`, `unique` | Interpreter allocates derived list/object handles with taint joins. | Generated allocates derived store records with bounded capacity and item/object taint propagation. | Updated helper capacity and taint emission; focused codegen suite. | Covered |
| `Choose` | Interpreter branches by bool expression or slot condition. | Generated evaluates bool condition with taint-aware expression call and emits deterministic branches. | Choose generated tests and proptests. | Covered |
| `ForEach` | Runtime/interpreter path preserves deterministic iteration and joins. | Generated iterator support emits checked cursor/slot transitions for supported mode. | ForEach generated tests and focused suite. | Covered |
| `Do` action boundary | Interpreter suspends with pending action metadata. | Generated returns typed `SuspensionOutcome::ActionPending` through `DriveError::ActionSuspend`; it does not continue past the boundary. | `emit_step_match_produces_correct_arm_for_do_node`, `step_do_action_emits_suspend`. | Covered |
| `WaitUntil` | Interpreter suspends until deadline and preserves resume pc. | Generated reads deadline and returns typed wait suspension with `step`, `deadline_slot`, and `resume_pc`. | `step_wait_until_with_next_emits_continue`, `edge_case_wait_until_step_generates_wait_code`. | Covered |
| `WaitEvent` | Interpreter suspends on event/timeout and preserves resume pc. | Generated reads event/optional timeout and returns typed wait-event suspension. | `step_wait_event_with_next_and_timeout`, timeout/no-timeout tests. | Covered |
| `Ask` / `AskResume` | Interpreter suspends for ask and resumes through answer slot. | Generated returns typed ask suspension with prompt, timeout, and resume pc; resume emits checked answer write. | Ask generated tests and focused suite. | Covered |
| `RetryCheck` | Runtime/interpreter decodes retry state and branches by attempts. | Generated decodes retry state and validates max-attempt policy with typed errors. | RetryCheck generated trace/parity tests. | Covered |
| Error handlers | Interpreter routes errors to handler step/error slot when present. | Generated wraps handler step bodies and routes `DriveError` through handler metadata. | Error-handler emission tests. | Covered |
| Unsupported generated mode: `Together`, `Reduce`, `Repeat`, `Collect` | Interpreter/runtime may have non-generated support or typed unsupported paths. | Generated subset rejects these before emission with `UnsupportedIr` instead of runtime fallback. | Unsupported generated-mode tests; collect validation now rejects collect variants. | Covered |

Known gate caveat: canonical `moon ci` is blocked in this environment by the supply-chain store acquisition error captured in QA evidence and explicitly waived for this run. Focused codegen/core/runtime/storage tests and scoped production-library clippy pass.
