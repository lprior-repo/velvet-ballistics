# vb-qi37.1.2 Codebase Map

## Bead
- **ID**: vb-qi37.1.2
- **Title**: runtime/recovery: Journal slot writes with taint
- **State**: 2 (Codebase Mapping)
- **Branch/Workspace**: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`

## Exploration Summary

This bead concerns the correct handling of taint metadata when journal slots are written during runtime execution and recovered via the recovery subsystem. The key flow is:

1. **Runtime execution** writes slots with taint via `Frame::write_slot_with_taint` or generated `write_slot_with_journal`
2. **Runtime journal** emits `RuntimeJournalEvent::SlotWritten { slot, value, taint, extra }`
3. **Storage journal** maps to `JournalEvent::SlotWrittenEvent { slot, value, extra }` where `extra` encodes taint via `encoded_slot_taint_extra`
4. **Recovery replay** decodes events and reconstructs frame state via `FrameSeedAccumulator::record_slot_write` -> `recovered_slot_taint`

## Relevant Crates and Files

### vb_core
| File | Symbols | Role |
|------|---------|------|
| `crates/vb_core/src/frame.rs` | `write_slot_with_taint` (line 229), `read_taint` (line 261), `write_taint` (line 317), `initialized_slots` (line 248) | Core slot write with taint API |
| `crates/vb_core/src/value.rs` | `Taint` enum (line 13), `join_taint` | Taint type definition |

### vb_runtime
| File | Symbols | Role |
|------|---------|------|
| `crates/vb_runtime/src/journal.rs` | `RuntimeJournalEvent`, `RuntimeJournal` trait | Journal module entry |
| `crates/vb_runtime/src/journal/chunk_001.rs` | `RuntimeJournalEvent::SlotWritten` (line 106-117), `VolatileRuntimeJournal`, `StorageRuntimeJournal` | SlotWritten event with taint field |
| `crates/vb_runtime/src/journal/chunk_002.rs` | `encoded_slot_taint_extra` (line 192-194), `SlotWrittenEvent` mapping | Encodes taint into extra via postcard |
| `crates/vb_runtime/src/taint.rs` | `ResolvedNodeTaintInput`, `resolved_node_output_taint` (line 64), `join_all` | Taint resolution for expressions |
| `crates/vb_runtime/src/recovery.rs` | `DurableFrameRecoveryBoundary`, `apply_recovered_slots` (line 100-106), `reject_unsupported_live_frame_state` (line 73) | Runtime recovery boundary |

### vb_storage
| File | Symbols | Role |
|------|---------|------|
| `crates/vb_storage/src/events.rs` | `JournalEvent::SlotWrittenEvent` (line 97-112) | Storage journal event |
| `crates/vb_storage/src/recovery/replay/summary.rs` | `FrameSeedAccumulator::record_slot_write` (line 389), `recovered_slot_taint` (line 428), `slot_taint: HashMap<SlotIdx, Taint>` (line 288) | Frame seed building |
| `crates/vb_storage/src/recovery/types.rs` | `RecoveredSlotEntry`, `RecoveryFrameSeed`, `UnsupportedRecoveryState` | Recovery type definitions |
| `crates/vb_storage/src/recovery/replay/core.rs` | `replay_events`, `recover_run_from_events` | Core replay engine |
| `crates/vb_storage/src/recovery/recover.rs` | `recover_run`, `RecoveryError` | Top-level recovery entry |
| `crates/vb_storage/src/codec.rs` | `encode_decode_roundtrip_journal_event_slot_written_with_value` | Codec roundtrip test |

### vb_codegen
| File | Symbols | Role |
|------|---------|------|
| `crates/vb_codegen/src/lib.rs` | `write_slot_with_journal` (line 2472), `GeneratedRunState`, `DriveOutput` | Generated runtime code |
| `crates/vb_codegen/tests/compile-fail/pass/minimal_workflow.rs` | `write_slot_with_journal` (line 165), `slot_taints`, `new_with_taints` (line 981) | Generated test workflow |

## Key Data Flow

```
Runtime execution:
  Frame::write_slot_with_taint(slot, value, taint)
      -> writes to self.slots[index] and self.taint[index]

Generated code:
  write_slot_with_journal(slot, value, taint)
      -> write_slot_with_taint(slots, slot_taints, slot, value, taint)
      -> journal.push(JournalEvent::SlotWritten { slot, value, taint })

Journal encoding (chunk_002.rs:146-159):
  RuntimeJournalEvent::SlotWritten { taint, extra }
      -> JournalEvent::SlotWrittenEvent { extra: encoded_slot_taint_extra(taint, extra) }
      where encoded_slot_taint_extra = postcard::to_allocvec(&taint).ok()

Recovery decoding (summary.rs:389-411):
  SlotWrittenEvent { slot, value, extra }
      -> record_slot_write(slot, value, extra)
      -> recovered_slot_taint(value, extra)
      -> extra.as_ref().map(|bytes| postcard::from_bytes::<Taint>(bytes))
         .unwrap_or_else(|| legacy_slot_taint(value))

Runtime recovery hydration (recovery.rs:100-105):
  seed.slots.iter().for_each(|entry| {
      frame.write_slot_with_taint(entry.slot, entry.value, entry.taint)
  })
```

## Risk Tags

| Risk | Description |
|------|-------------|
| `persistence` | JournalEvent::SlotWrittenEvent encodes taint in extra field via postcard. Loss/corruption of extra causes slot_taint_unsupported recovery state. |
| `recovery` | recovered_slot_taint falls back to legacy_slot_taint when extra is None. Legacy path infers taint from SlotValue type, may not preserve intended taint. |
| `taint` | write_slot_with_taint allows any Taint value. Taint propagation through expressions via resolved_node_output_taint must correctly join contributor taints. |
| `concurrency` | StorageRuntimeJournal and QueuedStorageRuntimeJournal use Mutex for next_seq_by_run. Journal append is serialized per-run. |

## Dependencies

- `vb_core -> vb_storage`: vb_core defines SlotValue, Taint, SlotIdx. vb_storage defines JournalEvent, recovery types.
- `vb_runtime -> vb_core`: vb_runtime/recovery.rs uses vb_core::frame::RunFrame and vb_core types.
- `vb_runtime -> vb_storage`: vb_runtime/journal maps RuntimeJournalEvent to JournalEvent.
- `vb_codegen -> vb_core`: Generated code uses vb_core types.

## Open Questions / Unknowns

- **UNKNOWN**: Whether legacy_slot_taint inference (summary.rs:435) correctly handles all SlotValue variants
- **UNKNOWN**: Whether encoded_slot_taint_extra handles all Taint variants correctly (postcard roundtrip)
- **MISSING**: No direct test coverage found for slot write taint roundtrip through full journal->recovery->hydration cycle

## Recommended Downstream Owners

- **contract**: rust-contract for function contracts on write_slot_with_taint, recovered_slot_taint, encoded_slot_taint_extra
- **proof**: kani for bounded model checking of slot write/recovery paths; verus for proof of taint invariants
- **test**: test-writer for BDD tests covering slot write -> journal -> recovery roundtrip with taint
- **impl**: functional-rust for implementation review of apply_recovered_slots and frame seed building
