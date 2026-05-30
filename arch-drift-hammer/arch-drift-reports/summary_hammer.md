## Architectural Drift Report: summary.rs

**File:** `crates/vb_storage/src/recovery/replay/summary.rs`
**Total Lines:** 1576
**Violation:** 1576 > 300 (OVER BY 1276 LINES / 425% OVER LIMIT)

---

### Line Count Breakdown

| Section | Lines | Description |
|---------|-------|-------------|
| Public API + impl | 27-272 | Builder pattern and public entry points |
| Frame seed inner logic | 273-410 | Inner functions and accumulator type |
| Accumulator impl | 424-686 | 262-line impl block with 15+ methods |
| Slot taint recovery | 688-754 | 66 lines for taint reconstruction |
| RecoveryIndex trait | 756-780 | 25 lines |
| Dimension helpers | 782-810 | 29 lines |
| Slot recovery | 827-954 | 127 lines for RecoveredSlots |
| Error mapping | 956-985 | 30 lines |
| Tests | 987-1576 | 590 lines (59% of file) |

---

### Functions Requiring Extraction

| Function | Lines | Responsibility | Extraction Target |
|----------|-------|----------------|-------------------|
| `apply_summary_event` | 27-81 | Applies event effects to summary counters | `summary_counters.rs` |
| `recover_run_admission_from_events` | 85-101 | Finds latest admission metadata | `admission_recovery.rs` |
| `summarize_recovery_events` | 104-145 | Summary-only hydration | `summary_hydration.rs` |
| `apply_summary_event_checked` | 147-221 | Checked event application with tracker | `summary_validation.rs` |
| `reject_resolved_summary_action` | 223-232 | Idempotency guard | `summary_validation.rs` |
| `RecoveryFrameSeedBuilder` | 239-272 | Builder struct + impl | `frame_seed_builder.rs` |
| `recover_runtime_frame_seed_from_events*` | 278-311 | Frame seed entry points | `frame_seed_recovery.rs` |
| `recover_runtime_frame_seed_from_events_inner` | 313-323 | Inner frame seed logic | `frame_seed_recovery.rs` |
| `recover_frame_seed_accumulator` | 325-334 | Fold-based accumulator | `frame_seed_accumulator.rs` |
| `build_recovery_frame_seed` | 336-360 | Final seed construction | `frame_seed_accumulator.rs` |
| `seed_unsupported_state` | 362-392 | Unsupported state analysis | `frame_seed_accumulator.rs` |
| `FrameSeedAccumulator` impl | 424-686 | **262-line god impl** — split into: |
| `apply` | 456-481 | Event routing | `frame_seed_accumulator.rs` |
| `apply_frame_event` | 483-541 | Step/action/slot events | `frame_event_handler.rs` |
| `record_step` | 544-550 | Step state tracking | `step_state.rs` |
| `record_slot_write` | 562-585 | Slot write with decode | `slot_write.rs` |
| `record_action_completion_envelope` | 587-619 | Envelope processing | `envelope_handler.rs` |
| `record_envelope_slot` | 621-639 | Envelope slot recording | `envelope_handler.rs` |
| `record_action_*` methods | 641-681 | Action tracking | `action_tracking.rs` |
| `first_step` | 683-685 | First step accessor | `step_state.rs` |
| `recovered_slot_taint` family | 694-742 | Taint reconstruction | `slot_taint.rs` |
| `max_step/min_step/max_slot` | 744-754 | Option helpers | `index_helpers.rs` |
| `RecoveryIndex` trait | 756-770 | Polymorphic index | `index_helpers.rs` |
| `dimension_count` | 772-780 | Dimension overflow check | `index_helpers.rs` |
| `recovered_steps` | 811-816 | Step map → vec | `step_state.rs` |
| `recovered_pending_actions` | 818-825 | Pending action conversion | `action_tracking.rs` |
| `RecoveredSlots` impl | 827-954 | Slot recovery logic | `slot_recovery.rs` |
| `recover_slots` | 833-848 | Slot recovery entry | `slot_recovery.rs` |
| `merge_recovered_slots` | 850-865 | Slot override merge | `slot_recovery.rs` |
| `recovered_event_slots` | 867-881 | Event-based slot recovery | `slot_recovery.rs` |
| `recover_slots_through_step` | 883-901 | Workflow-based slot recovery | `slot_recovery.rs` |
| `initialized_recovered_slots` | 903-916 | Frame slot initialization | `slot_recovery.rs` |
| `replay_error_to_recovery` | 956-985 | Error conversion | `replay_error_map.rs` |
| Tests | 987-1576 | 590 lines | Keep in respective modules |

