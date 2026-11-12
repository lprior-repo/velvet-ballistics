# vb-l5ru0 Rust contract
Parent vb-ny068. Master77.3/77.11. This actual Runtime effect-boundary gate consumes sibling vb-z7ky4 strict journal port. Preserve existing RuntimePolicy durability field and all existing execute/reconcile public signatures.

## Proposed exact sibling interface
Shard::append_policy_decision_strict(&mut self, decision: vb_core::policy::DecisionRecord) -> RuntimeResult<()>; implemented in sibling, do not implement duplicate here. Sibling trait port takes fullDecisionRecord and shard-owned EventSeq and returns only after SyncAll.
Runtime calls self.shard_for_mut(ticket.run)?.append_policy_decision_strict(decision.clone())? BEFORE adapter.execute.

## State and context
ADD actor-owned LivePolicy to Runtime with Unconfigured initial state in every constructor, retain policy: RuntimePolicy. Trusted configure_live_policy(&mut self, PolicyDigest, PolicyRule) updates it. Activation API returns RuntimeResult and persists actual store marker before reporting success; void instance-only activation is insufficient.
Shard-built RegisteredActionContext adds actual admission policy pin, canonical SlotValue::I64 serialized bytes already computed in builder, and actual frame input taint. Convert RunPolicyPin::Pinned WorkflowDigest bytes to PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes(...)); Unverified stays Unverified. No artifact/current/zero substitution. Retain ownership, preflight, grant/catalog and Live mode checks.
Read latest durable emergency_deny_state via actual RuntimeJournal::storage_journal on EVERY new execute. Missing store, corruption or read error fails closed. Owned evaluator may mirror a read Active state monotonically; cache does not replace current store read.

## Decision and effect ordering
Actual LivePolicy::evaluate takes &self and returns RuntimeResult<DecisionRecord>; use actual bytes/taint/fullticket. It currently does not reject Unverified, so a small Runtime dispatch-decision helper must force Denied at Admission layer for Unverified while retaining true emergency priority/influence. Emergency Active stays emergency denial, not overwritten as an ordinary admission denial. Unconfigured/current Denied/Suspended are never Allowed.
Persist full final decision through sibling Shard strict port for allow and deny. If append fails, no adapter.execute and pending unchanged. Only Allowed after successful SyncAll reaches existing adapter.execute(grant,ticket,context.value); then retain existing completion enqueue/receipt semantics. Add typed RegisteredExecutionError policy-rejection variant carrying decision; keep journal/runtime/executor failures distinct. Avoid expanding global RuntimeError registry when the local execution error suffices.
Reconcile remains actual existing receipt read with catalog.require_receipt_recovery and grant validation, not newly denied by current policy/emergency. Never call execute from replay/reconcile. Scheduler/replay source remains untouched.

## Source ownership
Runtime.rs minimal new field/constructor/module wiring; new runtime/live_policy_dispatch.rs for focused policy/config helpers; runtime/registered_executor.rs for actual pre-effect call and local error variant; shard/lifecycle/registered_completion.rs for context and builder. Do not edit journal.rs/chunks or shard impl_.rs; sibling owns those. No core policy rewrite or config/gate changes.

## Behavioral checks next
After-scheduling policy changes; real allow decision durable before effect; deny/unverified/unconfigured/suspend/append error zero effects and exact pending equality; canonical bytes+actualtaint; durable emergency after actual close/reopen; corrupt marker fail closed; existing receipt reconciles under new deny without fresh execute. No physical disk-failure claim from simulated port fault. Grant expiry/revocation remain separate tracked child, not waived or declared complete.
