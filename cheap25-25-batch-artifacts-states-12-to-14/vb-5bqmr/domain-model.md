# Domain Model — vb-5bqmr

**Bead**: vb-5bqmr — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)
**Contract owner**: rust-contract (State 3)
**Scope**: `crates/vb_storage/src/slot_extra.rs` decoder tightening + recovery/runtime error propagation.
**Non-goal**: Encoder change; fuzz harness authoring (RED QUEEN §M3 tracked as `vb-1rqz7.15`); `PayloadTooLarge` cap (tracked as `vb-1rqz7.15.15`).

## 1. Ubiquitous Language

| Term | Definition | Owning type |
|---|---|---|
| **VBSE envelope** | A versioned (magic + version + payload) record of a `Taint` and an optional `Vec<u8>` frame-extra payload, serialized into a `Vec<u8>` produced by `encode_slot_written_extra`. | `SlotWrittenExtraEnvelope` |
| **VBSE magic** | The 4-byte ASCII literal `b"VBSE"` that prefixes every versioned envelope. NEVER appears in legacy frame-extra bytes (which were authored before the envelope was introduced). | `SLOT_WRITTEN_EXTRA_MAGIC` (`&[u8; 4]`) |
| **VBSE version byte** | A single byte immediately following the magic that names the codec variant of the envelope payload. Only `0x01` is currently defined. | `SLOT_WRITTEN_EXTRA_VERSION` (`u8`) |
| **VBSE prefix** | The historical 5-byte constant `SLOT_WRITTEN_EXTRA_PREFIX = b"VBSE\x01"` that conflates magic + version. Retained for backward source compatibility but documented as compositional. | `SLOT_WRITTEN_EXTRA_PREFIX` (`&[u8; 5]`) |
| **Envelope payload** | The postcard-serialized `SlotWrittenExtraEnvelope` placed after the prefix. | implicit `&[u8]` slice |
| **Decoded envelope** | A successful round-trip from `bytes` to `SlotWrittenExtraEnvelope`; in `DecodedSlotWrittenExtra::Envelope(...)`. | `DecodedSlotWrittenExtra` |
| **Legacy frame extra** | A `&[u8]` that does NOT start with the VBSE magic; treated as opaque frame-extra bytes authored by pre-VBSE producers. Preserved verbatim through the hydration pipeline. | `DecodedSlotWrittenExtra::LegacyFrameExtra(&'a [u8])` |
| **Unknown VBSE version** | A `&[u8]` whose first four bytes equal `SLOT_WRITTEN_EXTRA_MAGIC` but whose 5th byte is NOT `SLOT_WRITTEN_EXTRA_VERSION`. Distinct from "corrupt payload", which has the v1 prefix but a malformed postcard body. | `SlotWrittenExtraError::VersionMismatch { found }` |
| **Corrupt v1 envelope** | A `&[u8]` with the full v1 prefix but an unparseable postcard body. Distinct from unknown version and from legacy frame extra. | `SlotWrittenExtraError::DecodeFailed` |
| **Taint-sidecar** | The `taint: Taint` field of the envelope that records provenance for the slot value. Master §47 lattice: `Clean ⊑ DerivedFromSecret ⊑ Secret`. | `SlotWrittenExtraEnvelope::taint` |
| **Hydration** | Recovery-time parsing of journal events back into runtime state. Taint sidecar deserialization is the site of this bead's discriminated error. | `decode_slot_written_extra` |

## 2. Entities, Value Objects, Aggregates

### 2.1 Value objects (newtypes + smart constructors)

| Name | Underlying | Invariants | Smart constructor |
|---|---|---|---|
| `SlotWrittenExtraMagic` (newtype proposal, deferred) | `&'static [u8; 4]` | Always exactly `b"VBSE"`; cannot be constructed by external code without `#[doc(hidden)] pub const fn new()`. | `pub const SLOT_WRITTEN_EXTRA_MAGIC: &[u8; 4] = b"VBSE";` |
| `SlotWrittenExtraVersion` (newtype proposal, deferred) | `u8` | Only the literal `0x01` is a recognized version. | `pub const SLOT_WRITTEN_EXTRA_VERSION: u8 = 0x01;` |
| `VbseFoundVersion` (newtype in error) | `u8` | Any value `0x00..=0xFF`. In the current path only values in `0x02..=0xFF` and `0x00` are reachable; values `> 0x01` trigger the unknown-version rejection. | inferred; field is named `found: u8` |