---

### Primitive Obsession Violations

| Location | Primitive | Missing Type |
|----------|-----------|--------------|
| `FrameSeedAccumulator::slot_values` (line 399) | `HashMap<SlotIdx, SlotValue>` | `SlotValueMap` wrapper |
| `FrameSeedAccumulator::slot_taint` (line 400) | `HashMap<SlotIdx, Taint>` | `SlotTaintMap` wrapper |
| `FrameSeedAccumulator::step_states` (line 398) | `HashMap<StepIdx, RecoveredStepState>` | `StepStateMap` wrapper |
| `FrameSeedAccumulator::pending_actions` (line 401) | `HashSet<(ActionId, StepIdx)>` | `PendingActionSet` wrapper |
| `ActionEnvelopeView::value` (line 418) | `&[u8]` | `EncodedValue` newtype |
| `record_envelope_slot::value` (line 624) | `&[u8]` | `EncodedValue` newtype |
| `ActionEnvelopeView::encoded_len` (line 419) | `u32` | `EncodedLen(u32)` wrapper |
| `ActionEnvelopeView::value_digest` (line 421) | `[u8; 32]` | `ValueDigest([u8; 32])` wrapper |
| `record_slot_write::value` (line 565) | `&Option<Vec<u8>>` | `SlotValueBytes(Option<Vec<u8>>)` |
| `record_slot_write::extra` (line 566) | `&Option<Vec<u8>>` | `SlotExtra(Option<Vec<u8>>)` |
| `FrameSeedAccumulator::pc` (line 406) | `StepIdx` | Already wrapped ✓ |
| `max_step/min_step/max_slot` | `Option<T>` | `IndexedDimension<T>` wrapper |

---

### Recommended Split (7 Target Files)

```
recovery/replay/
├── summary.rs                    # 300 lines: Re-exports + thin module glue
├── summary_counters.rs           # 81 lines: apply_summary_event
├── admission_recovery.rs         # 17 lines: recover_run_admission_from_events  
├── summary_hydration.rs         # 42 lines: summarize_recovery_events + validation
├── frame_seed_builder.rs         # 34 lines: RecoveryFrameSeedBuilder
├── frame_seed_recovery.rs        # 39 lines: public entry points
├── frame_seed_accumulator.rs     # 120 lines: accumulator type + core impl
├── frame_event_handler.rs        # 59 lines: apply_frame_event match
├── slot_recovery.rs             # 122 lines: RecoveredSlots + slot logic
├── slot_taint.rs                # 49 lines: taint reconstruction
├── action_tracking.rs           # 41 lines: action recording methods
├── step_state.rs                # 35 lines: step recording + accessors
├── envelope_handler.rs          # 53 lines: envelope processing
├── slot_write.rs               # 24 lines: slot write recording
├── index_helpers.rs            # 40 lines: RecoveryIndex + dimension helpers
├── replay_error_map.rs         # 30 lines: replay_error_to_recovery
└── summary_tests.rs            # 590 lines: tests (or inline with #[cfg(test)])
```

---

### DDD Bounded Contexts Identified

1. **Runtime Summary Context** — Counter accumulation, event→summary mapping
2. **Frame Seed Context** — Step/slot state reconstruction, dimension tracking
3. **Slot Recovery Context** — Value decode, taint inference, workflow replay
4. **Action Tracking Context** — Pending/resolved action idempotency guards
5. **Error Mapping Context** — ReplayError → RecoveryError translation

---

### Immediate Actions Required

1. Extract `FrameSeedAccumulator` impl (262 lines) into separate file
2. Create `SlotValueMap`/`SlotTaintMap` wrappers for HashMap primitives
3. Create `EncodedValue`/`EncodedLen`/`ValueDigest` newtypes for byte primitives
4. Split tests into their own file(s) — 590 lines is 37% of file
5. Target: 7 files ≤300 lines each
