# Proof-to-Implementation Input — vb-jpq7.21

STATUS: PLANNED. This is bridge input, not bridge approval.

## Production source refs

- `crates/vb_ipc/src/payloads.rs:45-57` — repaired `IpcPayload::AnswerAsk` fields.
- `crates/vb_ipc/src/server/handlers.rs:174-228` — top-level handler decode/bounds/SlotValue/taint/runtime call.
- `crates/vb_ipc/src/server/handlers/command.rs:16-69` — command handler decode/bounds/SlotValue/taint/runtime call.
- `crates/vb_runtime/src/runtime_actions.rs:93-113` — public `answer_pending_ask_slot` API.
- `crates/vb_runtime/src/runtime_actions.rs:159-201` — pending Ask ticket derivation and AskResume answer slot validation.

## Existing behavior tests to keep linked

- `crates/vb_ipc/src/tests.rs:603-618` — deterministic AnswerAsk roundtrip.
- `crates/vb_ipc/src/tests.rs:1274-1306` — zero/max answer slot deterministic adversarial roundtrips.
- `crates/vb_cli/tests/cli_integration.rs:4082-4168` — CLI emits AnswerAsk run_id/answer_slot/answer/taint None.
- `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:613-662` — legacy direct AskAnswer resume/AskAnswered bridge context.

## Planned future artifacts

- `crates/vb_runtime/src/verification/kani/kani_answer_ask_slot_semantics.rs`
  - `answer_slot_equality_accepts_only_exact_ask_resume_slot`
  - `pending_ask_ticket_derivation_rejects_invalid_shard_states`
- `crates/vb_runtime/src/verification/proptest/proptest_answer_ask_slot_semantics.rs`
  - `vb_jpq7_21_answer_pending_ask_slot_generated`
- `crates/vb_ipc/tests/vb_jpq7_21_answerask_payload_props.rs`
  - `vb_jpq7_21_answerask_payload_roundtrip_generated`
- Existing fuzz targets to refresh/seed if needed:
  - `fuzz/src/bin/ipc_decode.rs`
  - `fuzz/src/bin/ipc_frame_fuzz_boundary.rs`

## Non-negotiable bridge checks

- No proof may rely on a removed legacy `ticket` field in `IpcPayload::AnswerAsk`.
- Kani generators must cover invalid as well as valid shapes; no fixed dummy workflow/run frame.
- Mismatched `answer_slot` must map to `RuntimeError::InvalidActionCompletion` and must not enqueue `AskAnswered`.
- Missing `taint` from IPC must default to `Taint::Clean`; explicit taint must pass through.
- Handler malformed postcard `SlotValue` bytes must return an error response before runtime mutation.
