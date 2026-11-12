---
section: 14
title: "Effect and Retry Classification"
parent: velvet-ballistics-MASTER.md
---

## 14. Effect and Retry Classification

Every action declares side effect and retry safety.

```rust
pub enum SideEffect {
    Pure,
    LocalRead,
    LocalWrite,
    ExternalRead,
    ExternalWrite,
    Process,
    UnsafeShell,
    Unknown,
}

pub enum RetrySafety {
    AlwaysSafe,
    RequiresIdempotencyKey,
    NeverSafe,
    Unknown,
}

pub enum IdempotencyScope {
    PerActionTicket,
    PerRunStep,
    PerBusinessObject,
    ProviderNativeKey,
}
```

Retry defaults:

| Side effect | Default retry rule |
|---|---|
| `Pure` | allowed |
| `LocalRead` | allowed when manifest declares `AlwaysSafe` |
| `ExternalRead` | allowed when manifest declares `AlwaysSafe` |
| `ExternalWrite` | requires idempotency key and manifest attestation |
| `LocalWrite` | requires idempotency key or explicit policy override |
| `Process` | rejected by default |
| `UnsafeShell` | rejected by default |
| `Unknown` | rejected |

`Process`, `UnsafeShell`, and `Unknown` require explicit policy opt-in and are forbidden under `strict_ai`.

---

