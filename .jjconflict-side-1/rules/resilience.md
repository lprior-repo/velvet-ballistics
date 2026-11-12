# Cross-service resilience

Read this when a service calls other services — HTTP/RPC clients, retries, timeouts, cross-service transactions, or deciding failure behavior under a slow or dead dependency.

- A timeout alone is not resilience. Every synchronous cross-service call gets an explicit timeout, a circuit breaker, and a named fallback — cached value, default, degraded feature, or a clean fast failure surfaced to the caller.
- Retry only transient failures (timeouts, 5xx, connection resets) with jittered backoff and a capped total budget — never 4xx, never unbounded (`rules/backend-security.md` for the outbound retry contract). The circuit breaker is what stops retrying a persistently dead dependency.
- Retrying a non-idempotent call requires an idempotency key; without one a retry is a duplicate side-effect, not resilience.
- A timeout is a budget, not a per-hop constant: the caller's remaining deadline travels with the request (propagated header or context), and each hop uses the smaller of its own limit and what's left — three sequential 5s calls under a 5s edge timeout keep working on requests the client already abandoned.
- A distributed lock or lease expires while its holder still runs (GC pause, network stall): correctness requires a fencing token — a monotonically increasing number the protected resource itself checks — otherwise the lock is advisory. A TTL-based lock without fencing (`SETNX`/Redlock-style) protects efficiency, not correctness.
- Shed load at the edge under aggregate overload: reject early (429/503 with `Retry-After`) while the process still has headroom, keeping health checks and critical traffic ahead of the rest — a service that accepts everything degrades everything (`rules/rate-limiting.md` for per-client limits).
- Cross-service consistency is a saga: local transactions plus compensating actions. Never a distributed transaction/2PC across service boundaries, where a coordinator failure leaves every participant holding its locks.
- Multi-step workflows run in a state machine/orchestrator with explicit per-step compensation — not chained calls inside one request handler, where step 3's failure leaves steps 1–2 committed and invisible.
- Database-per-service is a boundary, not a suggestion: never query or report from another service's tables — a direct read couples you to a schema you don't control. Cross-service reads go through its API, a CQRS read model, or consumed events.
- Put a queue between a bursty producer and a rate-limited consumer (load leveling) before scaling either side; scale consumers on queue depth.
- Autoscale on the signal that actually saturates: queue depth or consumer lag for workers, in-flight requests or a latency percentile for services, CPU only when CPU is the real ceiling.
- Every queue is bounded with a declared overflow policy — backpressure (pull), drop, or throttle. An unbounded in-memory queue is a deferred crash. Broker-side delivery, DLQ, and idempotent-consumer rules: `rules/messaging.md`.
- Bulkhead: isolate connection pools and concurrency limits per downstream dependency, so one slow dependency saturates its own pool instead of the shared one.
- A shared internal library that couples services (shared domain models, shared DB-access helpers) version-locks its consumers into deploying together — share contracts and truly stable primitives, duplicate the rest.
- Correlation id on every request and message end to end, so one id reconstructs a request's path across services (`rules/observability.md`).
- Never order cross-node events by wall-clock timestamps, which drift between nodes and misorder events; use sequence numbers or per-key serial processing (`rules/messaging.md`).
