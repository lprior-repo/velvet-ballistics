# vb-z7ky4 Rust contract
Parent vb-ny068. Master77.3/77.11 governs. This is the strict durable decision port and checked shard sequencing prerequisite; no adapter effects, current-policy state, scheduler/replay changes or fullv1 acceptance.

## Proposed exact shared interface
Add RuntimeJournal::append_policy_decision_strict(&self, decision: vb_core::policy::DecisionRecord, seq: vb_storage::EventSeq) -> RuntimeResult<()> with default UnsupportedOperation, never default success.
Add Shard::append_policy_decision_strict(&mut self, decision: vb_core::policy::DecisionRecord) -> RuntimeResult<()>.
Sibling vb-l5ru0 calls the Shard method before adapter.execute.

## Invariants
DecisionRecord.ticket.run supplies outer event run; full9fields preserved as JournalEvent::PolicyDecisionEvent. StorageRuntimeJournal uses actual FjallJournal::append_strict regardless of normal Journaled/Strict profile. Return success only after SyncAll.
QueuedStorageRuntimeJournal must preserve earlier queued lifecycle prefix and actual synchronous acknowledgement without closing the queue. A supported route is probe accepting writes, bounded drain_all to commit the prefix, then direct append_strict on the same store; that final SyncAll also covers preceding WAL writes. Do not claim enqueue or one arbitrary flush acknowledges this decision. Do not use shutdown/drain_for_shutdown. Errors propagate.
Shard owns sequence. Check exact pending-ticket ownership, checked next sequence and map capacity BEFORE append. Call strict trait port with current seq. Publish next seq only on success; append error leaves pending state and next seq unchanged. No unchecked arithmetic, no synthetic default digest.
No new Kani success stub, shadow proof or harness weakening. Keep real behavior in normal builds and report any cfg issue.

## Existing sources
RuntimeJournal trait journal/chunk_001.rs; Storage implementation journal/chunk_002.rs; Queued journal/chunk_003.rs. Actual storage_journal and convert_to_storage_event already exist.
Shard impl_.rs includes impl_parts chunks; new included sibling can access existing private sequence/map helpers in same module. Existing batch helper prechecks next/reserve before append. Field journal exists in all builds.
Fjall append_strict in storage journal/append.rs; queue drain_all in queue/writer.rs; flush_batch_durable in queue/writer/flush.rs.

## Source ownership
Only journal.rs, journal/chunk_001.rs, journal/chunk_002.rs, journal/chunk_003.rs, new focused journal/policy_decision.rs, shard/impl_.rs and new shard/impl_parts/policy_decision.rs. Logic in new focused files, existing files get minimal wiring. Do not edit Runtime or registered context; sibling owns those.

## Behavior checks required next
Real storage fullDecisionRecord/seq roundtrip and durable reopen under ordinary Journaled profile; queued prefix before fulldecision and queue remains accepting; default unsupported adapter fails closed; injected typed strict-port error leaves shard sequence/pending unchanged; overflow/capacity rejected before append. Fault injection is a port failure, not physical disk/power loss.
