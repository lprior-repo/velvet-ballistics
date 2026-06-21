# RS-214-core-introspection-epoch-saturation: Saturating epochs can let stale inspect handles unregister current handles

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/introspection.rs:110`
- **Confidence**: confirmed

## Description
`IntrospectionRegistry` uses `saturating_add(1)` for epochs. After `u64::MAX`, every new registration reuses the same epoch, defeating the stale-drop protection in `InspectHandle::drop`.

## Evidence
```rust
68:         if let Ok(mut guard) = self.registry.lock() {
69:             // Only remove if the epoch matches (handles stale drops correctly)
70:             if let Some(current_epoch) = guard.get(&self.run)
71:                 && *current_epoch == self.epoch
72:             {
73:                 guard.remove(&self.run);
74:             }
...
110:         let epoch = self.next_epoch;
111:         self.next_epoch = self.next_epoch.saturating_add(1);
112:         guard.insert(run, epoch);
...
135:             let new_epoch = self.next_epoch;
136:             self.next_epoch = self.next_epoch.saturating_add(1);
137:             guard.insert(run, new_epoch);
```

At saturation, `new_epoch == old_epoch == u64::MAX`. In the overlap path, an old handle and its replacement can share the same epoch. Dropping the old handle then passes the equality check and removes the current registration.

## Adversarial Check
The code intentionally uses epochs to prevent stale drops, so epoch uniqueness is part of the contract. Saturating arithmetic avoids numeric overflow but replaces it with identity reuse. The edge is rare, but long-lived deterministic runtimes should fail explicitly rather than silently break RAII ownership.

## Suggested Fix
Use `checked_add(1)` and return a typed runtime error when the epoch space is exhausted. If wraparound is desired, only allow it when the registry is empty and no stale handles can exist.
