# RA-014: `lock_admission` permanently bricks shard admission on mutex poison

- **Severity**: Low
- **Category**: correctness (availability)
- **Location**: `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:118-132`
- **Confidence**: confirmed

## Description

`lock_admission` recovers the poison guard via `drop(poisoned.into_inner())` and returns `Err(RuntimeError::JournalPoisoned)`. The drop releases the guard, so subsequent callers re-hit `Mutex::lock` on a still-poisoned mutex — which keeps returning `Err(PoisonError)` forever. One panic thus permanently disables the admission gate for that shard.

## Evidence

```rust
pub(crate) fn lock_admission(
    &self,
) -> Result<std::sync::MutexGuard<'_, ()>, crate::RuntimeError> {
    match self.admission_lock.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            drop(poisoned.into_inner());
            Err(crate::RuntimeError::JournalPoisoned)
        }
    }
}
```

`std::sync::Mutex::lock` on a poisoned mutex always returns `Err(PoisonError)`. Once poisoned, the mutex never un-poisons. The `drop(...)` here does *not* clear the poison flag — it just releases the recovered guard. Every subsequent `submit_direct`, `submit_compiled_with_inputs`, `submit_direct_with_inputs_grants_and_contracts`, and `submit_artifact` call on this shard will return `Err(JournalPoisoned)`.

The docstring at `config.rs:258-266` and the function-level docstring at `chunk_001.rs:107-117` both describe the lock as the load-bearing atomicity guarantee for the entire submit path. Permanently disabling it permanently disables all submits to that shard.

## Adversarial Check

The function-level comment says "Production code never panics, so this is a defense-in-depth typed error path." But the AGENTS.md engineering rules forbid `panic!`, `unwrap`, `expect`, etc. in production, AND the same rules forbid `unsafe`. The codebase has verifiable no-panic properties only on paper — any panic in `try_add_budget`, `check_capability`, or any of the called functions across the admission lock-holding window (which includes `clone()` on `CapabilitySet` and `postcard::take_from_bytes`) would brick the shard permanently. Even with no-panic guarantees, a panic via `Vec::push` allocation failure (which `push` propagates as abort on most platforms, but `try_reserve` callers in the same module suggest allocation failure is considered recoverable elsewhere) would brick the shard. A defense-in-depth path that converts a transient failure into a permanent outage is itself a bug.

## Suggested Fix

Either (a) on poison, *keep* the recovered guard and proceed (typical Rust recovery pattern) — return the guard wrapped in a "recovered from poison" log and let the submit continue; or (b) surface the error to the caller with a `RuntimeError::AdmissionPoisoned` variant and provide a `Runtime::reset_admission_poison(&mut self, shard: u32)` escape hatch that explicitly re-initializes the mutex. Option (a) is the standard Rust pattern and matches what the rest of the codebase does for journal poisoning (`JournalPoisoned` is treated as terminal elsewhere, but the runtime keeps running for already-queued commands).
