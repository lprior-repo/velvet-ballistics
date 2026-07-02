# Boundary Map — vb-5bqmr

**Bead**: vb-5bqmr — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)

This bead touches three boundaries:

1. **Codec boundary** (`crates/vb_storage/src/slot_extra.rs`): pure decode of a `&[u8]` host-buffer into either a typed envelope, an opaque legacy slice, or a typed error.
2. **Recovery boundary** (`crates/vb_storage/src/recovery/replay/summary/hydrate.rs:220`): the recovery hydrator translates the codec error into the recovery-error lattice.
3. **Runtime boundary** (`crates/vb_runtime/src/primitives/collect.rs:248`): the journal-driven collect-state hydrator translates the codec error into the runtime-error lattice.

There is NO new boundary introduced. All three sites already exist; this bead refines the discrimination performed at each site.

## 1. Boundary diagram

```
                                ┌───────────────────────────────────────┐
                                │ EXTERNAL: durable journal event      │
                                │  SlotWrittenEvent { extra: Vec<u8> }  │
                                └─────────────────┬─────────────────────┘
                                                  │
                                                  ▼
        ╔═══════════════════════════════════════════════════════════════╗
        ║  BOUNDARY 1: CODEC PARSE — vb_storage::slot_extra             ║
        ║  ─ in:  &bytes (borrowed from the durable event)              ║
        ║  ─ out: Result<DecodedSlotWrittenExtra<'_>,                   ║
        ║         SlotWrittenExtraError>                                ║
        ║  ─ actor: sync, pure, total, deterministic                     ║
        ║  ─ back-pressure: none; one byte slice, one Result              ║
        ║  ─ failure modes:                                              ║
        ║    • DecodeFailed   (v1 prefix + bad postcard)                  ║
        ║    • VersionMismatch{ found } (magic + unrecognised version)    ║
        ║  ─ forbidden side effects: allocation on legacy path            ║
        ╚════════════════════╤══════════════════════════════════════════╝
                             │
                             │  Result<DecodedSlotWrittenExtra, SlotWrittenExtraError>
                             │
        ┌────────────────────┴───────────────────────┐
        │                                            │
        ▼                                            ▼
   Recovery caller                            Runtime caller
   (hydrate.rs:220)                          (collect.rs:248)
        │                                            │
        ▼                                            ▼
   ╔════════════════════════════════════╗    ╔════════════════════════════════════╗
   ║  BOUNDARY 2: RECOVERY ERROR        ║    ║  BOUNDARY 3: RUNTIME ERROR         ║
   ║  TRANSLATION                       ║    ║  TRANSLATION                       ║
   ║  ─ in:  codec Result + slot ctx    ║    ║  ─ in:  codec Result + slot ctx   ║
   ║  ─ out: RecoveryResult<            ║    ║  ─ out: Result<(), EngineError>   ║
   ║         RecoveredSlotTaint>        ║    ║  ─ failure modes:                 ║
   ║       or RecoveryError::           ║    ║    • CollectExtraHydration-       ║
   ║         CorruptSlotTaint{slot}     ║    ║      Failed{kind=Version-         ║
   ║  ─ actor: sync, pure translation   ║    ║      Mismatch, ...}   (NEW)       ║
   ║    with tracing::warn! side-       ║    ║    • CollectExtraHydration-       ║
   ║    effect at log level             ║    ║      Failed{kind=DecodeFailed,...}║
   ║  ─ forbidden: collapsing           ║    ║  ─ actor: sync, pure translation  ║
   ║    VersionMismatch into "ok" or    ║    ║    with tracing::warn! log        ║
   ║    LegacyFrameExtra                ║    ║  ─ forbidden: collapsing          ║
   ╚════════════════════════════════════╝    ║    VersionMismatch into ok or     ║
                                            ║    LegacyFrameExtra               ║
                                            ╚════════════════════════════════════╝
```

## 2. Pure core vs imperative shell split