> **Domain decision (contract-ratified)**: We ratify surface (a) from `codebase-map.md §5 Q1` — add `VersionMismatch` only to `SlotWrittenExtraError`; do NOT add a new variant to `RecoveryError` or to `EngineError` enum roots in this bead. Recovery/runtime callers pattern-match on `SlotWrittenExtraError::VersionMismatch { found }` and translate into their own existing failures (see §3.3). This minimizes the blast radius through the compile-time exhaustiveness check in `recovery_unit_tests.rs:1149` and the `PartialEq`/`Display` impls in `recovery/types.rs:142-336`.
>
> **Domain decision (contract-ratified)**: We ratify surface (i) for the collect side — add `CollectExtraHydrationFailureKind::VersionMismatch` (reusing the existing `CollectExtraHydrationFailed` `EngineError` variant, which already carries the `kind` discriminator). No new `EngineError` variant is required.
>
> **Domain decision (contract-ratified)**: We ratify magic hoisting — replace the single 5-byte `SLOT_WRITTEN_EXTRA_PREFIX` constant with TWO public constants: `SLOT_WRITTEN_EXTRA_MAGIC: &[u8; 4] = b"VBSE"` and `SLOT_WRITTEN_EXTRA_VERSION: u8 = 0x01`. The 5-byte `SLOT_WRITTEN_EXTRA_PREFIX` constant is RETAINED (for backward compat) but documented as the concatenation of the two. The encoder and the new decoder are recomputed as `const { concat }` so the byte sequence is provably identical to the historical value.

### 2.2 Envelope (entity)

`SlotWrittenExtraEnvelope { taint: Taint, frame_extra: Option<Vec<u8>> }` is the durable sidecar. No invariants added; the envelope is opaque to the codec layer apart from the discriminator.

### 2.3 Result enum

```rust
pub enum DecodedSlotWrittenExtra<'a> {
    Envelope(SlotWrittenExtraEnvelope),
    LegacyFrameExtra(&'a [u8]),
}
```

Lifetime `'a` borrowed from `bytes`; no allocation when the legacy branch is taken. The `LegacyFrameExtra` arm remains in place because pre-VBSE producers (and the `b"\x01\x02\x03\x04"` byte sequences used in BDD fixtures) MUST continue to classify as legacy.

### 2.4 Error surface

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SlotWrittenExtraError {
    EncodeFailed,
    AllocationFailed,
    DecodeFailed,                        // v1 prefix present, postcard payload corrupt
    VersionMismatch { found: u8 },       // magic-present, version byte != SLOT_WRITTEN_EXTRA_VERSION
}
```

`#[non_exhaustive]` already in place; adding `VersionMismatch` is API-additive.

## 3. Failure Lattice for the Codec Boundary

Three mutually exclusive outcomes of `decode_slot_written_extra(bytes)`:

| Branch | Discriminator | Result | Why |
|---|---|---|---|
| **v1 envelope** | `bytes.len() >= MAGIC.len() + 1 && bytes[..MAGIC.len()] == MAGIC && bytes[MAGIC.len()] == VERSION` AND `postcard::from_bytes` succeeds | `Ok(DecodedSlotWrittenExtra::Envelope(envelope))` | happy path |
| **Corrupt v1 payload** | full v1 prefix matches AND `postcard::from_bytes` returns `Err` | `Err(SlotWrittenExtraError::DecodeFailed)` | v1 was the writer's intent; the bytes are malformed |
| **Unknown VBSE version** | `bytes.len() >= MAGIC.len() + 1 && bytes[..MAGIC.len()] == MAGIC && bytes[MAGIC.len()] != VERSION` | `Err(SlotWrittenExtraError::VersionMismatch { found: bytes[MAGIC.len()] })` | explicit rejection: writer is from a future runtime we do not understand; must fail closed |
| **Legacy frame extra** | `bytes.len() < MAGIC.len() + 1` OR `bytes[..MAGIC.len()] != MAGIC` | `Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(bytes))` | preserve pre-envelope producer contract |

> **Branch ordering matters**: the `Magic-but-unknown-version` check must happen BEFORE the postcard decode attempt, because the postcard framing for v2+ envelopes may not even be parseable as a v1 envelope and we MUST NOT silently fall through to `LegacyFrameExtra`. This is the entire bug fix.

## 4. Forbidden States

A successfully decoded `bytes` input MUST satisfy exactly ONE of these properties; the type system cannot enforce uniqueness (external input), so the decoder enforces it as a `match`:

| Forbidden state | Why illegal | Caught by |
|---|---|---|
| `bytes.starts_with(b"VBSE\x02")` returning `Ok(LegacyFrameExtra(_))` | silent downgrade of a future-version writer | new `VersionMismatch` arm |
| `bytes.starts_with(b"VBSE\xFF")` returning `Ok(LegacyFrameExtra(_))` | same | new `VersionMismatch` arm |
| `bytes == b"VBSE"` (4-byte magic only, no version byte) returning `Ok(LegacyFrameExtra(_))` | magic-present-but-truncated MUST reject too | new `VersionMismatch` arm (treats missing version byte as version 0 mismatch) |
| v1 envelope being parsed when the discriminator is corrupt | would corrupt taint | `DecodeFailed` arm |
| A new `SlotWrittenExtraError` variant being silently dropped by a recovery/runtime caller | mirrors the original bug | `match` propagation contract (see workflow-model.md) |

