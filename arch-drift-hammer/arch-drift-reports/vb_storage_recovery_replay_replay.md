# Architectural Drift Report: `vb_storage/src/journal/replay.rs`

**File**: `crates/vb_storage/src/journal/replay.rs`  
**Total Lines**: 157  
**Threshold**: 300  
**Status**: ✅ PASS (under 300 lines)

---

## DDD Cohesion Analysis

### Purpose
This module handles **event replay** for runs — collecting journal events for a specific run in contiguous sequence order.

### What the Module Owns
| Concept | Type | Location |
|---------|------|----------|
| `ReplayPushLimitDecision` | Value Object (enum) | Line 14 |
| `classify_replay_push_len` | Domain Function | Line 30 |
| `events_for_run*` | FjallJournal Methods | Lines 53-119 |
| `validate_replay_sequence` | Domain Function | Line 122 |
| `push_replay_event` | Domain Function | Line 133 |

### DDD Smells

#### 1. **Infrastructure Leakage** (Medium Severity)
The replay domain logic directly calls codec internals:

```rust
// Line 109 - codec.decode_record leaks into domain
decode_record(value.as_ref(), MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;

// Line 128 - codec.validate_replayed_event leaks into domain  
crate::codec::validate_replayed_event(run, expected_seq, event)?;

// Line 79, 129 - next_seq is codec concern
crate::codec::next_seq(seq)?
```

**Smell**: `ReplaySequenceValidator` or a replay-specific `ReplayEvent` wrapper should encapsulate these codec interactions. The `events_for_run_from` method is performing codec+domain+marshaling orchestration — this violates Single Responsibility.

#### 2. **Primitive Obsession in Domain Functions** (Low Severity)
```rust
// Line 31: raw usize instead of domain type
pub(crate) fn classify_replay_push_len(
    current_len: usize,   // <-- primitive
    limit: EventReplayLimit,
) -> ReplayPushLimitDecision
```

**Smell**: `current_len` should be `ReplayEventCount` (NewType). The `EventReplayLimit` already wraps `usize` but the input is unwrapped.

#### 3. **Key Construction is Exposed** (Low Severity)
```rust
// Lines 97-98: Key construction in domain method
let start_key = run_event_key(run, start_seq)?;
let run_prefix = run_prefix_key(run)?;
```

**Smell**: `run_event_key` and `run_prefix_key` from `keys.rs` are public infrastructure functions exposed to domain logic. A `RunKeySpace` or `JournalKeySpace` abstraction would encapsulate this.

#### 4. **No Replay Aggregate/Entity** (Medium Severity)
The replay state machine (`expected` sequence tracking in `events_for_run_from`) is implemented via mutable state in a standalone function:

```rust
// Line 96: Mutable state passed through
let mut expected = Some(first_event);
// ...
validate_replay_sequence(run, &mut expected, &event)?;
```

**Smell**: This should be a `ReplayState` entity or `ReplayIterator` that encapsulates the sequence validation state machine, not mutable locals passed to helper functions.

---

## Violations Summary

| # | Violation | Severity | Type |
|---|-----------|----------|------|
| 1 | Codec internals (`decode_record`, `validate_replayed_event`, `next_seq`) called directly from replay domain | Medium | Cross-cutting Concern Leak |
| 2 | `current_len: usize` primitive instead of domain type | Low | Primitive Obsession |
| 3 | Exposed `run_event_key`/`run_prefix_key` infrastructure to domain layer | Low | Infrastructure Leak |
| 4 | Replay sequence state machine as mutable locals instead of encapsulated entity | Medium | Anemic Domain Model |

---

## Recommendation Priority

| Priority | Action | Complexity |
|----------|--------|------------|
| **P2** | Create `ReplayEvent` wrapper that hides codec decoding | Medium |
| **P2** | Encapsulate sequence state into `ReplayIterator` entity | Medium |
| **P3** | Wrap `current_len` in `ReplayEventCount` NewType | Low |
| **P3** | Create `JournalKeySpace` to hide `run_event_key`/`run_prefix_key` | Low |

---

## Verdict

**DDD Smell**: MODERATE  
**Priority**: P2-P3 (medium-term refactor)  
**Lines**: ✅ 157/300  
**Status**: `PERFECT` (no blocking issues, under threshold)

The module is well-structured for 157 lines and demonstrates good functional decomposition (`classify_replay_push_len` is a pure, testable domain function). The primary issue is that codec concerns leak into the replay workflow — this is a common pattern in storage layers but violates strict DDD boundaries. A `ReplayRun` aggregate or `ReplayIterator` entity would tighten the cohesion.