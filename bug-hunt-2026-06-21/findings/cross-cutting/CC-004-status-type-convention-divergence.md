# CC-004: `IndexStatusState` and `RunHeaderStatus` use different conventions for handling unknown wire bytes

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_storage/src/types/index.rs:11-44` (`IndexStatusState::from_u8` is total and returns `Other(value)`); `crates/vb_storage/src/records/status.rs:60-110` (`RunHeaderStatus::from_byte` is total but `known()` returns `Result`)
- **Confidence**: confirmed

## Description

Two status-shaped enums in the storage layer handle unknown wire bytes with completely different conventions:

- `IndexStatusState::from_u8(byte)` returns `Other(byte)` for any byte not in `{0, 1, 2}`. There is no `known()` or `classify()` helper; callers must pattern-match the enum directly.
- `RunHeaderStatus::from_byte(byte)` returns a lossless wrapper, with `known()` returning `Result<KnownRunHeaderStatus, UnknownRunHeaderStatus>` and `classify()` returning `RunHeaderStatusClass::Known(_) | Unknown(_)`. Three layered types model the same concept.

Both types exist for the same architectural reason: storage is supposed to outlive individual runtime versions, so unknown status bytes must round-trip losslessly through future-unaware code. But the APIs have diverged, making it hard to write generic code that handles both.

## Evidence

```rust
// crates/vb_storage/src/types/index.rs:22-44
impl IndexStatusState {
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Submitted,
            1 => Self::Active,
            2 => Self::Completed,
            _ => Self::Other(value),
        }
    }
    pub const fn to_u8(self) -> u8 { ... }
}
```

```rust
// crates/vb_storage/src/records/status.rs:49-110
impl RunHeaderStatus {
    pub const fn from_byte(byte: u8) -> Self { Self(byte) }
    pub const fn as_byte(self) -> u8 { self.0 }
    pub const fn known(self) -> Result<KnownRunHeaderStatus, UnknownRunHeaderStatus> { ... }
    pub const fn classify(self) -> RunHeaderStatusClass { ... }
}
```

## Adversarial Check

`IndexStatusState` is embedded directly into a storage key (via `to_u8`), so its wire format must round-trip through Fjall sort order without loss. `RunHeaderStatus` is embedded in a postcard-encoded record payload, so its wire format is the postcard encoding of `u8` — also lossless. Both types have the same architectural constraint (lossless round-trip through future-runtime versions) but expose completely different surfaces. There is no shared trait or convention; a maintainer who learns one API has to re-learn the other.

## Suggested Fix

Define a shared trait (or pattern) for "lossless wrapper around a wire byte with known/unknown classification":

```rust
pub trait WireByte {
    type Known;
    type Unknown;
    fn from_byte(byte: u8) -> Self;
    fn as_byte(self) -> u8;
    fn known(self) -> Result<Self::Known, Self::Unknown>;
    fn classify(self) -> WireByteClass<Self::Known>;
}
```

Implement for both `IndexStatusState` and `RunHeaderStatus`. This also closes SC-001 because the trait forces the implementer to think about how `Other(v)` collides with `Known` variants.
