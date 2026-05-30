# Architectural Drift Report: `payloads.rs`

**File:** `crates/vb_ipc/src/payloads.rs`  
**Line Count:** 575 (LIMIT: 300)  
**Excess:** 275 lines (192% of limit)  
**Status:** 🔴 CRITICAL VIOLATION

---

## 1. LINE COUNT VIOLATION

| Metric | Value |
|--------|-------|
| Actual Lines | 575 |
| Max Allowed | 300 |
| Excess | 275 |
| Violation % | 192% over limit |

---

## 2. PAYLOAD TYPE MAP

| Type | Lines | Purpose |
|------|-------|---------|
| `SubmitRunPayload` | 12-19 | Workflow run submission input |
| `IpcPayload` | 24-112 | Main IPC command enum (14 variants) |
| `RunListState` | 117-126 | Run lifecycle state enum |
| `RunSummary` | 130-145 | List-run response summary |
| `VerificationResult` | 149-158 | Workflow verification outcome |
| `GateKind` | 163-182 | Verification gate type enum (9 gates) |
| `ParseGateKindError` | 185-187 | Parse error sentinel |
| `PassFail` | 230-233 | Binary status marker |
| `TaintPathStatus` | 236-241 | Taint severity level |
| `NodeKind` | 244-281 | Workflow node type enum (36 variants) |
| `EdgeType` | 370-379 | Graph edge classification (8 types) |
| `CertificateWire` | 414-422 | Verification certificate |
| `TaintPathWire` | 425-433 | Taint propagation edge |
| `NodeDescriptor` | 436-446 | Workflow graph node |
| `EdgeDescriptor` | 449-459 | Workflow graph edge |
| `IpcTraceEvent` | 463-468 | Trace event wrapper |
| `IpcTraceEventKind` | 471-512 | Trace event payload (12 variants) |

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 🔴 CRITICAL: Untyped Ticket Identifiers

**Locations:**
- `AnswerAsk.ticket: u64` (line 51)
- `CompleteAction.ticket: u64` (line 64)
- `FailAction.ticket: u64` (line 73)

**Problem:** `u64` is used for ticket IDs across three separate variants. This conflates raw numeric identity with domain concept. No validation boundary.

**Fix:** Introduce `TicketId` newtype wrapper.

---

### 🔴 CRITICAL: Untyped Sequence/Count Primitives

**Locations:**
- `ListEvents.from_sequence: u64` (line 44)
- `DrainTrace.max_records: u32` (line 82)
- `ListRuns.limit: u32` (line 87)
- `RunSummary.submitted_seq: u64` (line 138)
- `RunSummary.finished_seq: Option<u64>` (line 140)
- `RunSummary.step_count: u16` (line 142)
- `RunSummary.steps_completed: u16` (line 144)
- `VerificationResult.total_checks: u32` (line 153)
- `VerificationResult.pass_count: u32` (line 155)
- `VerificationResult.fail_count: u32` (line 157)

**Problem:** Raw integers without domain typing. No compile-time enforcement of valid ranges.

**Fix:** `SequenceNumber`, `RecordLimit`, `StepCount`, `CheckCount` newtypes.

---

### 🟡 MEDIUM: Untyped Step Indices

**Locations:**
- `TaintPathWire.from: u16` (line 428)
- `TaintPathWire.to: u16` (line 430)
- `NodeDescriptor.step_idx: u16` (line 439)
- `NodeDescriptor.next: Option<u16>` (line 443)
- `EdgeDescriptor.from: u16` (line 452)
- `EdgeDescriptor.to: u16` (line 454)

**Problem:** `u16` for step indices - already imported `StepIdx` but not used consistently.

**Fix:** Use `StepIdx` consistently instead of raw `u16`.

---

### 🟡 MEDIUM: Raw Byte Vectors

