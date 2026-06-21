# SC-001: `IndexStatusState::to_u8` produces key collisions for `Other(v)` when `v` overlaps named discriminants

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_storage/src/types/index.rs:36-43`
- **Confidence**: confirmed

## Description

`IndexStatusState::to_u8` returns the inner payload byte for `Other(v)` rather than the discriminant (255). When `v ∈ {0, 1, 2}` (or any byte used by `Submitted`/`Active`/`Completed`), the encoded byte collides with the named variant, so `Other(0).to_u8()` and `Submitted.to_u8()` both produce `0u8`. The encoding is not invertible: round-tripping `Other(0)` through `to_u8`/`from_u8` yields `Submitted`, silently losing the original variant.

## Evidence

```rust
// crates/vb_storage/src/types/index.rs:25-43
pub const fn from_u8(value: u8) -> Self {
    match value {
        0 => Self::Submitted,
        1 => Self::Active,
        2 => Self::Completed,
        _ => Self::Other(value),
    }
}
pub const fn to_u8(self) -> u8 {
    match self {
        Self::Submitted => 0,
        Self::Active => 1,
        Self::Completed => 2,
        Self::Other(v) => v,                  // <-- discards discriminant
    }
}
```

`index_status_key` (`crates/vb_storage/src/keys/encode.rs:74-89`) embeds `state.to_u8()` directly into the Fjall key, so `Other(0)`, `Other(1)`, `Other(2)` produce byte-identical keys to `Submitted`, `Active`, `Completed`. Two semantically distinct states collapse onto the same keyspace row, and last-write-wins during inserts.

## Adversarial Check

The natural construction path is `from_u8(byte)` which never emits `Other(0..=2)`. However, the type's public surface allows direct construction `IndexStatusState::Other(0)` (the variant and payload are both `pub` via the enum's public visibility, and `Other(u8)` has no constructor gate). Any caller — internal or third-party via the crate's public API — that builds `Other(v)` with `v < 3` will silently corrupt the index. The variant is also reachable via Serde deserialization (`#[derive(Serialize, Deserialize)]` would need to be present — the type currently lacks it, but `from_u8` is `pub const` and trivially callable). The collision is on the encoded byte that determines the storage key, so this is not a theoretical concern: it is a keyspace-integrity defect.

## Suggested Fix

Either (a) reject `Other(v)` with `v < 3` at construction time, or (b) encode the discriminant alongside the payload so the wire format is invertible (e.g., reserve a high sentinel byte for `Other` and append the payload elsewhere). The minimal fix is to make `from_u8`/`to_u8` a strict bijection over `0..=2` and reject the remainder as `Other(_)` only when paired with a separate payload byte.
