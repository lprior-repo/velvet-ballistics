# Executor and receipt workflow

Raw pack -> parsed manifest -> resolved registered executor -> executed
conformance resolved -> CheckedActionPack -> accepted artifact binding. Each
transition returns a semantic error and produces no checked value on failure.
Legacy APIs with no executor context continue rejecting Strict side effects.

Live adapter flow: validate accepted pack and current resource grant -> verify
canonical scheduled ticket exists durably -> bounded receipt lookup. Existing
matching receipt returns its prior result with no mutation. A conflicting
receipt returns divergence. An absent receipt permits exactly one atomic batch
for target update plus receipt, followed by SyncAll before success acknowledgement.

Crash before batch commit leaves neither effect nor receipt. Crash after durable
commit but before completion acknowledgement leaves both, and replay performs
receipt lookup only. A completed matching receipt reconciles the result; mismatch
is divergence. Missing receipt under unsupported recovery is Blocked. Never
re-execute an unknown non-idempotent scheduled effect during replay.

Conformance execution uses real issued tickets, the actual adapter implementation,
a temporary bounded store, same-ticket replay, conflicting input rejection and
close/reopen reconciliation. The report identifies exact adapter version,
receipt schema, scenario observations and bounds. Recorded observations do not
claim universal external idempotency or formal proof.
