# Type Contracts — vb-5bqmr

**Bead**: vb-5bqmr — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)
**Owns**: type-level surface for `vb_storage::slot_extra` and propagation sites at `recovery/replay/summary/hydrate.rs` + `runtime/primitives/collect.rs`.

## 1. Constants (hoist + composable)

```rust
// crates/vb_storage/src/slot_extra.rs (lines 7, NEW hoisted)

/// 4-byte ASCII magic that disambiguates versioned envelopes from legacy frame-extra bytes.
pub const SLOT_WRITTEN_EXTRA_MAGIC: &[u8; 4] = b"VBSE";

/// Currently-defined VBSE envelope version byte (v1).
pub const SLOT_WRITTEN_EXTRA_VERSION: u8 = 0x01;

/// Historical 5-byte prefix = `[MAGIC, &[VERSION]].concat()`.
///
/// RETAINED for source compatibility with downstream crates that re-export it
/// from `vb_storage::slot_extra`. New code SHOULD compose the prefix from
/// `SLOT_WRITTEN_EXTRA_MAGIC` and `SLOT_WRITTEN_EXTRA_VERSION`. The byte
/// sequence is provably equal to `b"VBSE\x01"` (see compile-time equality test
/// in `slot_extra_tests::prefix_constant_matches_composition`).
pub const SLOT_WRITTEN_EXTRA_PREFIX: &[u8; 5] = const {
    // Compile-time concat using `const_panic` is unavailable on stable;
    // use a hand-assembled array initialiser so any future drift fails to compile.
    let mut out = [0u8; 5];
    out[0] = SLOT_WRITTEN_EXTRA_MAGIC[0];
    out[1] = SLOT_WRITTEN_EXTRA_MAGIC[1];
    out[2] = SLOT_WRITTEN_EXTRA_MAGIC[2];
    out[3] = SLOT_WRITTEN_EXTRA_MAGIC[3];
    out[4] = SLOT_WRITTEN_EXTRA_VERSION;
    &out
};
// Alternative (preferred if `const { concat!(...) }` is not available on the
// pinned nightly — see `scripts/check-nightly-features.sh`): assert equality
// with a #[test] that bails the build on drift.

/// Maximum accepted prefix length used by the discriminator pre-check.
pub const SLOT_WRITTEN_EXTRA_PREFIX_LEN: usize = 5; // MAGIC.len() + 1
```

**Why hoist**: today the 5-byte constant conflates two orthogonal facts (the 4-byte magic and the 1-byte version), and that conflation is what enables the silent-downgrade bug. A new reader of the code cannot easily distinguish "magic-but-unknown-version" from "magic-absent" because the prefix is a single 5-byte pattern.

**Why retain `SLOT_WRITTEN_EXTRA_PREFIX`**: it is re-exported from `crates/vb_storage/src/lib.rs:208-211` and downstream crate graphs and BDD fixtures reference it. Removing it is out of scope; making it compositionally derived makes the relationship invariant.

## 2. Error enum (one new variant)

```rust
// crates/vb_storage/src/slot_extra.rs (lines 10-19, ADD a variant)

/// Errors while encoding or decoding the slot-write extra envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SlotWrittenExtraError {
    /// Envelope payload could not be encoded.
    EncodeFailed,
    /// Envelope output allocation failed.
    AllocationFailed,
    /// v1 envelope prefix was present and matched `SLOT_WRITTEN_EXTRA_VERSION`,
    /// but the postcard payload could not be decoded.
    DecodeFailed,
    /// VBSE magic was present but the version byte was not the recognized
    /// `SLOT_WRITTEN_EXTRA_VERSION`. The decoder refuses to silently classify
    /// such bytes as legacy frame extra.
    ///
    /// `found` carries the version byte observed (any value except
    /// `SLOT_WRITTEN_EXTRA_VERSION`).
    VersionMismatch {
        /// Version byte observed after the magic (any value except
        /// `SLOT_WRITTEN_EXTRA_VERSION`).
        found: u8,
    },
}
```

Notes:
- Adding a variant to an enum that is already `#[non_exhaustive]` is **API-additive**. Existing downstream callers that use `_ =>` catch-all arms continue to compile.
- The variant is `Copy`-able so it can flow through the `Result<_, SlotWrittenExtraError>`-typed sites without allocation.
- `PartialEq`/`Eq` derivation continues to work because the field is `u8`.

## 3. Decoder contract (tightened)

