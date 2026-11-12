# Source/API exploration and boundary map

Existing production source at base57f:
- vb_core/src/action/contract.rs:83: ABI declarations only, no executor metadata.
- vb_core/src/action/ticket.rs:6: full numeric ticket; deterministic key helper.
- vb_runtime/src/action.rs:189: dispatch_generic returns Suspended, executes no
  external adapter; this cannot generate external conformance evidence.
- vb_runtime/src/idempotency.rs:97: bounded in-memory dispatch/completion tracking
  can evict entries. It cannot establish durable provider idempotency.
- vb_runtime/src/runtime.rs:862/900: typed completion and canonical pending-ticket
  lookup are existing public bridges to a future adapter.
- vb_storage/src/admission/verification.rs:147: Strict side effects fail without
  conformance; retain this default until the actual execution authority is wired.
- vb_storage/src/batch/: bounded durable batch infrastructure, no current generic
  action receipt store or executor adapter abstraction found by targeted search.

Core: pure manifest/pack/receipt/classification types and deterministic checks.
Cold compile/admission: parse, exact contract/pack digest binding, resolve trusted
conformance evidence, explicit schema compatibility.
Executor shell: dedicated local Fjall adapter, bounded synchronous execution,
atomic target+receipt batch and durability barrier. No core async or network.
Runtime: numeric accepted action binding, durable scheduled ticket and current
capability/resource authorization before adapter invocation; receipt recovery
never dispatches effects while replaying.

Proposed APIs (names not committed): check_action_pack(raw, ExecutorCatalog,
ConformanceResolver), prepare_artifact_with_action_pack(..., CheckedActionPack),
execute_local_register(ScheduledTicket, AuthorizedResource, BoundedInput),
reconcile_receipt(ReceiptIdentity). Legacy prepare APIs retain fail-closed behavior.

Trust boundary needing root decision: where ExecutedConformance is published and
resolved as trusted local evidence. Arbitrary deserialized reports cannot become
authority merely because all hashes are internally consistent. No such resolver
currently exists; avoid creating another self-attested verification path.
