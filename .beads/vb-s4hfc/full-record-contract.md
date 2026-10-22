# Bead vb-s4hfc: Full Typed DecisionRecord Persistence

## Scope

Integrate full typed `DecisionRecord` persistence into the storage codec.
New `JournalEvent::PolicyDecisionEvent` variant carries the complete
`vb_core::policy::DecisionRecord` including all 8 ticket fields, admission
identity, current live identity, canonical input digest, taint, outcome,
layer, and influence factors.

## Schema

New stable wire tag: RecordKind 38 (PolicyDecision)

PolicyDecisionEvent fields:
- run: RunId
- seq: EventSeq
- decision: DecisionRecord (full core policy decision)

Ticket.seq is SeqNo; journal event.seq is EventSeq. No conflation.

## Acceptance

- All DecisionRecord fields survive Fjall strict append, close/reopen, events_for_run
- Round-trip equality for complete record including all enum variants
- Stable tag 38 assigned and used; old tags preserved
- Malformed/truncated events rejected
- No journal-seq/ticket-seq conflation
- Run envelope mismatch protected
- All tests compile and pass
- Strict clippy clean on vb_storage and vb_runtime
