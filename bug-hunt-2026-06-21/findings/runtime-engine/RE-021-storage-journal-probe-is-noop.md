# RE-021: Storage-backed journal health probes always report success

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/journal/chunk_002.rs:353-355`, `crates/vb_runtime/src/journal/chunk_003.rs:18-20`
- **Confidence**: likely

## Description

The `RuntimeJournal` trait defines `probe` as a health check for journal availability, but both storage-backed implementations return `Ok(())` unconditionally. A caller can receive a healthy probe result even when the underlying Fjall journal or queued writer path is unavailable.

## Evidence

The trait contract at `crates/vb_runtime/src/journal/chunk_001.rs:283-285`:

```rust
/// Probes journal health without side effects.
/// Returns `JournalPoisoned` if the underlying storage is unavailable.
fn probe(&self) -> RuntimeResult<()>;
```

Direct storage implementation at `crates/vb_runtime/src/journal/chunk_002.rs:353-355`:

```rust
fn probe(&self) -> RuntimeResult<()> {
    Ok(())
}
```

Queued storage implementation at `crates/vb_runtime/src/journal/chunk_003.rs:18-20`:

```rust
fn probe(&self) -> RuntimeResult<()> {
    Ok(())
}
```

Neither implementation touches `self.journal`, `self.queue`, or any storage health indicator.

## Adversarial Check

Noop and volatile journals can reasonably probe only their local state, but these are storage-backed adapters. They expose `storage_journal()` because callers depend on the storage backing, and the trait documentation specifically mentions unavailable storage. Returning success without checking anything makes `probe` unsuitable for admission readiness, shutdown diagnostics, or fail-fast durability checks.

## Suggested Fix

Delegate to a read-only Fjall journal health check if one exists, or add one to the storage facade. For the queued adapter, also check that the writer queue can accept or flush work according to its durability profile. If no meaningful non-side-effecting probe exists, rename the method or remove the storage-backed implementation's health guarantee.
