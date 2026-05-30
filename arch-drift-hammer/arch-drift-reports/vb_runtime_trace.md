# Architectural Drift Report: `vb_runtime/src/trace.rs`

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total file lines | **1380** | 300 | ❌ OVER LIMIT (+1080) |
| Production code | ~291 (lines 1–291) | 300 | ✅ PASS |
| Inline test module | ~1089 (lines 292–1380) | N/A | ❌ VIOLATION |

**Severity**: CRITICAL — file is **4.7× the line limit**.

---

## 2. DDD Cohesion Analysis

**Domain Concept**: Trace event recording and bounded ring buffer

**Cohesion Verdict**: **BORDERLINE / SMELLY**

The filename `trace.rs` correctly maps to the trace domain, but the module violates single-responsibility:

| Type | Lines | Concern |
|------|-------|---------|
| `TraceRing` struct | ~165 | Single concept — bounded ring buffer |
| `TraceEvent` enum | ~145 (incl. impl) | 12 variants — too many for one enum |

### DDD Smells Detected

1. **Enum Overload** (`TraceEvent`): 12 variants spanning three sub-domains:
   - Step events: `StepStarted`, `StepEnded`
   - Action events: `ActionScheduled`, `ActionCompleted`, `ActionFailed`
   - Run lifecycle events: `RunSubmitted`, `RunFinished`, `RunFailed`, `RunCancelled`, `RunKilled`
   - Slot/ask events: `SlotWritten`, `AskAnswered`

   **Should be split into**:
   - `StepEvent`, `ActionEvent`, `RunEvent`, `SlotEvent` sub-enums composed into `TraceEvent`

2. **Primitive Obsession** (minor): `value: Vec<u8>` on `SlotWritten` — bytes without a typed wrapper

3. **Inline Tests Polluting Domain Module**: 1089 lines of `#[cfg(test)] mod tests` mixed into the domain module — should be `tests/trace_tests.rs` or `trace/tests.rs`

---

## 3. Violations

### V-1: File Size Exceeded (CRITICAL)
- **Lines**: 1–1380
- **Required**: ≤300 lines
- **Actual**: 1380 lines
- **Overflow**: +1080 lines
- **Remediation**: Split into `trace/ring.rs`, `trace/event.rs`, `trace/tests.rs`

### V-2: Inline Test Module Oversized (CRITICAL)
- **Location**: Lines 292–1380
- **Size**: ~1089 lines
- **Test-to-Production Ratio**: 1089 / 291 = **3.74×**
- **Violation**: Tests should be in a separate `tests/` subdirectory or sibling test file
- **Remediation**: Move to `vb_runtime/tests/trace_tests.rs` or `trace/tests.rs`

### V-3: Missing Module Separation
- `TraceRing` and `TraceEvent` are two distinct domain concepts sharing one file
- No `mod.rs` splitting for `trace` directory
- Remediation: Create `crates/vb_runtime/src/trace/mod.rs`, `trace/ring.rs`, `trace/event.rs`

### V-4: Oversized `TraceEvent` Enum (MODERATE)
- 12 variants covering 4 sub-domains
- Violates "small enum" heuristic from Scott Wlaschin DDD
- Remediation: Decompose into `StepEvent`, `ActionEvent`, `RunEvent`, `SlotEvent` with a tagged union

### V-5: No `FromStr` / `Parse` Pattern for TraceEvent (MINOR)
- Event construction is only via direct struct literal; no validation layer
- Remediation: Add `TryFrom<&str>` for parsing trace strings

---

## 4. Specific Line References

```
Lines 1–2:     Module doc + forbid unsafe
Lines 4–8:     Imports
Lines 10–18:   TraceRing struct definition
Lines 20–164:  TraceRing impl block (all methods inline)
Lines 166–250: TraceEvent enum (12 variants)
Lines 252–290: TraceEvent impl block
Lines 292–1380: #[cfg(test)] mod tests — 1089 lines of inline tests
```

### Largest Individual Functions (production code)

| Function | Lines | Concern |
|----------|-------|---------|
| `TraceRing::new` | 10 | ✅ OK |
| `TraceRing::push` | 10 | ✅ OK |
| `TraceRing::drain_into` | 13 | ✅ OK |
| `TraceRing::drain_for_run` | 17 | ⚠️ Slightly large but acceptable |
| `TraceRing::snapshot_for_run` | 17 | ⚠️ Slightly large but acceptable |
| `TraceRing::has_terminal_event_for_run` | 16 | ⚠️ Slightly large but acceptable |
| `TraceEvent::run_id` | 16 | ⚠️ Large match, but clear pattern |
| `TraceEvent::is_terminal_for_run` | 16 | ⚠️ Large match, but clear pattern |

**No production function exceeds 20 lines — production code is well-structured.**

---

## 5. Recommended File Split

```
crates/vb_runtime/src/trace/
├── mod.rs          (reexports TraceRing, TraceEvent, re-exports from submodules)
├── ring.rs         (TraceRing struct + impl — ~165 lines)
├── event.rs        (TraceEvent enum + impl — ~145 lines)
└── tests/
    └── integration_tests.rs  (~1089 lines — moved inline tests here)
```

**Target**: Each file ≤300 lines after split.

---

## 6. Remediation Priority

| Priority | Violation | Effort |
|----------|-----------|--------|
| **P0 — CRITICAL** | Move inline tests to `tests/trace_tests.rs` | Low |
| **P0 — CRITICAL** | Split `trace.rs` → `trace/{mod,ring,event}.rs` | Medium |
| **P1 — HIGH** | Decompose `TraceEvent` into sub-enums | High |
| **P2 — MODERATE** | Wrap `value: Vec<u8>` in `SlotValue` newtype | Medium |
| **P3 — LOW** | Add `TryFrom` parsing for trace events | Low |

---

## 7. Summary

| Check | Result |
|-------|--------|
| Total lines under 300? | ❌ **NO — 1380 lines** |
| DDD cohesion (single concept)? | ⚠️ **BORDERLINE** — trace domain, but overloaded enum |
| Any function >300 lines? | ❌ **NO** — production functions are all <20 lines |
| Inline tests in module? | ❌ **YES — 1089 lines of inline tests (79% of file)** |
| Module separation? | ❌ **NO — trace types + tests all in one file** |
| DDD smell detected? | ✅ **YES — TraceEvent enum has 12 variants spanning 4 sub-domains** |

**STATUS**: `REFACTOR REQUIRED`

---
*Report generated by architectural-drift skill*
*File*: `crates/vb_runtime/src/trace.rs`
*Total lines*: 1380 | *Limit*: 300 | *Overflow*: +1080