## 5. Policies and Invariants (behavior-affecting)

| ID | Policy | Invariant |
|---|---|---|
| P-001 | **Unknown versions fail closed**. The decoder MUST return `Err(VersionMismatch { found })` for any `bytes` whose first four bytes are the VBSE magic but whose 5th byte is not the v1 version. | `bytes.len() >= 5 && bytes[..4] == MAGIC && bytes[4] != VERSION => Err(VersionMismatch { found: bytes[4] })` |
| P-002 | **Legacy path preserved**. Bytes with no VBSE magic continue to flow through `LegacyFrameExtra`. | `bytes.len() < 4 || bytes[..4] != MAGIC => Ok(LegacyFrameExtra(bytes))` |
| P-003 | **v1 corruption is `DecodeFailed`, not `VersionMismatch`**. The two failures are distinct and distinguishable. | post-refinement: `DecodeFailed::payload_corrupt(i) != VersionMismatch::version_byte(j)` for all `i, j` |
| P-004 | **Constant compositionality**. `SLOT_WRITTEN_EXTRA_PREFIX` is provably equal to `[MAGIC, &[VERSION]].concat()`. | compile-time `const { ... }` expression; or compile-fail test asserting byte equality. |
| P-005 | **No allocation on legacy path**. `decode_slot_written_extra(b"\x01\x02\x03\x04")` returns `Ok(LegacyFrameExtra(&[...]))` with zero `Vec` allocations. | ownership of the borrow `'a` enforces; Kani `cover!` on the allocation counter. |
| P-006 | **Recovery-side translation is total and exhaustive**. `decoded_slot_taint` (hydrate.rs:220) translates every `SlotWrittenExtraError` variant into exactly one `RecoveryError`. `VersionMismatch { found }` becomes `RecoveryError::CorruptSlotTaint { slot, cause: SlotTaintCause::VersionMismatch { found } }` OR (per recommendation in §6) collapses to `CorruptSlotTaint { slot }` with a parallel diagnostic if the runtime chooses not to widen `RecoveryError`. | exhaustive `match` on the new enum in hydrate.rs |
| P-007 | **Collect-side translation is total and exhaustive**. `hydrate_slot_written_extra` (collect.rs:248) translates every `SlotWrittenExtraError` variant into exactly one `EngineError`. | exhaustive `match` on the new enum in collect.rs |
| P-008 | **Encoder is unchanged**. `encode_slot_written_extra` always emits the v1 envelope. No bump of any version counter in this bead. | unchanged function body |

## 6. Recovery-side error translation (contract decision)

Recovery callers do NOT need to gain a new `RecoveryError` variant in this bead. They branch on the storage-layer `VersionMismatch` and translate it to `RecoveryError::CorruptSlotTaint { slot }` (the existing variant already covers "durable slot taint metadata was present but could not be decoded"). The `found: u8` byte is captured in logs/diagnostics, not in `RecoveryError` fields.

> **Rationale**: Master §19 (typed storage-error surface) requires discriminated failure, not field-level cause. The discriminator is already present at the `SlotWrittenExtraError` layer; widening `RecoveryError` would duplicate the discriminator and break the compile-time exhaustiveness check. If a future bead needs `RecoveryError` to carry the version byte (for UI/diagnostics), it adds a new variant with its own `#[non_exhaustive]` additive change. **Decision**: surface (a) ratified; surface (b) deferred to a future bead if needed.

## 7. Runtime-side error translation (contract decision)

`CollectExtraHydrationFailureKind` (in `vb_core::errors`) gains a single new arm:

```rust
pub enum CollectExtraHydrationFailureKind {
    DecodeFailed,           // existing — v1 prefix + bad postcard body
    VersionMismatch,        // NEW — magic + unknown version byte
    // … any other existing arms preserved
}
```

The collect handler returns `EngineError::CollectExtraHydrationFailed { kind: VersionMismatch, run_id, collector_slot, event_seq }`. No new `EngineError` variant.

## 8. Migration / Forward Compatibility

A writer that emits `b"VBSE\x02"` or any other unrecognised version MUST be rejected. Future beads that introduce v2 envelopes:

1. Add `SLOT_WRITTEN_EXTRA_VERSION_V2` and a v2 branch in BOTH `encode_slot_written_extra` and `decode_slot_written_extra`.
2. Bump `SLOT_WRITTEN_EXTRA_VERSION` to the highest known version OR introduce an envelope-version cursor.
3. Update this contract; do not silently widen the legacy fallback.

This bead makes that path explicit instead of letting it silently degrade.
