# Architectural Drift Report: `vb_ipc_payloads.rs`

**File:** `crates/vb_ipc/src/payloads.rs`
**Analyzed:** 2026-05-29
**Status:** REFACTOR REQUIRED

---

## 1. Line Count Analysis

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **575** | 300 | ❌ OVER LIMIT |
| Code lines (excl tests) | ~460 | 300 | ❌ OVER LIMIT |
| Test lines | ~60 | — | ✓ |

**Verdict:** File MUST be split. At 575 lines, this is **91% over the 300-line limit**.

---

## 2. DDD Cohesion Analysis

### Bounded Contexts Identified

The file mixes **three distinct DDD bounded contexts** in a single 575-line file:

| Context | Types | Cohesion |
|--------|-------|----------|
| **IPC Command Surface** | `IpcPayload`, `SubmitRunPayload` | High — these ARE the IPC commands |
| **Workflow Graph Wire Format** | `NodeKind`, `EdgeType`, `NodeDescriptor`, `EdgeDescriptor`, `GateKind`, `CertificateWire`, `VerificationResult`, `PassFail` | Medium — IPC transport of workflow structure |
| **Trace/Event Wire Format** | `IpcTraceEvent`, `IpcTraceEventKind`, `TaintPathWire`, `TaintPathStatus`, `RunListState`, `RunSummary` | Medium — IPC transport of runtime events |
| **Parse Errors** | `ParseGateKindError` | Low — belongs with GateKind, not standalone |

### Cohesion Verdict

**COHESION SMELL: Low**. The file serves as a catch-all for all IPC wire types, violating the principle of **single responsibility for bounded contexts**. A DDD-aligned structure would split this into:

```
vb_ipc/src/payloads/
├── mod.rs           (reexports)
├── commands.rs      (IpcPayload, SubmitRunPayload)
├── graph_wire.rs    (NodeKind, EdgeType, NodeDescriptor, EdgeDescriptor, GateKind, CertificateWire, VerificationResult, PassFail)
├── trace_wire.rs    (IpcTraceEvent, IpcTraceEventKind, RunListState, RunSummary, TaintPathWire, TaintPathStatus)
└── errors.rs        (ParseGateKindError — if standalone)
```

---

## 3. Violations

### 🔴 CRITICAL: Structural Violations

#### V-001: File Size Exceeded (575 > 300 lines)
- **Severity:** Critical
- **Rule:** `<300 lines per .rs file`
- **Impact:** Maintainability, compilation unit isolation, cognitive load
- **Fix:** Split into `commands.rs`, `graph_wire.rs`, `trace_wire.rs`

#### V-002: Multiple Bounded Contexts in One File
- **Severity:** Critical
- **Rule:** DDD cohesion — one bounded context per module
- **Impact:** Violates single responsibility, makes IPC context monolithic
- **Fix:** Split by context as described above

### 🟠 HIGH: Parse, Don't Validate Violations

#### V-003: `NodeKind::from(&str)` Silently Falls Through to `Nop`
```rust
impl From<&str> for NodeKind {
    fn from(s: &str) -> Self {
        match s {
            "Nop" => NodeKind::Nop,
            // ... 30+ variants ...
            _ => NodeKind::Nop,  // ← SILENT FALLTHROUGH
        }
    }
}
```
- **Severity:** High
- **Rule:** Parse, don't validate — unknown input should return Error
- **Impact:** Corrupted data silently becomes `Nop`, masking errors
- **Fix:** Implement `TryFrom<&str>` returning `ParseNodeKindError` for unknown strings, keep `From<&str>` for known-only contexts or deprecate

#### V-004: `EdgeType::from(&str)` Silently Falls Through to `Fallthrough`
```rust
impl From<&str> for EdgeType {
    fn from(s: &str) -> Self {
        match s {
            "branch" => EdgeType::Branch,
            // ... 7 variants ...
            _ => EdgeType::Fallthrough,  // ← SILENT FALLTHROUGH
        }
    }
}
```
- **Severity:** High
- **Rule:** Parse, don't validate
- **Impact:** Unknown edge type becomes `Fallthrough`, corrupting graph semantics
- **Fix:** Implement `TryFrom<&str>` returning `ParseEdgeTypeError`

### 🟡 MEDIUM: Primitive Obsession