| Layer | Pure or impure | Why |
|---|---|---|
| `decode_slot_written_extra` (new body) | **PURE** | borrows the input, returns a `Result`. No I/O, no time, no async, no allocation on the legacy path; v1 decode uses postcard's allocation strategy internally but is otherwise pure. |
| `encode_slot_written_extra` | pure (one `Vec<u8>` output) | unchanged |
| `decoded_slot_taint` (hydrate.rs:220) | pure + `tracing::warn!` side-effect | the side-effect is a log; the function remains deterministic |
| `hydrate_slot_written_extra` (collect.rs:248) | mutates self (state-bearing), pure translation of the codec error | the codec branch in this function is pure (the mutation is on surrounding state, not on the codec translation itself) |

## 3. Boundary 1 — Codec parse (`slot_extra.rs`)

### Inputs and outputs

| Channel | Direction | Type | Notes |
|---|---|---|---|
| `bytes: &[u8]` | in | borrowed lifetime tied to caller | the journal bytes |
| `→ DecodedSlotWrittenExtra<'a>` | out | borrowed `'a = lifetime of bytes` | preserves the legacy slice borrow |

### Invariants crossing IN

- `bytes` is a sub-range of a `SlotWrittenEvent.extra` field; the parser does not need to know how it was persisted.

### Invariants crossing OUT

- A `DecodedSlotWrittenExtra` borrowed variant holds a `&'a [u8]` referencing the input; the caller MUST NOT free or mutate the source buffer while the borrow is alive.
- An `Err` variant has a `Copy` payload (`u8` for `found`); no buffers escape the boundary.

### Failure surface

| Failure | Bound condition |
|---|---|
| `EncodeFailed` | encoder-only; never crosses the decoder boundary |
| `AllocationFailed` | encoder-only; never crosses the decoder boundary |
| `DecodeFailed` | v1 prefix present + postcard fails |
| `VersionMismatch { found }` | magic present + unknown version byte |

### Forbidden crossings

- No allocation on the legacy path; an attempt to round-trip a legacy slice into an owned buffer at this layer would be wrong.
- No I/O, no time, no randomness, no FFI, no `unsafe`.

## 4. Boundary 2 — Recovery translation (`hydrate.rs:220`)

### Inputs and outputs

| Channel | Direction | Type | Notes |
|---|---|---|---|
| `slot: SlotIdx` | in | typed newtype | from the journal event header |
| `value: SlotValue` | in | typed | from the journal event payload |
| `bytes: &[u8]` | in | borrowed from `extra: &Option<Vec<u8>>` | the codec input |
| `→ RecoveryResult<RecoveredSlotTaint>` | out | `Result<_, RecoveryError>` | typed lattice |

### Translation table (after this bead)

| Codec outcome | Translation | Logging |
|---|---|---|
| `Ok(Envelope(env))` | `RecoveredSlotTaint { taint: env.taint, unsupported: false }` | none |
| `Ok(LegacyFrameExtra(_))` | `RecoveredSlotTaint { taint: Taint::Secret, unsupported: true }` (lattice-preserving) | none |
| `Err(VersionMismatch { found })` | `Err(RecoveryError::CorruptSlotTaint { slot })` | `tracing::warn!(slot, found, ...)` |
| `Err(DecodeFailed)` | `Err(RecoveryError::CorruptSlotTaint { slot })` | none |
| `Err(EncodeFailed)` / `Err(AllocationFailed)` | `Err(RecoveryError::CorruptSlotTaint { slot })` | none (defensive arms; unreachable today on the decode path) |

### Forbidden crossings

- The translation MUST be exhaustive on `SlotWrittenExtraError`. A catch-all `Err(_)` arm is forbidden for this site (the new variant `VersionMismatch` would otherwise be conflated).
- The `RecoveryError` enum is `#[non_exhaustive]`; this translation does NOT add a new variant to it.
- `tracing::warn!` is the only side-effect of the `VersionMismatch` arm; it does not mutate the recovery state.

## 5. Boundary 3 — Runtime translation (`collect.rs:248`)

### Inputs and outputs

| Channel | Direction | Type | Notes |
|---|---|---|---|
| `run: RunId` | in | typed newtype | from the journal event |
| `slot: SlotIdx` | in | typed newtype | from the journal event |
| `seq: vb_storage::EventSeq` | in | typed newtype | from the journal event |
| `value: Option<&[u8]>` | in | optional borrowed slice | the slot value |
| `extra: &[u8]` | in | borrowed from the journal event | the codec input |
| `→ Result<(), EngineError>` | out | typed lattice | mutates `self.collect_states` on success |

