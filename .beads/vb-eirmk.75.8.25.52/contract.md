# Bounded implementation contract: incident action evidence

## Authorization and scope

Implement only the exact approved proposal at `/home/lewis/src/isoloated/r17/.evidence/reconciliation/incident52-proposal/proposal.md` (SHA-256 `4ab49cd36c97b385bed247ab6ce3e4a7d0aa2c570086d6bb6428480967ff36f3`) and its independent bounded review. Approval covers the documented public `SideEffect` fields and `SideEffectCertainty::{Scheduled, Abandoned}` variants, corresponding CLI report fields and ordered `journal_tail`, and caller/test updates. It does not authorize a new persisted format, journal/ticket encoding, dependency, command, version, gate, unrelated report fields, action-policy lookup, safe-retry policy, or execution/recovery behavior.

## Facts and identity

Project each retained event for the requested run into the report in its existing event order. Action facts comprise actual scheduled, completed, failed, and abandoned journal events. Preserve repeated attempts as separate rows. Use the event's actual run, sequence and attempt. For `ActionScheduledTicket`, `ActionCompletedEnvelope`, and `ActionAbandoned`, retain the actual full `ActionTicket` and ABI digest when present. Never synthesize absent identity. Encode the `u128` idempotency key as a decimal string. Sparse historical action events retain their real legacy identity fields and missing ticket/digest as null.

Add `scheduled_durable` and `completion_durable` as nullable booleans. A phase is true only when its durable event is evidenced in this retained stream for the same complete ticket identity. Do not correlate distinct tickets by step, action, attempt, idempotency key alone, or by event proximity. For sparse ticketless history, report its own recorded phase as true and leave the opposite phase null. Run, step, action, and attempt fields do not prove that two historical records describe one invocation, even when they form a unique count or appear in sequence order. Missing/truncated history is unknown, not false. Abandonment is cancellation of an invocation, never completion and never proof of absent external effects.

For every action row, `external_effect` is `uncertain`, `safe_to_retry` is null, and `reason` is `retry_contract_not_loaded`. A ticket/key alone is insufficient to assert policy, taint, executor safety, or permission to retry. Preserve repair hints while ensuring they distinguish recorded completion, failure, and abandonment and do not claim compensation or safe retry.

## Ordered tail and output

Add public `IncidentReport.journal_tail`, containing one entry per actual retained `JournalEvent`, ordered as supplied, with only `seq`, `kind`, `run_id`, and nullable `attempt`. Obtain kind from the existing event-to-record-kind mapping; attempt from the existing event accessor. Do not add a synthetic checkpoint or include slot values, inputs, results, source payloads, or secret material.

YAML and Postcard JSON preserve the current CLI envelope conventions. Text emits the same incident facts and ordered tail. Existing run ID, failure code, failed step, command, error handling, success/failure exit codes, and persisted bytes remain unchanged.

## Acceptance evidence

Add/revise tests from the approved contract. Cover actual scheduled-only, same-full-ticket completion, action failure, abandonment, pending, sparse historical completion, multiple runs/run attempts/action attempts/fences/keys, ordered unrelated tail events, missing identity, exact retry uncertainty, and redaction sentinel exclusion. Assert complete relevant public report values and exact retained persisted bytes for public CLI fixtures sourced through real current event issuer/runtime paths. Construction-only unit fixtures must not be described as public runtime qualification. All three existing output formats must expose the same facts. Preserve meaningful old tests; replace only assertions that call abandonment confirmed with exact abandoned/uncertain assertions.

No new production function may exceed 25 raw lines or five parameters; no changed file may exceed 300 lines. Preserve functional style and the repository prohibitions on unsafe, panic paths, unchecked access/arithmetic, ignored errors, and production imperative loops. Run scoped owning tests, strict source gates and formatter; retain raw commands, logs and exit codes under this isolated workspace. No benchmarks.
