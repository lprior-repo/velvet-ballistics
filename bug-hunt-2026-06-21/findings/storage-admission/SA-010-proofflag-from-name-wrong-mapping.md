# SA-010: `ProofFlag::from_flag_name` returns wrong variant for `"idempotency_verified"` and silently defaults unknown names to `Bounded`

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_storage/src/admission/types.rs:69-79`
- **Confidence**: confirmed

## Description

`ProofFlag::from_flag_name("idempotency_verified")` returns `ProofFlag::RetrySafe` (no `IdempotencyVerified` variant exists on the enum), and the catch-all wildcard maps any unrecognized name to `ProofFlag::Bounded`. Both branches silently produce wrong-but-typed values instead of erroring.

## Evidence

```rust
// crates/vb_storage/src/admission/types.rs:53-79
pub enum ProofFlag {
    Bounded,
    TaintSafe,
    RetrySafe,
    Replayable,
}

impl ProofFlag {
    #[allow(dead_code, reason = "reserved for future flag-name parsing")]
    pub(crate) fn from_flag_name(name: &str) -> Self {
        match name {
            "bounded" => Self::Bounded,
            "taint_safe" => Self::TaintSafe,
            "retry_safe" => Self::RetrySafe,
            "idempotency_verified" => Self::RetrySafe,           // <-- wrong variant
            "replayable" => Self::Replayable,
            _ => Self::Bounded,                                  // <-- silent default
        }
    }
}
```

The enum has no `IdempotencyVerified` variant even though `VerificationProof` has an `idempotency_verified_claimed: bool` field (`crates/vb_storage/src/admission/types.rs:108`) and `MissingProofFlag::IdempotencyVerified` exists in `crates/vb_storage/src/admission/record.rs:151-157` with `as_str()` returning `"idempotency_verified"`. The string round-trips through `MissingProofFlag::as_str()` but does not round-trip through `ProofFlag::from_flag_name`.

## Adversarial Check

The function is currently `#[allow(dead_code)]` so no in-tree code exercises it. The defect is latent: as soon as a caller wires it into error-message parsing (the doc-comment says "reserved for future flag-name parsing"), the wrong-variant return becomes a silent correctness bug. The `VerificationProof::idempotency_verified_claimed` field and `MissingProofFlag::IdempotencyVerified` variant both exist, so any future caller will reasonably expect `from_flag_name("idempotency_verified")` to map to something idempotency-related, not to `RetrySafe`. The silent catch-all default compounds the issue: typos in flag names will silently produce `Bounded` instead of failing.

## Suggested Fix

Add an `IdempotencyVerified` variant to `ProofFlag` (matching `MissingProofFlag`), make `from_flag_name` return `Option<Self>` or `Result<Self, _>`, and remove the silent catch-all:

```rust
pub enum ProofFlag {
    Bounded,
    TaintSafe,
    RetrySafe,
    IdempotencyVerified,
    Replayable,
}

impl ProofFlag {
    pub(crate) fn from_flag_name(name: &str) -> Option<Self> {
        match name {
            "bounded" => Some(Self::Bounded),
            "taint_safe" => Some(Self::TaintSafe),
            "retry_safe" => Some(Self::RetrySafe),
            "idempotency_verified" => Some(Self::IdempotencyVerified),
            "replayable" => Some(Self::Replayable),
            _ => None,
        }
    }
}
```
