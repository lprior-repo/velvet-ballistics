# Trusted Base Plan — vb-jpq7.21 AnswerAsk IPC/runtime semantic delta

## In scope

- `IpcPayload::AnswerAsk { run_id, answer_slot, answer, taint }` postcard shape.
- IPC handler decode/size/value/taint checks and call to `Runtime::answer_pending_ask_slot`.
- Runtime derivation of `AskTicket` from shard run state, pending Ask timer, Ask successor, and `AskResume { answer }` equality with requested `answer_slot`.
- Enqueue intent for `ShardCommand::AskAnswered` on valid state only.

## Trusted or abstracted surfaces

1. **Postcard/serde internals** are trusted for byte-level serialization correctness. Fuzz/proptest exercise the boundary but do not prove postcard itself.
2. **Shard queue internals** are trusted below enqueue/no-enqueue observability. Kani/proptest must prove the decision to enqueue, not lock-free queue implementation.
3. **CompiledWorkflow construction invariants** are trusted when supplied by compiler/validator. Kani/proptest bounded generators may construct minimal valid/invalid node graphs for this semantic seam.
4. **SlotValue semantic meaning** is out of scope. This bead verifies answer routing, taint propagation/defaulting, and encoded length; it does not prove each SlotValue variant's downstream interpretation.
5. **Runtime scheduler timing** is out of scope. The pending timer is treated as existing shard state; no Loom obligation is planned because no new scheduler interleaving is introduced.
6. **No unsafe/UB trusted code** is added in scoped first-party files; Miri is not applicable unless proof-writer introduces unsafe/raw pointer/FFI code, which would violate repository policy.

## Required reviewer checks

- Kani harnesses must not hardcode one dummy workflow. They must generate valid and invalid bounded state shapes.
- Verus must stay not-applicable unless proof-writer can bind specs to production exec functions; mirror-only Verus models are forbidden.
- Flux remains not-applicable until Flux annotations/extern specs exist for the runtime/IPC types in this scope.
- Fuzz evidence must include hostile IPC bytes and must not be replaced by deterministic roundtrip tests.
