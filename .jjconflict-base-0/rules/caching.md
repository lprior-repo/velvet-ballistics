# Caching

Read this when adding or reviewing a cache (in-process, a shared cache server, or a cached computation), or chasing data that stays stale after a change. The rules hold for any cache. HTTP/CDN response caching lives in `rules/public-api-design.md`.

- Cache-aside is the default: read → miss → load from the source of truth → set with TTL. The write path invalidates the key rather than updating the cached value in place — two writers updating a value race; a delete is idempotent.
- Choose the write strategy by read-follows-write distance, before writing code: cache-aside covers most cases; write-through when a read follows its write immediately and staleness is unacceptable; write-behind only for loss-tolerant counters and metrics — it acknowledges before the store write, so a crash loses data.
- Invalidation is designed before the cache is added, not after the first stale bug: every entry gets a TTL as the guardrail, and the domain event that makes the value wrong is named and invalidated on (`rules/backend-security.md`). A cache "invalidated" only by expiry serves known-stale data for the whole window.
- List every cache between the user and the source of truth before declaring one cleared: a CDN, a framework's own route or data cache, and the application cache are separate layers with separate invalidation. Wire each into the same domain event, or a stale-data fix clears one layer and the bug returns from the next.
- Set the eviction policy and memory ceiling explicitly — the unconfigured default either grows until OOM or silently evicts the wrong class of keys.
- Cache keys encode their scope — tenant, user, locale, schema version; permission- or billing-sensitive data never sits under a shared key (`rules/backend-security.md`) — a mis-scoped key is a cross-tenant data leak, not a staleness bug.
- The cache is disposable: anything that can't be rebuilt from the source of truth doesn't live only in the cache. An ephemeral cache server as the primary store for real state is an outage on a timer.
- Stampede protection on expensive keys: single-flight/lock so one expiry triggers one rebuild, or jittered TTLs so a cohort of keys doesn't expire in the same second.
- A hot key melts one shard while the cluster idles: shard the key (bucket suffix, aggregated on read) or put a short-TTL in-process cache in front of it.
- Application code never enumerates the keyspace: a match over every key is linear in the whole store and blocks a single-threaded server. Operations tooling uses the store's incremental scan, application lookups a maintained index structure.
- Model the value for its access pattern: a structure with independently addressable fields when fields are read or written independently, a serialized blob only when the value is always read whole.
- Pipeline or batch serial round-trips; a per-item network round-trip in a loop is the cache's own N+1.
- An in-process cache is per-instance state: N instances hold N divergent copies and a deploy wipes them all. Anything needing cross-instance coherence goes to the shared cache; in-process caches stay small, bounded (LRU with a max size), and safe to lose (`rules/backend-security.md` stateless-process rule).
- Expose hit/miss counters per cache (`rules/observability.md`): an unmeasured cache can't justify its complexity, and a 5% hit rate is a bug that looks like a feature.
- Break hit/miss down per key family: a healthy aggregate hides one family near zero. Track the age of what is served and the lag from a domain event to the cache reflecting it as signals of their own, since hit counts show neither.