```rust
// crates/vb_storage/src/slot_extra.rs (lines 60-69, REPLACE body)

/// Decodes a slot-write extra envelope or classifies legacy frame extra bytes.
///
/// # Behaviour
///
/// - If `bytes` starts with the VBSE magic AND `bytes[MAGIC.len()] == SLOT_WRITTEN_EXTRA_VERSION`,
///   the trailing bytes are decoded as a postcard-serialised
///   `SlotWrittenExtraEnvelope`. On success: `Ok(DecodedSlotWrittenExtra::Envelope(_))`.
///   On postcard failure: `Err(SlotWrittenExtraError::DecodeFailed)`.
/// - If `bytes` starts with the VBSE magic AND `bytes[MAGIC.len()] != SLOT_WRITTEN_EXTRA_VERSION`:
///   `Err(SlotWrittenExtraError::VersionMismatch { found: bytes[MAGIC.len()] })`.
/// - Otherwise: `Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(bytes))`.
///
/// `bytes.len() < MAGIC.len() + 1` automatically falls into the legacy arm
/// (the magic is incomplete and cannot represent a future-version envelope).
pub fn decode_slot_written_extra(
    bytes: &[u8],
) -> Result<DecodedSlotWrittenExtra<'_>, SlotWrittenExtraError> {
    // Branch 1: input is at least MAGIC.len() + 1 bytes long.
    if let Some((magic, rest)) = bytes.split_at_checked(SLOT_WRITTEN_EXTRA_MAGIC.len()) {
        if magic == SLOT_WRITTEN_EXTRA_MAGIC {
            if let Some((&version, payload)) = rest.split_first() {
                if version == SLOT_WRITTEN_EXTRA_VERSION {
                    return postcard::from_bytes::<SlotWrittenExtraEnvelope>(payload)
                        .map(DecodedSlotWrittenExtra::Envelope)
                        .map_err(|_| SlotWrittenExtraError::DecodeFailed);
                } else {
                    return Err(SlotWrittenExtraError::VersionMismatch { found: version });
                }
            }
        }
    }
    // Branch 2 / fallback: legacy frame extra, preserved verbatim.
    Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(bytes))
}
```

Equivalence note: the historical implementation used `match bytes.strip_prefix(SLOT_WRITTEN_EXTRA_PREFIX)`. The new body is observationally equivalent for all v1 inputs and the historical `LegacyFrameExtra` inputs, but additionally rejects the magic-but-unknown-version branch.

## 4. Recovery-side translation (type contract)

```rust
// crates/vb_storage/src/recovery/replay/summary/hydrate.rs (lines 220-235, REPLACE the collapse arm)

fn decoded_slot_taint(
    slot: SlotIdx,
    value: SlotValue,
    bytes: &[u8],
) -> RecoveryResult<RecoveredSlotTaint> {
    match crate::slot_extra::decode_slot_written_extra(bytes) {
        Ok(DecodedSlotWrittenExtra::Envelope(envelope)) => Ok(RecoveredSlotTaint {
            taint: envelope.taint,
            unsupported: false,
        }),
        Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_)) => {
            Ok(legacy_frame_extra_recovered_slot_taint(value))
        }
        Err(crate::slot_extra::SlotWrittenExtraError::VersionMismatch { found }) => {
            // Diagnostic-friendly log site (not in `RecoveryError`).
            tracing::warn!(slot = ?slot, found, "slot extra: VBSE magic present but unknown version");
            Err(RecoveryError::CorruptSlotTaint { slot })
        }
        Err(crate::slot_extra::SlotWrittenExtraError::DecodeFailed)
        | Err(crate::slot_extra::SlotWrittenExtraError::EncodeFailed)
        | Err(crate::slot_extra::SlotWrittenExtraError::AllocationFailed) => {
            // Defensive: `DecodeFailed` is the only reachable variant today for
            // decoded bytes, but the exhaustive match prevents regression if a
            // future variant is added without an explicit branch here.
            Err(RecoveryError::CorruptSlotTaint { slot })
        }
    }
}
```

> The collapse for the non-version-mismatch error arms is unchanged from today. The new arm lifts the version byte into a `tracing::warn!` for diagnostics without widening `RecoveryError`.

## 5. Runtime-side translation (type contract)

