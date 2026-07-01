# Workflow Model — vb-5bqmr

**Bead**: vb-5bqmr — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)
**Workflow surface**: Codec decode of a `&[u8]` (durable journal payload) into one of three terminal outcomes (envelope | legacy-frame-extra | typed error).

This bead is a SYNTACTIC-CODEC bead. There is no async, no concurrency, no state machine in the temporal sense. The "workflow" is the discriminated-decoder pipeline. Per the master pipeline: "Temporal workflows are covered by loom+proptest." This bead does not have a temporal workflow in that sense; the only relevant workflow property here is the discriminated-decoder outcome lattice.

## 1. Legal states (decode outcome lattice)

```
                    ┌──────────────────────────┐
   bytes: &[u8] ───▶│  decode_slot_written_    │
                    │        extra             │
                    └────────────┬─────────────┘
                                 │
       ┌─────────────────────────┼────────────────────────────┐
       │                         │                            │
       ▼                         ▼                            ▼
  bytes.len() < 5          magic matches AND                magic matches AND
  OR no magic              version == 0x01                  version != 0x01
       │                         │                            │
       ▼                         ▼                            ▼
 LegacyFrameExtra          ┌───────────────┐          VersionMismatch
 (preserved, O)            │ postcard::    │          (fail-closed, X)
       │                   │ from_bytes    │                │
       ▼                   └───────┬───────┘                ▼
  hydrate_frame_extra       ┌──────┴──────┐          RecoveryError::
  or                       │             │          CorruptSlotTaint
  legacy_frame_extra_      Ok(Envelop)   Err         (recovery) /
  recovered_slot_taint       │            │          EngineError::
                            ▼            ▼          CollectExtraHydration
                       recover taint  DecodeFailed  Failed{kind=
                            │            │          VersionMismatch} (collect)
                            ▼            ▼
                       RecoveredSlot-  RecoveryError::
                       Taint.taint     CorruptSlotTaint
                       (success)
```

Three TERMINAL outcomes from the user perspective:

| Outcome | Symbol | Description | Type |
|---|---|---|---|
| Ok-Envelope | E | v1 envelope decoded, taint recovered | `Ok(DecodedSlotWrittenExtra::Envelope(_))` |
| Ok-Legacy | L | bytes preserved as opaque legacy frame extra | `Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_))` |
| Err-Mismatch | X | magic-but-unknown-version rejected | `Err(SlotWrittenExtraError::VersionMismatch { found })` |
| Err-Decode | D | v1 prefix + corrupt postcard | `Err(SlotWrittenExtraError::DecodeFailed)` |

## 2. Transitions

| From | To | Guard | Effect |
|---|---|---|---|
| any | E | `bytes[..4] == MAGIC && bytes[4] == VERSION && postcard::from_bytes succeeds` | terminal |
| any | D | `bytes[..4] == MAGIC && bytes[4] == VERSION && postcard::from_bytes fails` | terminal |
| any | X | `bytes[..4] == MAGIC && bytes[4] != VERSION && bytes.len() >= MAGIC.len() + 1` | terminal; fail-closed |
| any | L | `bytes.len() < MAGIC.len() + 1 || bytes[..4] != MAGIC` | terminal; bytes borrowed |

Each input produces exactly one transition. There are no retries, no cancellation, no async suspension. The decode is `fn` (not `async fn`), no task wakeups, no memory-order fences.

## 3. Guards (pre-conditions)

| Guard | What it checks |
|---|---|
| `bytes.len() >= MAGIC.len() + 1` | ensures the discriminator (version byte) is in-bounds before we attempt to read it |
| `bytes[..MAGIC.len()] == SLOT_WRITTEN_EXTRA_MAGIC` | distinguishes VBSE envelopes from arbitrary legacy bytes |
| `bytes[MAGIC.len()] == SLOT_WRITTEN_EXTRA_VERSION` | distinguishes v1 from future versions |
| `postcard::from_bytes::<SlotWrittenExtraEnvelope>(payload)` succeeds | ensures the payload is well-formed v1 |

## 4. Idempotence

`decode_slot_written_extra(bytes)` is a pure function. Calling it repeatedly on the same `bytes` is observably identical (same return value, no side effects, no caching, no mutation). Idempotence is automatic by virtue of the function being total and deterministic.

## 5. Cancellation / shutdown

N/A — sync, total, no I/O. There is no cancellation point.

## 6. Outcomes and consumer handling

### 6.1 Recovery path

