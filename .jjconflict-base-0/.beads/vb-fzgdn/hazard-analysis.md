# Hazard Analysis: vb-fzgdn

## Temporal/replay hazards
- `Instant::now()` during registration causes replay divergence; replace with explicit numeric tick.
- Deadline addition can overflow; checked add before mutation.
- Equal deadlines can fire nondeterministically; stable ordering or journaled sequence.
- Clock regression can reorder/resurrect timers; monotonic guard.
- Zero delay ambiguity; explicit policy.

## Rust invariant hazards
- Partial authority comparison permits stale mutation; structural equality over all authority fields.
- Generation wrap validates old authorities; checked non-wrapping successor.
- Mutation before validation loses pending timers; validate-before-remove.
- Duplicate keys can duplicate side effects; classify identical/conflict before mutation.
- Signed/float slot values can be cast unsafely; constructors reject invalid encodings.

## Bounded-resource hazards
- Registry/key index can grow unbounded; explicit capacity and reservation.
- Fire can remove timer then fail downstream enqueue; check/reserve capacity before irreversible mutation or define atomic typed outcome.
- Journal batch can overflow; fixed-width payload and capacity check.

## Concurrency hazards
- External driver races cancellation/replacement; shard-owned serialized processing plus generation.
- Multiple clock advances out of order; reject backward commands.
- Duplicate fire commands; terminal duplicate outcome without resurrection.

## Hostile input hazards
- Max `u64` delay can overflow; checked constructor/add.
- Missing/non-finite/wrong-type slots; parser errors before mutation.
- Divergent duplicate keys can perturb original deadline; preserve identical original, reject divergent.

## Migration/API hazards
- Existing `TimerWheel`/`TimerEntry` use `Instant`; migrate or fence legacy wheel from deterministic path.
- Existing `Runtime::timer_fired(run)` fail-closes; do not reopen run-only mutation.
- Compatibility error mapping can hide proof distinctions; keep internal typed variants.

## Proof seed focus
Prioritize Rust-local obligations for constructors, checked arithmetic, authority equality, duplicate classification, capacity guards, and validation-before-mutation. TLA may be design smoke only unless bridged to Rust behavior.
