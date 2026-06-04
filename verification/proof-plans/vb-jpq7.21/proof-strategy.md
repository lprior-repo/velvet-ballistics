# Proof Strategy — vb-jpq7.21 AnswerAsk IPC/runtime semantic delta

STATUS: PLANNED. No verifier, test, fuzz, CI, or proof success is claimed here.

## Scope

This repaired plan covers the behavior-affecting AnswerAsk semantic delta: `IpcPayload::AnswerAsk { run_id, answer_slot, answer, taint }`, IPC handler SlotValue decode/size/taint defaulting, routing to `Runtime::answer_pending_ask_slot`, runtime derivation of `AskTicket` from pending Ask state, and exact `AskResume.answer == answer_slot` rejection-before-mutation semantics.

## Risk classification

Temporal/state-machine, Rust-local invariant, bounded state, refinement/type-state, untrusted input, IPC codec, and release-critical gates are in scope. Concurrency/Loom and unsafe/Miri triggers are not present because this bead adds no locks, atomics, tasks, cancellation, raw pointers, FFI, or unsafe code.

## Lane policy application

Default Rust behavior lanes are recorded per seed for `kani`, `verus`, `flux-rs`, and `proptest`. Required obligations are planned for Kani/proptest where they bind to production runtime or IPC seams. The IPC handler/runtime bridge Kani lane is required because the seam has bounded Rust control flow over hostile IPC bytes, SlotValue decode, answer byte bounds, taint defaulting/propagation, fail-closed rejection-before-runtime-mutation, and Runtime::answer_pending_ask_slot routing; the proptest lane is also required and planned for generated SlotValue bytes, malformed bytes, answer_slot equality/mismatch, taint defaulting/propagation, encoded length, and rejection-before-runtime-mutation. Verus and Flux-rs are recorded as `not_applicable` per seed with concrete source/tooling evidence rather than omitted or waived for behavior. `cargo-fuzz` is required for IPC/hostile-input seeds. Cargo behavior-test obligations are split into one exact test filter per command. `moon ci` remains the release gate.

## New behavior tests cited as planned evidence

- `handle_answer_ask_accepts_valid_postcard_slot_value_and_default_clean_taint`
- `handle_answer_ask_rejects_mismatched_answer_slot_without_consuming_pending_ask`
- `handle_answer_ask_rejects_absent_pending_ask`
- `handle_answer_ask_rejects_malformed_slot_value_bytes_before_runtime_mutation`
- `answer_pending_ask_slot_accepts_matching_answer_slot_and_completes_run`
- `answer_pending_ask_slot_rejects_mismatched_answer_slot_without_advancing_pending_ask`
- `answer_pending_ask_slot_rejects_absent_pending_ask_for_unknown_run`
- `answer_pending_ask_slot_rejects_action_suspended_non_ask_state`
- `answer_pending_ask_slot_rejects_wait_timer_non_ask_state`
- `cli_answer_slot_value_happy_path_uses_ipc_answer_ask`
- `cli_answer_rejects_malformed_slot_value_without_ipc_server`

## Waiver posture

No behavior-affecting waiver is made. `waiver-candidates.jsonl` contains only non-behavior proof-infrastructure candidates for not materializing new Flux/Verus infrastructure in this planning pass, with repair triggers and compensating required obligations.