```
hydrate_run_frame_from_events
  └── record_slot_write (hydrate.rs:275)
        └── recovered_slot_taint (hydrate.rs:209)
              └── decoded_slot_taint (hydrate.rs:220) ◀── THIS BEAD
                    ├── Ok(Envelope(_))    → RecoveredSlotTaint (taint recovered)
                    ├── Ok(LegacyFrameExtra) → legacy_frame_extra_recovered_slot_taint (unsupported=true)
                    ├── Err(VersionMismatch{ found }) → RecoveryError::CorruptSlotTaint{ slot } (with tracing warn)
                    ├── Err(DecodeFailed)   → RecoveryError::CorruptSlotTaint{ slot }
                    ├── Err(EncodeFailed)   → RecoveryError::CorruptSlotTaint{ slot }  (defensive — unreachable today for decode paths)
                    └── Err(AllocationFailed) → RecoveryError::CorruptSlotTaint{ slot } (defensive — unreachable today for decode paths)
```

### 6.2 Collect path

```
CollectStates::hydrate_journal_events
  └── CollectStates::hydrate_journal_event (collect.rs:234)
        └── CollectStates::hydrate_slot_written_extra (collect.rs:248) ◀── THIS BEAD
              ├── Ok(Envelope(_))      → hydrate_frame_extra (or no-op)
              ├── Ok(LegacyFrameExtra) → hydrate_frame_extra (legacy)
              ├── Err(VersionMismatch{ found }) → EngineError::CollectExtraHydrationFailed{kind: VersionMismatch, ...} (with tracing warn)
              ├── Err(DecodeFailed)    → EngineError::CollectExtraHydrationFailed{kind: DecodeFailed, ...}
              ├── Err(EncodeFailed)    → EngineError::CollectExtraHydrationFailed{kind: DecodeFailed, ...}  (defensive)
              └── Err(AllocationFailed) → EngineError::CollectExtraHydrationFailed{kind: DecodeFailed, ...} (defensive)
```

## 7. Failure modes (workflow-level)

| Failure | Trigger | Behavior |
|---|---|---|
| `AllocationFailed` (encoder side) | `Vec::try_reserve` fails | upstream `Result` returns `Err`; not reachable on the decoder path |
| `EncodeFailed` (encoder side) | `postcard::to_allocvec` fails | upstream `Result` returns `Err`; not reachable on the decoder path |
| `DecodeFailed` (decoder side) | v1 prefix present, postcard error | upstream `Result` returns `Err` |
| `VersionMismatch` (decoder side, NEW) | magic present, version byte != v1 | upstream `Result` returns `Err` |

Every failure has a distinct semantic meaning and a distinct downstream translation.

## 8. Backend state (durable)

The decoder reads from `SlotWrittenEvent.extra: Option<Vec<u8>>`. That field is durable; the decoder does NOT mutate it. There is no read-modify-write pattern, no transactional boundary around the decode.

## 9. Dispatch model

The decode is invoked inline at two sites:

1. `recovery/replay/summary/hydrate.rs:220` — called per `SlotWrittenEvent` during snapshot recovery.
2. `runtime/primitives/collect.rs:248` — called per `SlotWrittenEvent` during journal hydration (collect path).

Both sites are sync, single-threaded wrt the decode (the recovery/hydration paths are linear over event sequence). No work-stealing, no thread pool, no async runtime involvement for this function.

## 10. Terminal states (workflow outcome)

The workflow is "decode a single buffer". Its terminal states are:

| Terminal | Outcome type |
|---|---|
| Successful v1 envelope | `Ok(DecodedSlotWrittenExtra::Envelope(_))` |
| Successful legacy bytes | `Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_))` |
| Version mismatch | `Err(SlotWrittenExtraError::VersionMismatch { found })` |
| v1 corruption | `Err(SlotWrittenExtraError::DecodeFailed)` |

There are NO retries. The downstream `recovered_slot_taint` / `hydrate_slot_written_extra` propagates the failure (or its translation) and surfaces it to the recovery / runtime error lattice; the journal event is rejected and the recovery fails closed.

## 11. Timing and ordering constraints

- The decode MUST happen AFTER the journal event has been read into memory and BEFORE the slot taint is recorded into the accumulator. This ordering is enforced by the existing call graph; no ordering hazard added by this bead.
- The order of the two arms (LegacyFrameExtra first vs. magic-but-version-mismatch first) determines correctness. The discriminator uses `bytes[..4] == MAGIC` as the FIRST gate and `version != VERSION` as the SECOND gate; this ordering is preserved by the new body.
