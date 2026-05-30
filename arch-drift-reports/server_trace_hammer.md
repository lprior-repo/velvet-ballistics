# Architectural Drift Report: `vb_ipc/src/server/trace.rs`

**File**: `crates/vb_ipc/src/server/trace.rs`
**Status**: 🚨 VIOLATION — 672 lines (limit: 300)
**Enforcer**: architectural-drift

---

## Executive Summary

This file violates **TWO** hard limits:
1. **<300 line rule**: 672 lines — 124% over limit
2. **DDD cohesion**: Three distinct domain concerns crammed into one file

---

## Line Count Breakdown

| Region | Lines | Concern |
|--------|-------|---------|
| 1–11 | 11 | Module header + imports |
| 12–26 | 15 | `IpcResponseKind` + `count_response` |
| 28–63 | 36 | `typed_events_response` |
| 65–104 | 40 | `trace_event_kind` |
| 106–108 | 3 | `count_response_trace` wrapper |
| 111–150 | 40 | `handle_drain_trace` |
| 152–672 | **521** | Inline `mod tests` |

**Root cause**: 521 lines of tests are **inline** in the production module.

---

## DDD Violations

### 1. Primitive Obsession — `count: usize`

**Location**: `count_response(count: usize, ...)` (line 16)

```rust
fn count_response(count: usize, kind: IpcResponseKind) -> IpcResponse {
    match u32::try_from(count) {
        Ok(value) => match kind {
            IpcResponseKind::Trace => IpcResponse::TraceCount { count: value },
        },
        Err(_) => IpcResponse::CountOutOfRange { actual: count, limit: u32::MAX },
    }
}
```

**Problem**: `usize` is used for a domain concept ("trace event count"). This should be a NewType.

**Fix**: Create `TraceCount(u32)` or `EventCount(u64)` and make `CountOutOfRange` carry a domain-typed `actual` field.

---

### 2. Primitive Obsession — `max_records: u32` in Payload

**Location**: `IpcPayload::DrainTrace { run_id, max_records }` (payloads.rs:78)

`max_records` arrives as a raw `u32`. The handler converts it:

```rust
let max = match usize::try_from(max_records) {
    Ok(value) => value,
    Err(_) => usize::MAX,
};
```

**Problem**: `u32` is not a `MaxRecords` domain type. Conversion semantics are implicit.

---

### 3. Primitive Obsession — `from_sequence: u64`

**Location**: `typed_events_response(events: &[TraceEvent], from_sequence: u64)` (line 28)

**Problems**:
- Raw `u64` for a sequence cursor
- Manual index loop with repeated bounds checks instead of iterator combinators
- Sequence computation via `u64::try_from(index)` is repeated 3×

```rust
let Ok(sequence) = u64::try_from(index) else {
    return IpcResponse::CountOutOfRange { actual: index, limit: u32::MAX };
};
```

**Fix**: Replace with `.skip(from_sequence as usize).enumerate()` or use `.skip_while(...).take(...)`.

---

### 4. Primitive Obsession — Error Strings

**Location**: `handle_drain_trace` (lines 123–130)

```rust
return IpcResponse::RuntimeError {
    message: String::from("run not found"),
};
return IpcResponse::RuntimeError {
    message: String::from("unexpected inspect response"),
};
```

**Problem**: Hardcoded strings instead of typed domain error variants.

**Fix**: Add `RunNotFound(RunId)` and `UnexpectedInspectResponse` variants to a domain error enum, then map at the IPC boundary.

---

### 5. Workflow Violation — Manual Indexed Loop

**Location**: `typed_events_response` (lines 30–59)

```rust
let mut index = 0usize;
while index < events.len() {
    let Ok(sequence) = u64::try_from(index) else { ... };
    if sequence >= from_sequence {
        let Some(event) = events.get(index) else { ... };
        typed_events.push(IpcTraceEvent { sequence, kind: trace_event_kind(event) });
    }
    index = match index.checked_add(1) { ... };
}
```

**Problem**: This is a classic "imperative loop" when a functional pipeline would be clearer and safer.

**Fix**:
```rust
typed_events_response(events: &[TraceEvent], from_sequence: u64) -> IpcResponse {
    let start = from_sequence as usize;
    let typed: Vec<_> = events
        .iter()
        .skip(start)
        .enumerate()
        .map(|(i, e)| IpcTraceEvent { sequence: i as u64, kind: trace_event_kind(e) })
        .collect();
    IpcResponse::Events { events: typed }
}
```

---

## Required Refactoring Plan

### Split into 3 files

| New File | Contents | Target Lines |
|----------|----------|--------------|
| `trace/count.rs` | `IpcResponseKind`, `count_response`, `count_response_trace` | ~25 |
| `trace/mapping.rs` | `trace_event_kind`, `typed_events_response` | ~60 |
| `trace/handlers.rs` | `handle_drain_trace` | ~45 |
| `trace/mod.rs` | Module declaration + re-exports | ~15 |
| `trace/tests.rs` | All tests extracted from inline `mod tests` | ~520 |

**Total after split**: ~665 lines, but now properly separated by concern.

---

## Critical Rule Violations

| Rule | Status | Detail |
|------|--------|--------|
| <300 line files | 🚨 FAIL | 672 lines |
| No `unwrap`/`expect`/`panic` | ✅ PASS | No panic-inducing code |
| No `unsafe` | ✅ PASS | `#![forbid(unsafe_code)]` |
| Parse, don't validate | ⚠️ PARTIAL | Conversion is `try_from` with fallback |
| Primitive obsession | 🚨 FAIL | `usize`, `u32`, `u64` for domain concepts |
| State modeled as functions | ✅ N/A | Not a state machine file |

---

## Scott Wlaschin DDD Checklist

- [x] **Newtypes for quantities**: FAIL — `count: usize`, `max_records: u32`, `from_sequence: u64`
- [x] **No primitive fields in domain types**: FAIL — `IpcResponse::TraceCount { count: u32 }` uses raw `u32`
- [x] **Explicit error domains**: FAIL — `String` error messages instead of typed variants
- [x] **Workflow as state transitions**: N/A — this is a helper module
- [x] **Types make illegal states unrepresentable**: PARTIAL — `CountOutOfRange` exists but carries raw `usize`

---

## Recommended Refactoring Order

1. **Extract tests first** — move `mod tests` (lines 152–672) to `trace/tests.rs`
2. **Create `TraceCount` NewType** — wrap `u32` for event counts
3. **Create `SequenceCursor` NewType** — wrap `u64` for `from_sequence`
4. **Replace manual loop** in `typed_events_response` with iterator combinators
5. **Extract `handle_drain_trace`** to `trace/handlers.rs`
6. **Extract `count_response`** to `trace/count.rs`
7. **Extract `trace_event_kind`** + `typed_events_response` to `trace/mapping.rs`
8. **Replace string errors** with typed domain error enum variants in `IpcResponse`

---

**STATUS: REFACTOR REQUIRED** — File exceeds 300-line limit by 124%. Test extraction alone reduces it to ~152 lines. Additional NewType refactoring will further improve DDD compliance.
