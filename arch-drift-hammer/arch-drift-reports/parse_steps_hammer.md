# Architectural Drift Report: `parse_steps.rs`

**File**: `crates/vb_yaml/src/ast/parse_steps.rs`
**Line Count**: 354 lines
**Budget**: 300 lines
**Overage**: 54 lines (18% excess)
**Status**: 🚨 VIOLATION — IMMEDIATE REFACTOR REQUIRED

---

## 1. Line Count Violation

| Metric | Value |
|--------|-------|
| Actual | 354 |
| Max | 300 |
| Overage | 54 |
| % Excess | 18% |

---

## 2. Scott Wlaschin DDD Violations

### 2.1 Primitive Obsession (PRIMARY)

Every semantic domain concept is stored as raw `String` or primitive integer. No value objects exist for:

| Field | Type | Semantic Domain |
|-------|------|-----------------|
| `StepAst.condition` | `Option<String>` | Condition expression |
| `StepAst.then` | `Option<String>` | Flow-control label |
| `StepPrimitive::Wait.event` | `Option<String>` | Event identifier |
| `StepPrimitive::Wait.timeout` | `Option<String>` | Duration expression |
| `StepPrimitive::Ask.prompt` | `String` | Human prompt text |
| `StepPrimitive::Ask.timeout` | `Option<String>` | Duration expression |
| `RetryPolicy.delay` | `Option<String>` | Delay/duration expression |
| `StepPrimitive::Do.action` | `String` | Action identifier |
| `StepPrimitive::Set.output` | `String` | Variable name |
| `StepPrimitive::ForEach.variable` | `String` | Loop variable name |

**Consequence**: No validation, no type safety, no meaningful `Display`/`FromStr` implementations. Call sites cannot distinguish between semantically different strings without runtime checks.

### 2.2 Duplicated Code Pattern

`parse_wait` (lines 306–331) and `parse_ask` (lines 333–348) contain **near-identical** inline code for extracting optional strings:

```rust
// parse_wait lines 308-329
let event = match lookup(sub, "event") {
    Some(v) => Some(
        v.as_str()
            .ok_or(YamlError::FieldShape {
                field: "wait.event",
                expected: "string",
            })?
            .to_string(),
    ),
    None => None,
};
let timeout = match lookup(sub, "timeout") {
    Some(v) => Some(
        v.as_str()
            .ok_or(YamlError::FieldShape {
                field: "wait.timeout",
                expected: "string",
            })?
            .to_string(),
    ),
    None => None,
};

// parse_ask lines 336-346 — SAME PATTERN with different field names
let timeout = match lookup(sub, "timeout") {
    Some(v) => Some(
        v.as_str()
            .ok_or(YamlError::FieldShape {
                field: "ask.timeout",  // <-- only difference
                expected: "string",
            })?
            .to_string(),
    ),
    None => None,
};
```

This violates **DRY**. Extract `opt_string_field(node, key, context)` to `parse.rs`.

### 2.3 `is_primitive` String Matching

```rust
fn is_primitive(field: &str) -> bool {
    matches!(
        field,
        "set" | "save" | "do" | "run" | "choose" | "foreach" | ...
    )
}
```

This is a **language grammar encoded as string matching**. Should be derived from `StepPrimitive` enum via `impl StepPrimitive { pub fn kind(&self) -> &'static str }` or a `HashMap<&str, StepPrimitive>` dispatch table.

### 2.4 `reject_unknown_step_fields` Duplication

The hardcoded list at lines 121–146 **duplicates** the primitive names in `is_primitive()` plus legacy names. This is two sources of truth. Should be a single `const STEP_PRIMITIVES: &[&str]` or derived from the enum.

---

## 3. Wrong Abstraction Level

### 3.1 `parse_step_primitive` Is Too Large (52 lines)

Lines 44–96 in `parse_step_primitive`:
- Intercepts legacy names (lines 54–65)
- Dispatches to 15 primitive parsers (lines 81–95)
- Validates single-primitive constraint (lines 67–72)

This should be split:
- Legacy error interceptor → separate function
- Dispatch → small match or `HashMap` lookup
- Each primitive parser → its own function (already done)

### 3.2 Bounded Context Leakage

`saphyr::Yaml` (the YAML library type) appears in function signatures throughout. The parsing layer is correctly isolated in `vb_yaml`, but the **AST types** (`StepPrimitive`, `RetryPolicy`, `ErrorHandlerAst`) store raw primitives instead of domain types.

