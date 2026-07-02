# Error Taxonomy — vb-5bqmr

**Bead**: vb-5bqmr — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)

This taxonomy ONLY covers the errors added or re-classified by this bead. Existing crate-wide error surfaces (`RecoveryError`, `EngineError`, `CollectExtraHydrationFailureKind`) are documented where the bead adds a translation, but not redefined.

## 1. Layered error surfaces (railway)

```
producer bytes           decoder                    recovery caller            collect caller
────────────────         ────────────────────       ───────────────────────    ──────────────────────────
[&[u8]]                  SlotWrittenExtraError      RecoveryError             EngineError
                         ┌───────────────┐           ┌──────────────────┐     ┌──────────────────┐
                         │ EncodeFailed  │ ──▶       │ CorruptSlotTaint │     │ CollectExtra-    │
                         │ AllocationFaild           │ { slot }         │     │ HydrationFailed  │
                         │ DecodeFailed │ ──▶ (1.1)  │                  │     │ {kind=Decode-    │
                         │ VersionMismatch│ ──▶ (1.2)│ CorruptSlotTaint │     │  Failed, ...}    │
                         │ { found: u8 }             │ { slot } (+ warn │     │                  │
                         │             │             │ log carrying     │     │ CollectExtra-    │
                         │             │             │ `found`)         │     │ HydrationFailed  │
                         └───────────────┘           └──────────────────┘     │ {kind=Version-   │
                                                                          ──▶ │  Mismatch, ...}  │
                                                                          (2) └──────────────────┘
```

## 2. Codec-layer errors (decoder site, `crates/vb_storage/src/slot_extra.rs`)

### 2.1 `SlotWrittenExtraError::EncodeFailed`

- **Source**: `encode_slot_written_extra`, line ~46.
- **Trigger**: `postcard::to_allocvec` returns `Err`.
- **Category**: persistence failure (codec).
- **Reachable from decode path**: NO (decode path cannot produce a `EncodeFailed`). Callers pattern-match defensively for it.
- **Severity**: warning (encoder-side; not behavior-affecting on recovery — see §5.1).

### 2.2 `SlotWrittenExtraError::AllocationFailed`

- **Source**: `encode_slot_written_extra`, lines ~50, ~53.
- **Trigger**: `Vec::try_reserve` fails OR `checked_add` overflows.
- **Category**: persistent storage failure (host memory pressure or arithmetic overflow).
- **Reachable from decode path**: NO (decode allocates nothing).
- **Severity**: error (host has insufficient memory for envelope output).

### 2.3 `SlotWrittenExtraError::DecodeFailed` (RECLASSIFIED)

- **Source**: `decode_slot_written_extra`, NEW body.
- **Trigger**: `bytes[..4] == MAGIC && bytes[4] == VERSION && postcard::from_bytes fails`.
- **Distinct from**: legacy bytes (returns `Ok(LegacyFrameExtra(_))`) and from unknown-version bytes (returns `VersionMismatch`).
- **Category**: parser/codec (v1 corrupt payload).
- **Behavior-affecting**: yes — a corrupt v1 envelope causes the recovery gate to emit `RecoveryError::CorruptSlotTaint { slot }` (or `EngineError::CollectExtraHydrationFailed { kind: DecodeFailed, ... }`). This is the SAME behavior as before this bead. No regression.
- **Severity**: error (durable taint metadata was present but malformed).

### 2.4 `SlotWrittenExtraError::VersionMismatch { found }` (NEW)

- **Source**: `decode_slot_written_extra`, NEW body.
- **Trigger**: `bytes.len() >= MAGIC.len() + 1 && bytes[..4] == MAGIC && bytes[4] != SLOT_WRITTEN_EXTRA_VERSION`.
- **Reachable values for `found`**: any `u8` except `SLOT_WRITTEN_EXTRA_VERSION` (currently `0x01`). Practically `0x00` (rare) and `0x02..=0xFF` (future dialects).
- **Category**: parser/codec (forward-compat rejection).
- **Behavior-affecting**: yes — a previously-silently-accepted payload now produces an `Err` at the recovery / collect boundary.
- **Severity**: error (durable taint metadata has a magic we recognize but a dialect we do not). Operator action required: upgrade the runtime or migrate data off the unrecognized payload.
- **Diagnostic content**: the `found` byte is logged via `tracing::warn!` at the recovery call site (line 220) and at the collect call site (line 248).
- **Distinct from `DecodeFailed`**: same shape (`Err(_)`) but different discrimination. See §3.

## 3. Discriminability matrix (caller must distinguish)

After this bead, the recovery caller at `hydrate.rs:220` and the collect caller at `collect.rs:248` MUST distinguish ALL FOUR `SlotWrittenExtraError` variants in their `match`. Conflating `VersionMismatch` with `DecodeFailed` is a NO-OP (both translate to the same recovery / runtime error kind), but failing to distinguish them silently reintroduces the original bug if the translation collapses an `Err` to `Ok(LegacyFrameExtra(_))` — which neither translation does today.

| Discriminator | Behavior-affecting | What code distinguishes it |
|---|---|---|
| `Ok(Envelope(_))` | yes | `recovery` recovers taint; `collect` hydrates `frame_extra` |
| `Ok(LegacyFrameExtra(_))` | yes | both flows use `legacy_frame_extra_recovered_slot_taint` / `hydrate_frame_extra` |
| `Err(VersionMismatch { found })` | yes | NEW — explicit `VersionMismatch` arm with `tracing::warn!` and a typed translation |
| `Err(DecodeFailed)` | yes | existing `CorruptSlotTaint { slot }` / `kind: DecodeFailed` |
| `Err(EncodeFailed)` / `Err(AllocationFailed)` | n/a | defensive arms; unreachable from the decode path today |