### Translation table (after this bead)

| Codec outcome | Translation | Logging |
|---|---|---|
| `Ok(Envelope(env))` (frame_extra = Some) | `self.hydrate_frame_extra(run, slot, seq, value, &env.frame_extra)` | none |
| `Ok(Envelope(env))` (frame_extra = None) | `Ok(())` | none |
| `Ok(LegacyFrameExtra(bytes))` | `self.hydrate_frame_extra(run, slot, seq, value, bytes)` | none |
| `Err(VersionMismatch { found })` | `Err(EngineError::CollectExtraHydrationFailed { kind: VersionMismatch, run_id, collector_slot, event_seq })` | `tracing::warn!(slot, seq, found, ...)` |
| `Err(DecodeFailed)` | `Err(EngineError::CollectExtraHydrationFailed { kind: DecodeFailed, run_id, collector_slot, event_seq })` | none |
| `Err(EncodeFailed)` / `Err(AllocationFailed)` | `Err(EngineError::CollectExtraHydrationFailed { kind: DecodeFailed, run_id, collector_slot, event_seq })` | none (defensive) |

### Forbidden crossings

- The translation MUST be exhaustive on `SlotWrittenExtraError`. A catch-all `Err(_)` arm is forbidden for this site.
- `CollectExtraHydrationFailureKind::VersionMismatch` is the new discriminator arm; the caller's `match` must include it.

## 6. Storage boundary (orthogonal, confirmed)

The decoder does NOT touch the journal keyspace; it operates purely on bytes already loaded into memory by the surrounding hydration loops. The encoder writes bytes into the journal via `vb_runtime::journal::chunk_002::encoded_slot_taint_extra` (line 326), which is unchanged. No new Fjall / record / index boundary in this bead.

## 7. Async / concurrency boundaries

None. `decode_slot_written_extra` and its translation sites are synchronous and total. No tokio / async-std / futures traits involved at this layer.

## 8. Time / I/O / FFI / unsafe boundaries

None of these cross through the codec layer:

- No `std::time`, `Instant`, `Duration`.
- No filesystem, network, IPC.
- No FFI.
- No `unsafe` (the file head is `#![forbid(unsafe_code)]`).

## 9. Boundary verification (what each boundary test must assert)

| Boundary | Verification |
|---|---|
| Codec (parse) | `decode_slot_written_extra(b"VBSE\x02\x01\x02")` returns `Err(VersionMismatch{ found: 0x02 })`. |
| Codec (preservation) | `decode_slot_written_extra(b"\x01\x02\x03\x04")` returns `Ok(LegacyFrameExtra(b"\x01\x02\x03\x04"))`. |
| Codec (v1 happy path) | `decode_slot_written_extra(&encode_slot_written_extra(...)?)` is the identity modulo postcard round-trip. |
| Codec (v1 corruption) | `decode_slot_written_extra(b"VBSE\x01\xff\xff\xff")` returns `Err(DecodeFailed)`. |
| Recovery translation | Recovery unit test asserts that `decoded_slot_taint(slot, value, b"VBSE\x02...")` returns `Err(CorruptSlotTaint{ slot })` and emits a `tracing::warn!` carrying the `found` byte. |
| Runtime translation | Collect unit test asserts that `hydrate_slot_written_extra(... b"VBSE\x02...")` returns `Err(CollectExtraHydrationFailed{ kind: VersionMismatch, .. })`. |
| Exhaustiveness | `cargo build --all-targets` succeeds because both translation sites use explicit `match` arms (no `_ =>` fallthrough on the codec error). |

## 10. Boundary-map summary

| Boundary | Touched | Direction | Side-effect | New failure variant |
|---|---|---|---|---|
| Codec parse | YES (new logic) | in → out | none | `VersionMismatch { found }` |
| Recovery translation | YES (new arm) | in → out | `tracing::warn!` | none (reuses `CorruptSlotTaint`) |
| Runtime translation | YES (new arm + new enum variant) | in → out | `tracing::warn!` | `CollectExtraHydrationFailureKind::VersionMismatch` |
| Storage | NO | n/a | n/a | n/a |
| Async | NO | n/a | n/a | n/a |
| Time / IO / FFI / unsafe | NO | n/a | n/a | n/a |