**Evidence**: `types.rs` defines `Wait { event: Option<String>, timeout: Option<String> }` — these should be `Wait { event: Option<EventId>, timeout: Option<Timeout> }` with validation inside the value object constructors.

---

## 4. Function-by-Function Analysis

| Function | Lines | Issue |
|----------|-------|-------|
| `parse_steps` | 11 | ✅ Clean entry point |
| `parse_step` | 20 | ✅ Coherent step aggregator |
| `parse_step_primitive` | 53 | 🚨 Too large, mixes concerns |
| `is_primitive` | 19 | 🚨 String-matching replacement for type system |
| `reject_unknown_step_fields` | 30 | 🚨 Duplicates `is_primitive` list |
| `parse_set` | 11 | ✅ Small, focused |
| `parse_do` | 12 | ✅ Small, focused |
| `parse_choose` | 17 | ✅ Clean branching |
| `parse_foreach` | 13 | ✅ Clean iteration |
| `parse_together` | 12 | ✅ Clean parallel |
| `parse_collect` | 15 | ✅ Clean pagination |
| `parse_reduce` | 12 | ✅ Clean fold |
| `parse_repeat` | 6 | ✅ Small, focused |
| `parse_body_steps` | 10 | ✅ Clean recursion |
| `parse_retry` | 27 | 🚨 Inline string extraction duplicated elsewhere |
| `parse_error_handler` | 7 | ✅ Small, focused |
| `parse_wait` | 26 | 🚨 Duplicates `parse_ask` string extraction |
| `parse_ask` | 16 | 🚨 Duplicates `parse_wait` string extraction |
| `parse_finish` | 5 | ✅ Clean exit |

---

## 5. Refactoring Prescription

### Phase 1: Extract Helper (Smallest Safe Refactor)
Add to `parse.rs`:
```rust
pub(super) fn opt_string_field(
    node: &saphyr::Yaml<'_>,
    key: &str,
    context: &'static str,
) -> YamlResult<Option<String>> {
    match lookup(node, key) {
        Some(v) => match v.as_str() {
            Some(s) => Ok(Some(s.to_string())),
            None => Err(YamlError::FieldShape {
                field: context,
                expected: "string",
            }),
        },
        None => Ok(None),
    }
}
```

### Phase 2: Add Value Objects to `types.rs`
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeout(String); // or Duration if parsing is feasible

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delay(String);
```

### Phase 3: Replace String Types in `StepPrimitive`
```rust
Wait {
    event: Option<EventId>,
    timeout: Option<Timeout>,
},
Ask {
    prompt: Prompt,
    timeout: Option<Timeout>,
},
```

### Phase 4: Split `parse_steps.rs`
Target structure:
```
ast/
├── parse_steps.rs          (~80 lines: entry + shared)
├── parse_primitives.rs     (~120 lines: dispatch table + legacy)
├── parse_set_do.rs         (~30 lines)
├── parse_branches.rs       (~60 lines: choose/together)
├── parse_loops.rs          (~80 lines: foreach/collect/reduce/repeat)
└── parse_wait_ask.rs       (~30 lines after helper extraction)
```

### Phase 5: Replace `is_primitive` with Enum Dispatch
```rust
impl StepPrimitive {
    pub const DISPATCH: &[(&str, fn(&saphyr::Yaml<'_>) -> YamlResult<Self>)] = &[
        ("set", parse_set),
        ("save", parse_set),
        ("do", |s| parse_do(s, "do")),
        ("run", |s| parse_do(s, "run")),
        // ...
    ];
}
```

---

## 6. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Breaking `vb_yaml` consumers | MEDIUM | `StepPrimitive` is `#[non_exhaustive]`; add new variants |
| YAML error context loss | LOW | Helper preserves context strings |
| Test breakage | LOW | Existing tests use `parse_workflow_source` which calls this |

---

## 7. Verdict

**IMMEDIATE ACTION REQUIRED.** The 354-line file violates the <300-line mandate by 18%. Primitive obsession pervades the type definitions. The duplicated `parse_wait`/`parse_ask` pattern is the most obvious code smell — extract the helper and replace with value objects in a subsequent bead.

**Recommended Next Bead**: Extract `opt_string_field` helper + introduce `Timeout` value object.
