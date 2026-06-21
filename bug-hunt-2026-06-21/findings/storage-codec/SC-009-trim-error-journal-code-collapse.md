# SC-009: `TrimError::diagnostic_code` collapses `Journal` errors into the Fjall diagnostic code

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_storage/src/trimming/mod.rs:65-73`
- **Confidence**: confirmed

## Description

The `diagnostic_code` matcher returns `JournalError::FJALL_CODE` for both `Self::Fjall(_)` and `Self::Journal(_)`. Every distinct `JournalError` variant (digest mismatch, sequence gap, bad magic, payload too large, etc.) raised through the trim path gets the same diagnostic code as a raw Fjall IO error, defeating the purpose of having typed diagnostic codes.

## Evidence

```rust
// crates/vb_storage/src/trimming/mod.rs:65-73
pub const fn diagnostic_code(&self) -> vb_core::DiagnosticCode {
    match self {
        Self::Fjall(_) => JournalError::FJALL_CODE,
        Self::Journal(_) => JournalError::FJALL_CODE,           // <-- same code
        Self::NoDurableSnapshot { .. } => Self::NO_DURABLE_SNAPSHOT_CODE,
        Self::RetentionPolicyBlocks { .. } => Self::RETENTION_POLICY_BLOCKS_CODE,
        Self::IncompleteTrim { .. } => Self::INCOMPLETE_TRIM_CODE,
    }
}
```

`JournalError` itself exposes `FJALL_CODE` only for its own `Fjall(_)` variant; journal-level errors like `PayloadDigestMismatch`, `HeaderChecksumMismatch`, `SequenceGap`, etc. carry their own codes when reported directly. Wrapping them inside `TrimError::Journal(_)` strips that information and rewrites it to the Fjall code.

## Adversarial Check

The trim module's documented contract (`crates/vb_storage/src/trimming/mod.rs:27-55`) explicitly enumerates `Journal(#[from] JournalError)` as a distinct error category. The matcher's failure to delegate to the inner `JournalError::diagnostic_code()` (assuming such a method exists on `JournalError`) is therefore an oversight, not an intentional collapse. Operators rely on diagnostic codes to route alerts (e.g., page-on-digest-mismatch vs. log-on-Fjall-IO), and this collapse makes the trim path opaque.

## Suggested Fix

Delegate to the inner error's code:

```rust
Self::Fjall(_) => JournalError::FJALL_CODE,
Self::Journal(e) => e.diagnostic_code(),
```
