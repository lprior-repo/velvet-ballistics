# Coordinator source audit of local3090 map

Scope: advisory source map only at unaccepted prototype5fff342a. No behavior/release/policy acceptance.

Verified directly against actual files: wire_category.rs does not route PolicyDecision; wire_tags.rs omits the tag constant and variant name; custom JournalEvent codec routes by explicit RecordKind categories. Existing shape therefore cannot use actual durable append/read successfully. Child vb-s4hfc owns codec repair; parent vb-ny068 remains open.

Corrections to model report: ActionTicket has eight fields, including run_attempt, idempotency_key and capacity, not merely run/step/action/attempt. Existing PolicyDecisionEvent also lacks pinned admission identity. A recovery match arm that skips the event is not restoration of a complete decision identity. No-op summary/hydration means source exhaustiveness only, not correct durable replay semantics. Proposed precise Fjall APIs/test surfaces in the model report are unverified until actual scoped tests compile and execute. Explicit tag-based codec can preserve all older tags; the report closing sentence about failure to preserve old postcard numbering is unsupported and superseded by this note.

The worker ran source inspection only. No tests were claimed or run in this scout. Original report preserved unchanged for traceability.
