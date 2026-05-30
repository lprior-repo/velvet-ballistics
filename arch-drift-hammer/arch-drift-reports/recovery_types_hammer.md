# Architectural Drift Report: `recovery/types.rs`

**File**: `crates/vb_storage/src/recovery/types.rs`
**Line Count**: 606 (🔴 VIOLATION: exceeds 300 line limit by 102%)
**Workspace**: `arch-drift-hammer`

---

## Executive Summary

This file is a **catastrophic structural violation**. At 606 lines, it is 2x the permitted size. It violates:
- [x] **<300 Line Rule**: 606 lines is 102% over limit
- [x] **Single Responsibility Principle**: Mixes 4+ distinct domains
- [x] **Primitive Obsession**: Raw `[u8; 32]`, `u16`, `u32`, `Vec<u8>` scattered throughout
- [x] **DDD Boundary Violations**: Error types, recovery types, replay tracking, and digest checking are all co-mingled

---

## Primary Violation: Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 606 | 300 | 🔴 OVER BY 102% |
| Production Types | ~400 | - | - |
| Implementation Code | ~206 | 0 | 🔴 BELONGS IN .rs FILES |

**Verdict**: File MUST be split. Minimum 2 files required.

---

## Primitive Obsession Violations

### 1. `value_digest: [u8; 32]` (line 395)
**Location**: `ActionCompletionEvidence`
**Problem**: Raw 32-byte array for a digest. No type safety.
**Fix**: Introduce `DigestBytes([u8; 32])` or use existing `WorkflowDigest` wrapper if appropriate.

### 2. `encoded_len: u32` (line 393)
**Location**: `ActionCompletionEvidence`
**Problem**: Raw `u32` with no domain meaning. Could be length, offset, count.
**Fix**: Wrap in `EncodedLength(u32)` or `ByteLength(u32)`.

### 3. `step_count: u16`, `slot_count: u16` (lines 342-344)
**Location**: `RecoveryFrameSeed`
**Problem**: Raw `u16` for counts that should be non-zero.
**Fix**: Use `Count<u16>` or `NonZeroU16` with a domain-specific wrapper.

### 4. `Vec<u8>` for binary blobs (lines 367, 369)
**Location**: `RunSnapshot.slots` and `RunSnapshot.taint`
**Problem**: `Vec<u8>` is untyped bytes. No indication of encoding format.
**Fix**: Introduce `SlotBinary(Vec<u8>)` and `TaintBinary(Vec<u8>)` or a proper codec type.

### 5. `hierarchy_rank` returns raw `u8` (lines 575-580)
**Location**: `DigestCheck::hierarchy_rank`
**Problem**: `u8` is a primitive. The semantic meaning is "strictness level".
**Fix**: Return a domain type `DigestCheckLevel(u8)` or better yet, use an ordered enum with explicit rank.

---

## DDD Boundary Violations

### Domain Confusion: One File Contains Four Bounded Contexts

| Lines | Domain | Bounded Context |
|-------|--------|------------------|
| 37-129 | Error Taxonomy | `RecoveryError` belongs in `recovery/errors.rs` |
| 135-176 | Runtime Summary | `RecoveryRuntimeSummary` belongs in `recovery/summary.rs` |
| 180-189 | Admission | `RecoveredRunAdmission` belongs in `recovery/admission.rs` |
| 193-211 | Hydration | `RecoveryHydration` belongs in `recovery/hydration.rs` |
| 213-355 | Frame Seed | `RecoveryFrameSeed`, `RecoveredStepState`, etc. belong in `recovery/frame_seed.rs` |
| 357-370 | Snapshot | `RunSnapshot` belongs in `recovery/snapshot.rs` |
| 372-558 | Replay Tracking | `ActionReplayTracker` belongs in `recovery/replay.rs` |
| 560-606 | Digest Checking | `DigestCheck` belongs in `recovery/digest.rs` |

### `ActionReplayTracker` is a Large Blob (lines 372-558)

This struct handles:
1. Tracking scheduled action tickets
2. Tracking completed actions
3. Tracking failed actions
4. Detecting replay divergence
5. Blocking non-idempotent action re-execution

**Problem**: 186 lines of complex stateful logic in a "types.rs" file. This is a **Service** in DDD terms, not a type definition.

**Fix**: Extract to `recovery/replay_tracker.rs`.

---

## Scott Wlaschin DDD Violations

### 1. "Primitive Obsession" - Raw Integers Everywhere
- `u16` for counts without validation
- `u32` for encoded lengths
- `[u8; 32]` for digests
- `Vec<u8>` for binary data

### 2. "Type Reveals Domain" - Names Don't Tell Stories
- `RecoveryError` variant `FrameDimensionOverflow` contains `run: RunId` but no frame dimensions
- `UnsupportedRecoveryState` is a flags struct - could be a Set type

### 3. "Make Illegal States Unrepresentable"
- `step_count: u16` can be 0 but logically should be ≥1
- `RecoveryFrameSeed::steps` is `Vec` but semantically should be a set with known cardinality

### 4. "Command-Query Separation"
- `ActionReplayTracker` methods like `mark_completed` and `has_completed` are mixed
- Should separate "commands that mutate" from "queries that observe"

---

## Mandatory Refactoring Plan

### Phase 1: File Split (Mandatory)
```
recovery/
├── mod.rs          (~20 lines - re-exports)
├── types.rs        (~150 lines - core enums/structs ONLY)
├── errors.rs       (~100 lines - RecoveryError + RecoveryResult)
├── summary.rs      (~50 lines - RecoveryRuntimeSummary, RecoveredRunAdmission)
├── hydration.rs    (~30 lines - RecoveryHydration)
├── frame_seed.rs   (~120 lines - RecoveryFrameSeed + entry types)
├── snapshot.rs     (~30 lines - RunSnapshot)
├── replay.rs       (~180 lines - ActionReplayTracker)
└── digest.rs       (~50 lines - DigestCheck)
```

### Phase 2: Value Object Wrappers
```rust
// In types.rs or new value_objects.rs
pub struct DigestBytes([u8; 32]);
pub struct EncodedLength(u32);
pub struct StepCount(u16);
pub struct SlotCount(u16);
pub struct SlotBinary(Vec<u8>);
pub struct TaintBinary(Vec<u8>);
```

### Phase 3: UnsupportedRecoveryState Redesign
Convert from flags struct to a proper Set type or use a bitflags-like pattern with domain methods.

---

## Evidence

```
File: crates/vb_storage/src/recovery/types.rs
Lines: 606
Limit: 300
Overflow: 306 lines (102% over)
```

---

## Verdict

🔴 **REJECTED** - This file is architecturally bankrupt.

**Reasons**:
1. Double the allowed line count
2. Four+ bounded contexts jammed into one file
3. Primitive obsession throughout
4. `ActionReplayTracker` (186 lines) is a full domain service hiding in a "types" file
5. No evidence of domain modeling - just data bags with derive macros

**Required Actions**:
1. Split into minimum 5 files
2. Introduce value object wrappers for all primitives
3. Extract `ActionReplayTracker` to its own module
4. Apply `#[companion_rule]` or equivalent if companion types exist
5. Re-run architectural drift check after refactoring

---

*Report generated by arch-drift-hammer*
*Drift enforcement: ZERO TOLERANCE*
