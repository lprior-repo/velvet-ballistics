# Hazards and boundaries

Temporal: effect succeeds but acknowledgement is lost. Atomic target+receipt and
reopen reconciliation are required for the proposed adapter. This design cannot
transfer its atomicity to a remote provider.

Authority: a hostile pack/report can invent identities and passing observations.
A trusted registered executor plus independently resolved executed report is
required; digest equality alone is insufficient. Current Strict rejection remains
until this authority path exists.

Bounded state: in-memory eviction is not durable deduplication. Exhausted receipt
capacity must reject new effects, preserve existing receipts, and return typed
capacity errors. No silent overflow/truncation of ticket IDs or payload lengths.

Refinement: recovery and idempotency are independent. Blocked cannot be replaced
by RetrySameKey because a key happens to exist; Compensate needs a real inverse
operation with separate evidence.

Concurrency: either the minimal adapter has one owner or a transaction/lock guards
lookup+write across callers. Concurrent same-key submissions must not each mutate
the target. No await or async machinery is added.

Hostile input/release: positional postcard payloads cannot be extended by assuming
serde default works at EOF. Use explicit pack/evidence schema and old-wire fixtures;
unknown or incompatible schemas reject before being treated as authority.

API/performance: add an accepted pack boundary instead of expanding every existing
ActionContract literal. Cold pack resolution may allocate; runtime action bindings
remain numeric and bounded. No speed claim or optimization is in scope.
