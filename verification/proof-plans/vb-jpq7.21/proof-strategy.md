# Proof Strategy — vb-jpq7.21 AnswerAsk IPC/runtime semantic delta

STATUS: PLANNED. No verifier, test, fuzz, or CI execution is claimed here.

## Scope

This plan covers only the repaired AnswerAsk semantic delta:

- `IpcPayload::AnswerAsk` is `{ run_id, answer_slot, answer, taint }`.
- IPC handlers decode AnswerAsk and call `Runtime::answer_pending_ask_slot`.
- Runtime derives `AskTicket` from shard state, requires a pending `PendingTimerKind::Ask`, resolves the resume step from the Ask node, validates `AskResume { answer } == answer_slot`, and enqueues `AskAnswered` only for valid state.

Out of scope: production/test code edits by this planning pass, scheduler concurrency redesign, SlotValue semantics beyond decode/pass-through, postcard implementation internals, and legacy `answer_ask(AskAnswer)` ticket behavior except as bridge context.

## Risk classification

- Temporal/state-machine: applicable; pending Ask timer and resume-step state machine is central.
- Rust-local invariant: applicable; slot equality and enum shape are local Rust invariants.
- Bounded state: applicable; finite node/timer/slot cases fit Kani and proptest.
- Refinement/type-state: applicable conceptually, but Flux/Verus are not tooling-fit for current source annotations.
- Concurrency: not applicable; no new locks, atomics, tasks, cancellation, or interleavings in this delta.
- Unsafe/UB: not applicable; scoped first-party files forbid unsafe code.
- Untrusted input: applicable; IPC binary payload is hostile input.
- Dependency/supply-chain: not triggered; no dependency changes are in this scope.
- Performance: not triggered; no speed claim.
- Release-critical gate: applicable; run `moon ci` after focused obligations.

## Lane choices

Required lanes:

- **Kani** for bounded runtime answer-slot equality and ticket derivation over generated shard/workflow/timer states.
- **Proptest** for randomized IPC AnswerAsk roundtrip and valid/invalid answer-slot behavior.
- **cargo-fuzz** for hostile IPC frame/payload byte boundary (`ipc_decode`, `ipc_frame_fuzz_boundary`).
- **Cargo behavior tests** as bridge evidence to existing deterministic tests and future focused handler/runtime tests.
- **moon ci** as release gate after focused proof/test/fuzz obligations.

Not applicable lanes:

- **Flux**: no scoped Flux annotations/features for Runtime/IpcPayload/CompiledWorkflow; existing Flux is action-ticket-specific.
- **Verus**: no production-bound Verus requires/ensures for scoped exec functions; mirror-only Verus would violate no-vacuum-Verus rule.
- **Loom**: no new concurrency/interleaving surface.
- **Miri**: no unsafe/provenance/UB surface.

## Bridge to existing behavior

Existing tests already prove useful but insufficient facts:

- `payload_roundtrip_preserves_answer_ask_variant` and adversarial `AnswerAsk` roundtrips cover deterministic IPC shape examples.
- `cli_answer_slot_value_happy_path_uses_ipc_answer_ask` proves CLI emits `AnswerAsk { run_id, answer_slot, answer, taint: None }`.
- `test_direct_api_answer_ask_resumes_suspended_run` proves legacy direct `answer_ask(AskAnswer)` can resume and emit `AskAnswered`; it does **not** close the new `answer_pending_ask_slot` derivation proof.

Planned future harnesses must add explicit mismatch/no-timer/non-Ask/missing-resume coverage for `answer_pending_ask_slot`.

## Acceptance for proof-plan-reviewer

The plan is complete only if reviewer accepts `verifier-lane-decisions.jsonl`, every required obligation in `proof-obligations.planned.jsonl`, and the trusted-base exclusions above. Proof success is deliberately not claimed.
