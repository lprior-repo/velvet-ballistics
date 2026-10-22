# Binding coordinator corrections to local advisory drafts

The v1/v2 advisory documents are historical design input, not accepted implementation contracts. Their unverified symbol claims require source confirmation. The following constraints govern the next implementation:

- `RuntimePolicy` is durability/admission only. `Unconfigured` live authority denies; no missing-admission Relaxed fallback, zero digest, workflow-digest substitution or default AllowAll.
- The emergency deny overlay is monotonic. There is no Clear/unset operation in this scope, including explicit operator reset, policy replacement, new admission or reopen. The v2 Clear recommendation is rejected. Persistent current authority/recovery must not silently loosen a recorded deny.
- Historical replay restores recorded decisions and semantic history without current-policy evaluation or external effects. The v2 scenario C step that evaluates current deny *during historical replay* is rejected. A separate new live dispatch/retry evaluates current authority after reconstruction.
- Pin the actual accepted-artifact policy digest per admission. Explicitly represent missing/unverified admission; it cannot be promoted to trusted artifact authority by a default.
- A decision input digest binds actual input value and taint plus full ticket identity. Identity-only hashing is insufficient. Do not put unbounded payloads into a bounded journal record just to avoid computing a digest.
- Actual compiled methods are in `shard/lifecycle/chunk_005.rs`, included by `shard/lifecycle.rs`. The similarly named chunk_002_parts copy is orphaned. Follow include graph for all claimed production wiring.
- Every action boundary, including evidence-empty/Relaxed paths, requires a decision before pending-ticket visibility. Permit decision and schedule append atomically. Denied/suspended decisions append before a typed, recoverable state transition; an Err that discards a local batch does not satisfy durability.
- Current-live configuration must be explicitly supplied through a real public runtime/shard seam. Old accessors/recovery/retry paths cannot mint new authority from old admitted or allowed history.
- Behavioral acceptance requires the actual public runtime/shard and durable journal path: deny before dispatch/no pending ticket, monotonic deny, retry recheck, unchanged historical replay under new deny/zero effects, crash/reopen exact digests/identity. Unit tests of a recorder alone do not close these claims.

No production change or formal closure is asserted by these notes. Parent vb-ny068 remains in progress.
