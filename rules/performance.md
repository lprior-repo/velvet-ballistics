# Performance fundamentals

Read this when writing or reviewing performance-sensitive code, or when asked to make something faster or to explain rising latency — any language, any stack. Domain-specific hot paths live in their own modules: queries and indexes in `rules/database.md`, caches in `rules/caching.md`, animation in `rules/frontend-design.md`.

- Measure first: profile the real workload and name the hot spot before changing code. An optimization without a before/after number is a style change, and the bottleneck is routinely not where it was guessed to be — speed claims pass the same evidence gate as correctness claims (core Verification rule).
- An optimization's proof has two halves: the before/after number and identical output on the same input (row count plus a checksum, or a diff of the result). A rewritten query or reordered pipeline that got faster by returning less passes the timing and ships the bug.
- Before and after run on the same lockfile, toolchain, input and machine: a side whose dependencies were re-resolved or that ran on another host measures upstream drift and noise, not the change. Copy the baseline's lockfile into the second checkout and confirm the two match before comparing.
- Fix the complexity class before the constants: a quadratic pair of nested loops over user-sized data outgrows any micro-tuning of its body. Replace the inner scan with a hash index built once before the loop — that is the two-line change that turns O(n²) into O(n).
- Pick the data structure by the dominant operation: membership test → hash set, key lookup → map, FIFO → queue/deque, priority → heap, range or ordered scan → sorted structure. A linear `contains` inside a loop is the standard accidental O(n²).
- Library calls carry hidden complexity: string concatenation in a loop is quadratic where strings are immutable — collect parts and join once; a sort, deep copy, or full scan inside a loop multiplies its cost by N. Account for the big-O of what the call does, not only of the lines you wrote.
- Batch per-item round-trips — network, disk, database, IPC: latency dominates compute, so one call carrying N items beats N calls carrying one. The N+1 query is one instance of this class (`rules/database.md`); a per-item HTTP call in a loop is another.
- Hoist invariant work out of loops: a compiled regex, a parsed schema, an opened connection, a constructed formatter — built once before the loop, not once per iteration.
- Stream instead of materializing: process large inputs incrementally so peak memory is bounded by a chunk, not by the input size.
- Every buffer, cache, and queue gets an explicit bound — unbounded growth is a deferred crash (`rules/resilience.md`).
- Do less work before doing work faster: early-exit on the common case, compute lazily what may go unread, dedupe repeated identical calls, cache pure results (`rules/caching.md` owns invalidation).
- Prefer contiguous, in-order data access in hot paths: sequential iteration is cheap because of memory locality; pointer-chasing and per-item allocation defeat it. In garbage-collected languages, allocation churn inside a hot loop is GC pressure — reuse and preallocate there, and only there.
- Concurrency answers measured saturation, never a hunch: I/O-bound work wants batching or async multiplexing; CPU-bound work wants parallel workers sized to the cores actually free (`rules/shared-machine.md`). Parallelizing an unmeasured path buys coordination cost and race surface.
- Where speed is a requirement, set a budget — a latency target, a memory cap, the input size the code must handle — and leave the check runnable: a timed test on a fixed input, or the repo's benchmark harness. "Fast" without a number regresses silently.
- Optimizations expire: a workaround for a runtime or hardware bottleneck carries a comment naming the condition for removing it (core simplification-marker rule) — the next engine version often turns it into dead weight or a pessimization.
- Latency rising while CPU, memory and scheduler delay all look healthy points at a saturated bounded pool: a worker or thread pool, a connection pool, a semaphore. Check the queue depth of every bounded pool on the call path before suspecting the code.

Not a violation — leave these alone:

- Work that is slow by design: a password hashing cost factor, a rate limiter's delay, a retry backoff, a constant-time comparison. Speeding these up removes the property they exist to provide (`rules/crypto.md`, `rules/rate-limiting.md`).
- A quadratic loop over a bounded collection whose size the code controls — a config file's keys, a fixed enum, a handful of CLI flags. Complexity class decides where N grows; naming the bound beats rewriting the loop.
- Readable code on a cold path: startup, migrations, build scripts, an admin screen opened twice a month. Optimizing there spends clarity on time nobody waits for.
- A path with no measurement behind the suspicion. "This looks slow" is a reason to profile, never a reason to rewrite; an unprofiled optimization is a style change that also carries new bugs.
- Allocation and copying outside a hot loop. Reuse and preallocation are hot-path techniques, and applying them everywhere trades readability for nothing measurable.
- A cache deliberately absent because invalidation would cost more than the recomputation. Recomputing a cheap pure value is a decision (`rules/caching.md`).