```rust
// crates/vb_runtime/src/primitives/collect.rs (lines 248-275, REPLACE the collapse arm)

fn hydrate_slot_written_extra(
    &mut self,
    run: RunId,
    slot: SlotIdx,
    seq: vb_storage::EventSeq,
    value: Option<&[u8]>,
    extra: &[u8],
) -> Result<(), EngineError> {
    match vb_storage::decode_slot_written_extra(extra) {
        Ok(vb_storage::DecodedSlotWrittenExtra::Envelope(envelope)) => match envelope.frame_extra {
            Some(frame_extra) => self.hydrate_frame_extra(run, slot, seq, value, &frame_extra),
            None => Ok(()),
        },
        Ok(vb_storage::DecodedSlotWrittenExtra::LegacyFrameExtra(frame_extra)) => {
            self.hydrate_frame_extra(run, slot, seq, value, frame_extra)
        }
        Err(vb_storage::SlotWrittenExtraError::VersionMismatch { found }) => {
            // Logged at the runtime boundary; surfaced to the runtime error lattice
            // via the existing `EngineError` carrier and the new discriminator arm.
            tracing::warn!(
                slot = ?slot,
                seq = ?seq,
                found,
                "slot extra: VBSE magic present but unknown version"
            );
            Err(EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::VersionMismatch,
                run_id: run,
                collector_slot: slot,
                event_seq: Some(core_event_seq(seq)),
            })
        }
        Err(vb_storage::SlotWrittenExtraError::DecodeFailed)
        | Err(vb_storage::SlotWrittenExtraError::EncodeFailed)
        | Err(vb_storage::SlotWrittenExtraError::AllocationFailed) => {
            Err(EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::DecodeFailed,
                run_id: run,
                collector_slot: slot,
                event_seq: Some(core_event_seq(seq)),
            })
        }
    }
}
```

```rust
// crates/vb_core/src/errors.rs — APPEND a variant to `CollectExtraHydrationFailureKind`

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CollectExtraHydrationFailureKind {
    // … existing variants …
    /// The decode path observed the VBSE magic but an unrecognised version byte.
    ///
    /// Distinct from `DecodeFailed` (v1 prefix + corrupt postcard payload):
    /// the writer spoke a dialect we do not understand, so the runtime must
    /// fail closed at the hydration boundary instead of attempting to parse
    /// the payload as v1.
    VersionMismatch,
}
```

## 6. Re-exports (verify only — no edit unless necessary)

`crates/vb_storage/src/lib.rs:208-211` already re-exports the public surface:

```rust
pub use slot_extra::{
    DecodedSlotWrittenExtra,
    SLOT_WRITTEN_EXTRA_PREFIX,
    SlotWrittenExtraEnvelope,
    SlotWrittenExtraError,
    decode_slot_written_extra,
    encode_slot_written_extra,
};
```

After this bead the same lines must additionally re-export `SLOT_WRITTEN_EXTRA_MAGIC` and `SLOT_WRITTEN_EXTRA_VERSION`. (`SLOT_WRITTEN_EXTRA_PREFIX` is retained.)

## 7. Borrow / ownership invariants

| Function | Input | Output | Allocation | Notes |
|---|---|---|---|---|
| `decode_slot_written_extra(bytes: &[u8])` | borrowed | `Result<DecodedSlotWrittenExtra<'_>, SlotWrittenExtraError>` (lifetime tied to `bytes`) | zero on legacy path; one short-lived scratch on the v1 decode path (postcard-driven) | no `unsafe`, no `unwrap/expect`, no panic |
| `encode_slot_written_extra(...)` | owned `Taint`, `Option<Vec<u8>>` | `Result<Vec<u8>, SlotWrittenExtraError>` | one `Vec<u8>` output | unchanged |
| `recovered_slot_taint(...)`, `decoded_slot_taint(...)` | borrowed | owned `RecoveryResult<RecoveredSlotTaint>` | zero | unchanged shape |

## 8. Forbidden / deprecated patterns

| Pattern | Forbidden because |
|---|---|
| Re-introducing `match bytes.strip_prefix(SLOT_WRITTEN_EXTRA_PREFIX)` after this fix | Re-creates the magic-but-unknown-version downgrade bug. |
| Translating `SlotWrittenExtraError::VersionMismatch` to `Ok(LegacyFrameExtra(_))` at any call site | Same bug, expressed at the propagation layer. |
| Adding a new `RecoveryError` variant for `VersionMismatch` in this bead | Out of scope; surfaces (b) deferred to a future bead if needed. |
| Silently widening the legacy fallback to "any magic" | Defeats the purpose of the fix. |
| Returning `Err(VersionMismatch { found: 0 })` for the empty `b"VBSE"` 4-byte prefix | The 4-byte `b"VBSE"` with no 5th byte falls into the legacy arm by virtue of `bytes.len() < MAGIC.len() + 1`; the discriminant check is `version != SLOT_WRITTEN_EXTRA_VERSION`, which is `true` for `version == 0`, so 4-byte `b"VBSE\x00"` (truncated to 5 bytes total) IS rejected. The 4-byte `b"VBSE"` itself (no 5th byte) is legacy-by-construction. Documented in tests. |

## 9. Linter / source-lint envelope

- `#![forbid(unsafe_code)]` is already at the head of `slot_extra.rs`; do NOT remove.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg` (per AGENTS.md engineering rules).
- All checked arithmetic.
- `#[non_exhaustive]` preserved on both new and existing enums.
