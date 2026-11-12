# Rate limiting and quotas

Read this when designing or reviewing a rate limiter, quota, or capacity control — the algorithm, where the counter lives, and what happens when the limiter itself fails.

- Pick the algorithm by traffic shape: token bucket permits controlled bursts (public APIs); leaky bucket smooths to a constant processing rate; fixed window is simplest but admits 2× the limit at window boundaries; sliding-window counter fixes the boundary at near-fixed-window cost — the usual default.
- The counter lives where the instances are: an in-process counter multiplies the effective limit by the replica count. A fleet needs a shared store with atomic increment-and-expire; per-instance counting is valid only when the limit itself is per-instance (protecting one process's event loop).
- Decide fail-open vs fail-closed per endpoint class before the limiter's store first goes down: auth, checkout, and abuse-prone endpoints fail closed; general read traffic fails open. The library's implicit default is the wrong choice for at least one of those classes.
- Key limits by what they protect against: per-credential for quota, per-IP for unauthenticated abuse, per-tenant for fairness. One global limit lets a single tenant starve the rest; per-IP alone blocks a NAT'd office and waves through a botnet.
- Return the contract, not just the refusal: 429 with `Retry-After` and quota headers so clients self-throttle (`rules/public-api-design.md`); log limit hits with their key so abuse is distinguishable from a broken client.
- Rate limiting is not load shedding: limits enforce per-client policy; shedding protects the whole process under aggregate overload (`rules/resilience.md`). A thousand clients each under their limit still melt an underscaled backend — you need both.
- When limits keep tripping on legitimate load, fix capacity in order — measure, then index/cache/replica before sharding (`rules/database.md`, `rules/caching.md`); raising the limit without capacity work moves the failure downstream.