## 4. Recovery-side translation (`recovery/replay/summary/hydrate.rs:220`)

```rust
match decode_slot_written_extra(bytes) {
    Ok(DecodedSlotWrittenExtra::Envelope(env)) => /* ... */,
    Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_)) => /* ... */,
    Err(SlotWrittenExtraError::VersionMismatch { found }) => {
        tracing::warn!(slot = ?slot, found, "slot extra: VBSE magic present but unknown version");
        Err(RecoveryError::CorruptSlotTaint { slot })
    }
    Err(SlotWrittenExtraError::DecodeFailed)
    | Err(SlotWrittenExtraError::EncodeFailed)
    | Err(SlotWrittenExtraError::AllocationFailed) => Err(RecoveryError::CorruptSlotTaint { slot }),
}
```

> Note: `RecoveryError` is `#[non_exhaustive]` and already carries `CorruptSlotTaint`. The version byte flows through `tracing::warn!` and not through the `RecoveryError` fields. Adding a `RecoveryError::SlotExtraVersionMismatch { slot, found }` variant is DEFERRED.

## 5. Collect-side translation (`runtime/primitives/collect.rs:248`)

```rust
match decode_slot_written_extra(extra) {
    Ok(DecodedSlotWrittenExtra::Envelope(env)) => /* hydrate */,
    Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(bytes)) => /* legacy hydrate */,
    Err(SlotWrittenExtraError::VersionMismatch { found }) => {
        tracing::warn!(slot = ?slot, seq = ?seq, found, "slot extra: VBSE magic present but unknown version");
        Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::VersionMismatch, // NEW
            run_id: run,
            collector_slot: slot,
            event_seq: Some(core_event_seq(seq)),
        })
    }
    Err(SlotWrittenExtraError::DecodeFailed)
    | Err(SlotWrittenExtraError::EncodeFailed)
    | Err(SlotWrittenExtraError::AllocationFailed) => {
        Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::DecodeFailed, // existing
            run_id: run,
            collector_slot: slot,
            event_seq: Some(core_event_seq(seq)),
        })
    }
}
```

## 6. `CollectExtraHydrationFailureKind` (NEW arm)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CollectExtraHydrationFailureKind {
    // … existing variants retained …
    VersionMismatch,
}
```

This variant carries no payload; the version byte is log-only at this layer.

## 7. Forbidden conflations (post-fix)

| Conflation | Forbidden because |
|---|---|
| `Ok(LegacyFrameExtra(_))` for `b"VBSE\x02"` bytes | Original bug; explicit `Err(VersionMismatch)` is now in place. |
| `Err(DecodeFailed)` for unknown-version bytes | same; `Err(VersionMismatch)` is the explicit outcome. |
| `Err(VersionMismatch)` for v1-prefix corrupt payload | `Err(DecodeFailed)` is the explicit outcome. |
| Translating `VersionMismatch` to `Ok(LegacyFrameExtra(_))` at any layer | would silently re-introduce the downgrade bug. |
| Translating `DecodeFailed` to `Ok(VersionMismatch)` or vice versa | distinct semantics; conflating them hides the bug class. |
| Catching every `Err(_) => ...` without an explicit VersionMismatch arm | re-introduces the same shape of bug at the propagation layer. |

## 8. Recovery-behavior surface (NOT widened in this bead)

| Layer | Before bead | After bead |
|---|---|---|
| `SlotWrittenExtraError` variants | 3 (`EncodeFailed`, `AllocationFailed`, `DecodeFailed`) | 4 (`EncodeFailed`, `AllocationFailed`, `DecodeFailed`, `VersionMismatch { found }`) |
| `RecoveryError::CorruptSlotTaint` field | `slot: SlotIdx` | unchanged |
| `EngineError::CollectExtraHydrationFailed` `kind` arm | includes `DecodeFailed` | additionally includes `VersionMismatch` |
| `CollectExtraHydrationFailureKind` | pre-existing variants | one additional variant: `VersionMismatch` |

## 9. Severity classification (each variant)

| Variant | Severity | Operator action |
|---|---|---|
| `VersionMismatch { found }` | error | upgrade runtime / migrate data; durable storage carries an envelope we cannot interpret |
| `DecodeFailed` | error | durable storage carried a corrupt v1 envelope; data corruption needs investigation |
| `AllocationFailed` (encoder only) | warning | host memory pressure; recoverable by retry once pressure subsides |
| `EncodeFailed` (encoder only) | error | postcard encoding failed; check envelope payload for unsupported types |

## 10. Migration impact

- Operators with v1-only writers: zero behavior change (the v1 prefix is unchanged).
- Operators with v2+ writers (if such writers ever existed): previously their bytes were silently treated as legacy; now they fail closed at the recovery / collect boundary with explicit `VersionMismatch` errors and warn-level logs. This is the desired fail-closed behavior.
- Test fixtures (`b"\x01\x02\x03\x04"` in `recovery_bdd_tests.rs:3172`): these do NOT start with `b"VBSE"` and continue to be classified as `LegacyFrameExtra`. NO test regression.
