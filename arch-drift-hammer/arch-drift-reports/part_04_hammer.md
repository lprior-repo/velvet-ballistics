# Architectural Drift Report: part_04.rs

**File**: `crates/vb_compile/src/mod_compile_lowering/part_04.rs`  
**Line Count**: 312 lines (VIOLATION: exceeds 300 line limit)  
**Status**: REFACTOR REQUIRED

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 312 | 300 | **VIOLATION (+12)** |

**Required Action**: Split into 2 files at ~150 lines each.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw String Slot Parsing (CRITICAL)

| Location | Issue |
|----------|-------|
| `lower_canonical_aggregate` L18-19 | `input: &str`, `initial: &str` — raw strings parsed inline |
| `lower_canonical_wait` L156-157 | `event: Option<&str>`, `timeout: Option<&str>` — raw strings |
| `lower_canonical_ask` L187-188 | `prompt: &str`, `timeout: Option<&str>` — raw strings |
| `emit_single_body_set` L214 | `value: &str` passed to `body_constant_index` |
| `body_constant_index` L301 | `value: &str` parsed to `i64` directly |

**Violation**: `&str` used for domain concepts (slot names, timeout values, prompts). Should use dedicated value types.

### 2.2 Raw Integer Primitives

| Location | Primitive | Issue |
|----------|-----------|-------|
| `lower_canonical_repeat` L88 | `max_attempts: u16` | Raw u16, no wrapper type |
| `emit_single_body_set` L246-252 | `action.parse::<i64>()` | ActionId parsed from raw string |
| `emit_single_body_set` L262-268 | `input.parse::<i64>()` | SlotIdx parsed from raw string |
| `lower_canonical_aggregate` L26 | `parse_i64_field(initial, ...)` | Integer parsed inline |

**Violation**: Domain IDs (`ActionId`, `SlotIdx`) parsed from raw strings rather than being constructed via type-safe APIs.

### 2.3 Diagnostic Index Leakage

| Location | Issue |
|----------|-------|
| `lower_canonical_aggregate` L15 | `index: usize` passed alongside `id: StepIdx` |
| `lower_canonical_repeat` L86 | `index: usize` for diagnostics |
| `lower_canonical_wait` L154 | `index: usize` for diagnostics |
| `lower_canonical_ask` L185 | `index: usize` for diagnostics |
| `emit_single_body_set` L216 | `diagnostic_step: usize` |

**Violation**: `usize` index leaked into functions. Should be encapsulated or use a diagnostic wrapper type.

---

## 3. DDD WORKFLOW VIOLATIONS

### 3.1 `emit_single_body_set` - God Function (CRITICAL)

**Lines**: 213-297 (84 lines)

This function handles TWO completely different step primitives:
1. `StepPrimitive::Set` — constant assignment
2. `StepPrimitive::Do` — action invocation

**Scott Wlaschin Violation**: A single function processing multiple workflow states indicates missing type separation.

**Required Refactor**:
```
emit_single_body_set_set()   // Handle Set variant
emit_single_body_set_do()    // Handle Do variant
```

### 3.2 Inline Parsing in Workflow Functions

| Function | Violation |
|----------|-----------|
| `lower_canonical_aggregate` | `slot_from_text()`, `parse_i64_field()` called inline |
| `lower_canonical_repeat` | `checked_step_offset()` inline |
| `lower_canonical_wait` | Pattern matching on `(event, timeout)` tuple with inline parsing |
| `lower_canonical_ask` | Multiple inline parsing calls |

**Violation**: "Parse, don't validate" principle is subverted by mixing parsing with workflow logic.

---

## 4. SPECIFIC CODE SMELLS

### 4.1 Complex Match Expression (Wait)

```rust
// Lines 161-178: 18-line match with mixed parsing and lowering
let mut node = match (event, timeout) {
    (Some(event_text), timeout_text) => { /* parsing */ lower_wait(...) }
    (None, Some(timeout_text)) => { /* parsing */ lower_wait(...) }
    (None, None) => { return Err(...) }
};
```

**Issue**: Parsing logic mixed with control flow. Should extract `WaitConfig` struct.

### 4.2 Magic Constants

| Location | Issue |
|----------|-------|
| L25 `SlotIdx::new(1)` | Magic number 1 for accumulator |
| L105 `SlotIdx::new(1)` | Magic number 1 for attempt slot |
| L194 `SlotIdx::new(2)` | Magic number 2 for answer slot |

**Violation**: Magic numbers suggest missing named constants or configuration structs.

---

## 5. RECOMMENDED REFACTORING PLAN

### Phase 1: Split File (~150 lines each)
```
part_04_aggregate_repeat.rs  // lower_canonical_aggregate, lower_canonical_repeat
part_04_wait_ask.rs          // lower_canonical_wait, lower_canonical_ask, emit_single_body_set, body_constant_index
```

### Phase 2: Extract Value Types
- Create `SlotName(String)` wrapper instead of raw `&str`
- Create `TimeoutValue(&str)` instead of raw `Option<&str>`
- Create `PromptText(&str)` instead of raw `&str`

### Phase 3: Extract Constants
- `ACCUMULATOR_SLOT = SlotIdx::new(1)`
- `ATTEMPT_SLOT = SlotIdx::new(1)`  
- `ANSWER_SLOT = SlotIdx::new(2)`

### Phase 4: Split `emit_single_body_set`
- `emit_set_node()` — handles Set variant only
- `emit_do_node()` — handles Do variant only

---

## 6. SUMMARY

| Category | Count | Severity |
|----------|-------|----------|
| Line count violations | 1 | CRITICAL |
| Primitive obsession (String/str) | 8 | HIGH |
| Primitive obsession (integers) | 5 | HIGH |
| God functions | 1 | CRITICAL |
| Magic numbers | 3 | MEDIUM |
| Missing value types | 6+ | HIGH |

**VERDICT**: This file MUST be refactored before acceptance. The 300-line limit is hard enforcement.
