# Proof-to-Implementation Input — vb-jpq7.21

STATUS: PLANNED bridge input, not approval.

## Production refs

- `crates/vb_ipc/src/payloads.rs:45-57` — repaired AnswerAsk payload shape.
- `crates/vb_ipc/src/server/handlers.rs:174-228` — AnswerAsk handler decode, bounds, SlotValue decode, taint defaulting, encoded length, runtime call routing.
- `crates/vb_runtime/src/runtime_actions.rs:93-201` — `answer_pending_ask_slot`, pending Ask ticket derivation, AskResume slot equality.
- `crates/vb_cli/src/run_ops.rs:168-200` — CLI local malformed SlotValue validation rejects invalid bytes before IPC.
- `crates/vb_cli/src/run_ops.rs:228-237` — CLI slot-value AnswerAsk payload construction and IPC send.

## Required behavior bridge tests

The proof bridge must cite and execute obligations for: valid postcard SlotValue plus default clean taint; mismatched handler answer slot without consuming pending ask; absent pending ask; malformed SlotValue bytes before runtime mutation; valid runtime completion; runtime mismatch without advancing pending ask; absent run; action-suspended non-ask state; wait-timer non-ask state; CLI slot-value happy path over IPC; and local malformed CLI rejection without an IPC server.

## Required bounded Kani bridge

`obl-vb-jpq7-21-kani-handler-runtime-bridge-012` must be implemented by proof-writer as a production-bound bounded handler/runtime bridge harness. It must cover hostile bytes, malformed SlotValue rejection before runtime mutation, answer byte bounds, missing taint defaulting to Clean, explicit taint propagation, valid routing to `Runtime::answer_pending_ask_slot`, and mismatched slot rejection without consuming the pending ask; it may not use one hardcoded fixture or copied handler logic.

## Required generated property bridge

`obl-vb-jpq7-21-proptest-handler-bridge-020` must be implemented by proof/test writer as a production-bound generated property artifact. It must generate SlotValue bytes, malformed byte vectors, answer_slot equality/mismatch, taint None/Some, encoded length cases, and rejection-before-runtime-mutation observations; it may not replace behavior with a copied-only model.

## Non-negotiable checks

- No proof may rely on a removed legacy `ticket` field in `IpcPayload::AnswerAsk`.
- Kani must use bounded generators/`kani::any()` style coverage, not a fixed dummy `WorkflowParts`/`RunFrame`.
- Mismatched `answer_slot` rejects before `AskAnswered` enqueue or pending ask advancement.
- Missing IPC taint defaults to `Taint::Clean`; explicit taint passes through.
- Malformed postcard `SlotValue` bytes return an IPC error before runtime mutation.
- Handler obligations must demonstrate `Runtime::answer_pending_ask_slot` routing and encoded-length accounting.