**Locations:**
- `SubmitRunPayload.input: Vec<u8>` (line 18)
- `AnswerAsk.answer: Vec<u8>` (line 53)
- `CompleteAction.output: Vec<u8>` (line 66)
- `FailAction.error: Vec<u8>` (line 75)
- `SlotWritten.value: Vec<u8>` (line 483)

**Problem:** `Vec<u8>` for serialized data. No type marker for what encoding (postcard, json, cbor).

**Fix:** Typed byte wrappers: `PostcardBytes`, `JsonBytes`, `CborBytes`.

---

## 4. WIRE SERIALIZATION DUPLICATION

### GateKind (lines 189-225)
```rust
impl GateKind {
    pub const fn as_str(&self) -> &'static str { ... }
}
impl TryFrom<&str> for GateKind { ... }
```

### NodeKind (lines 283-365)
```rust
impl NodeKind {
    pub const fn as_str(&self) -> &'static str { ... }
}
impl From<&str> for NodeKind { ... }
```

### EdgeType (lines 381-411)
```rust
impl EdgeType {
    pub const fn as_str(&self) -> &'static str { ... }
}
impl From<&str> for EdgeType { ... }
```

**Problem:** IDENTICAL PATTERN repeated 3 times. Each `as_str()` is a 9+ line match statement. Each `TryFrom/From` is a 9+ line match statement. This is 54+ lines of mechanical duplication.

**Fix:** Generate via macro or move to a shared `WireEnum` trait.

---

## 5. DDD COHESION VIOLATIONS

### Bundle Smell
This file mixes:
- **Command payloads** (SubmitRun, CancelRun, InspectRun)
- **Query payloads** (ListRuns, GetMetrics, GetWorkflowGraph)
- **Event types** (IpcTraceEventKind)
- **Wire adapters** (CertificateWire, NodeDescriptor)
- **Verification domain** (GateKind, VerificationResult)
- **Taint analysis domain** (TaintPathWire, TaintPathStatus)

**Scott Wlaschin Principle:** One Aggregate per file/module. This file tries to be an IPC types bucket.

---

## 6. REFACTORING RECOMMENDATIONS

### Split into Modules (target: 5-7 files)

```
vb_ipc/src/payloads/
├── mod.rs           (reexports)
├── commands.rs      (IpcPayload, SubmitRunPayload, CancelRun, etc.)
├── queries.rs        (ListRuns, GetWorkflowGraph, GetMetrics, etc.)
├── events.rs         (IpcTraceEvent, IpcTraceEventKind)
├── verification.rs   (GateKind, VerificationResult, CertificateWire)
├── taint.rs          (TaintPathStatus, TaintPathWire)
└── graph.rs          (NodeKind, EdgeType, NodeDescriptor, EdgeDescriptor)
```

### Newtype Wrappers to Introduce

| Newtype | Underlying | Purpose |
|---------|------------|---------|
| `TicketId` | `u64` | Action/ask ticket identifier |
| `SequenceNumber` | `u64` | Event sequence number |
| `RecordLimit` | `u32` | Max records to return |
| `StepOffset` | `u16` | Step index offset |
| `CheckCount` | `u32` | Verification check count |

### Macro for Wire Enums

```rust
macro_rules! wire_enum {
    ($name:ident, $str_map:expr) => {
        impl $name {
            pub const fn as_str(&self) -> &'static str { ... }
        }
        impl TryFrom<&str> for $name { ... }
    };
}
```

---

## 7. EVIDENCE COMMAND

```bash
wc -l crates/vb_ipc/src/payloads.rs
# Expected: 575 (VIOLATION)
```

---

## 8. VERDICT

| Check | Status |
|-------|--------|
| Line count < 300 | 🔴 FAIL (575 lines) |
| Primitive obsession | 🔴 FAIL (multiple untyped u64/u32/u16) |
| DDD cohesion | 🔴 FAIL (mixed aggregates) |
| Wire duplication | 🔴 FAIL (3x identical patterns) |
| Tests present | 🟡 PRESENT (but inline, not ideal) |

**Overall: HARD FAIL — Refactor required before approval.**