#### V-005: Untyped Ticket Identifiers
- **Locations:** `IpcPayload::AnswerAsk { ticket: u64 }`, `IpcPayload::CompleteAction { ticket: u64 }`, `IpcPayload::FailAction { ticket: u64 }`
- **Issue:** `u64` is used for ticket IDs without a NewType wrapper
- **Impact:** Mixable with other `u64` values (run_id, sequence numbers)
- **Fix:** Create `TicketId(u64)` NewType with `TryFrom<u64>` validation

#### V-006: Untyped Index Primitives in Wire Types
- **Locations:** `TaintPathWire { from: u16, to: u16 }`, `NodeDescriptor { step_idx: u16, next: Option<u16> }`, `EdgeDescriptor { from: u16, to: u16 }`
- **Issue:** `u16` for step indices while `vb_core::ids::StepIdx` exists
- **Impact:** Type-level confusion between different index spaces
- **Fix:** Use `StepIdx` from `vb_core::ids` for wire types (requires bytes serialization compatibility check)

### 🟢 LOW: Additional Observations

#### V-007: `IpcTraceEventKind::Unknown` Lossy Parsing
- The `Unknown` variant for future compatibility is acceptable, but parsing into `Unknown` loses information that might be needed for debugging. Consider logging unknown variants when encountered.

#### V-008: Redundant `as_str()` + `TryFrom` Pairs
- `GateKind` has both `as_str()` and `TryFrom<&str>` correctly implemented
- `NodeKind` and `EdgeType` have `as_str()` but only `From<&str>` (should be `TryFrom`)
- This asymmetry is a maintenance hazard

#### V-009: Test Coverage Only on GateKind
- Tests only cover `GateKind` parsing (lines 518-574)
- `NodeKind` and `EdgeType` `From` implementations have NO test coverage
- This is risky given their fallthrough behavior

---

## 4. DDD Smell Summary

| Smell | Category | Severity |
|-------|----------|----------|
| Monolithic IPC payload file | Feature Envy / Silo | 🔴 Critical |
| Silent fallthrough on unknown parse | Parse not validate | 🟠 High |
| Untyped primitives in wire format | Primitive Obsession | 🟡 Medium |
| Asymmetric TryFrom/From implementation | Inconsistent API | 🟡 Medium |
| No test coverage on NodeKind/EdgeType parse | Test gap | 🟡 Medium |

---

## 5. Remediation Priority

| Priority | Action | Effort | Impact |
|----------|--------|--------|--------|
| **P0** | Split file into `commands.rs` + `graph_wire.rs` + `trace_wire.rs` | High | Fixes V-001, V-002 |
| **P0** | Implement `TryFrom<&str>` for `NodeKind` and `EdgeType` with explicit errors | Medium | Fixes V-003, V-004 |
| **P1** | Add test coverage for `NodeKind` and `EdgeType` parsing | Medium | Fixes V-009 |
| **P2** | Add `TicketId(u64)` NewType wrapper | Low | Fixes V-005 |
| **P2** | Evaluate `StepIdx` usage in wire types (bytes compat) | Low | Fixes V-006 |
| **P3** | Add logging for `Unknown` trace event variants | Low | Fixes V-007 |

---

## 6. Recommended Module Structure

```rust
// vb_ipc/src/payloads/mod.rs
pub mod commands;    // IpcPayload, SubmitRunPayload (~100 lines)
pub mod graph_wire;  // NodeKind, EdgeType, GateKind, VerificationResult, Certificates (~250 lines)
pub mod trace_wire;  // IpcTraceEvent, RunSummary, TaintPathWire (~150 lines)
pub mod errors;      // ParseGateKindError, ParseNodeKindError, ParseEdgeTypeError (~30 lines)
```

---

## 7. Verification Commands

```bash
# Check line counts after refactor
wc -l crates/vb_ipc/src/payloads/*.rs

# Verify no unsafe code
grep -r "unsafe" crates/vb_ipc/src/payloads/ && echo "VIOLATION: unsafe found" || echo "CLEAN"

# Verify all TryFrom implementations exist
grep -E "impl.*TryFrom.*str.*for (NodeKind|EdgeType|GateKind)" crates/vb_ipc/src/payloads/*.rs
```

---

**END REPORT**
