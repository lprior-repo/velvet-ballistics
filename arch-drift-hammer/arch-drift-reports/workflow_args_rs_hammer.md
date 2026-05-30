# Architectural Drift Report: `workflow.rs`

**File**: `crates/vb_cli/src/args/workflow.rs`
**Line Count**: 384 (VIOLATION: exceeds 300-line limit)
**Status**: REFACTOR REQUIRED

---

## Executive Summary

This file is a **384-line argument parsing module** with severe primitive obsession and DDD violations. It violates the `<300 line` architectural rule by **84 lines** and is a hotbed of Scott Wlaschin DDD anti-patterns.

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 384 | 300 | **VIOLATION (+84)** |
| Production Code | ~170 | - | - |
| Tests | ~204 | - | - |

**Required Action**: Split into at minimum 2 files:
- `workflow.rs` (~150 lines): High-level workflow command parsers
- `workflow_tests.rs` (~150 lines): Test scaffolding
- `workflow_parsing_helpers.rs` (~80 lines): Shared parsing primitives

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw `u16` for Step ID (Line 83)

```rust
let step_id = step_raw
    .parse::<u16>()
    .map_err(|_| ParseError::MissingArgument("--step"))?;
```

**Problem**: `u16` is a primitive. No validation that step_id is in valid range.

**Fix**: Create `StepId(u16)` newtype with bounded validation:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepId(pub u16);

impl StepId {
    pub fn parse(raw: &str) -> Result<Self, ParseError> {
        raw.parse::<u16>()
            .map(Self)
            .map_err(|_| ParseError::InvalidStep(raw.into()))
    }
}
```

### 2.2 Repeated `PathBuf::from(string)` Wrapping (11+ occurrences)

```rust
// Lines 52, 69, 89, 104, 115, 129, 130, 163, 164, etc.
out: PathBuf::from(out),
input_bin: PathBuf::from(input_bin),
socket: PathBuf::from(socket),
```

**Problem**: Every string-to-path conversion is manual. No domain-specific path types.

**Fix**: Create value objects:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPath(PathBuf);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBin(PathBuf);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDb(PathBuf);
```

### 2.3 Raw `String` for `OsString` Conversions

```rust
// Line 49, 55, etc.
named_flag(args, "--emit").as_deref()
```

**Problem**: `OsString` → `String` conversion via `to_str()` is fallible but handled inconsistently.

---

## 3. DDD VIOLATIONS (Scott Wlaschin)

### 3.1 `parse_durability` is a Standalone Function (Lines 170-177)

```rust
fn parse_durability(raw: &str) -> Result<DurabilityMode, ParseError> {
    match raw {
        "strict" => Ok(DurabilityMode::Strict),
        "journaled" => Ok(DurabilityMode::Journaled),
        "none" => Ok(DurabilityMode::None),
        other => Err(ParseError::UnknownDurability(other.into())),
    }
}
```

**Problem**: Should be `DurabilityMode::parse(raw)` - a method on the type itself.

**Fix**:
```rust
impl DurabilityMode {
    pub fn parse(raw: &str) -> Result<Self, ParseError> {
        match raw {
            "strict" => Ok(Self::Strict),
            "journaled" => Ok(Self::Journaled),
            "none" => Ok(Self::None),
            other => Err(ParseError::UnknownDurability(other.into())),
        }
    }
}
```

### 3.2 No Value Objects for Domain Concepts

| Primitive | Domain Concept | Missing Newtype |
|-----------|---------------|-----------------|
| `PathBuf` | Workflow path | `WorkflowPath` |
| `PathBuf` | Input binary | `InputBin` |
| `PathBuf` | Database path | `RunDbPath` |
| `u16` | Step ID | `StepId` |
| `String` | Run ID | `RunId` |

### 3.3 Commands Are Anemic Data Bags

```rust
pub(crate) enum Command {
    Run {
        workflow: PathBuf,
        input_bin: PathBuf,
        durability: DurabilityMode,
        db: Option<PathBuf>,
        step: Option<StepTarget>,
        output: OutputFormat,
    },
    // ...
}
```

**Problem**: No behavior, just data. No state transitions modeled.

---

## 4. REPETITION VIOLATIONS

### 4.1 `parse_output_format(args)` Called 9 Times Identically

```rust
// Lines 18, 28, 34, 48, 66, 101, 136, 142, 148
let output = parse_output_format(args);
```

**Fix**: This should be a shared helper in `shared.rs`, not duplicated.

### 4.2 `PathBuf::from(...)` Wrapping Pattern

```rust
// 11+ occurrences of PathBuf::from(something)
PathBuf::from(out)      // line 52
PathBuf::from(input_bin) // line 69
PathBuf::from(step_input) // line 89
// ... etc
```

**Fix**: Use domain value objects that carry their own parsing.

### 4.3 `named_flag(args, "--db").ok_or(...)` Pattern

```rust
// Lines 127, 156, etc.
named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?
```

**Fix**: Create `require_named_flag(args, flag)` helper.

---

## 5. TEST PRIMITIVE OBSESSION

The tests (lines 179-384) are **205 lines** of repetitive scaffolding:

```rust
#[test]
fn parse_durability_returns_strict() {
    assert_eq!(parse_durability("strict").unwrap(), DurabilityMode::Strict);
}
```

**Problem**: Each test is nearly identical. Should use parameterized tests.

**Fix**: Use `try_build!` macro or proptest for exhaustive coverage.

---

## 6. REQUIRED REFACTORING

### File Split Plan

```
args/
├── mod.rs           (existing, ~1500 lines - also needs review)
├── shared.rs        (existing, 180 lines - ok)
├── workflow.rs      (REFACTOR: 150 lines)
├── workflow/
│   ├── mod.rs       (new, 50 lines)
│   ├── parsers.rs   (new, 80 lines - DurabilityMode::parse, StepId::parse, etc.)
│   └── tests.rs     (new, 100 lines - refactored tests)
```

### New Value Objects to Create

```rust
// In vb_cli/src/args/workflow/values.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPath(PathBuf);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowCompiledPath(PathBuf);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBin(PathBuf);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDbPath(PathBuf);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepId(u16);

impl StepId {
    pub fn parse(raw: &str) -> Result<Self, ParseError> {
        raw.parse::<u16>()
            .map(Self)
            .map_err(|_| ParseError::InvalidStep(raw.into()))
    }
}
```

### Trait Implementation for Parse

```rust
impl DurabilityMode {
    pub fn parse(raw: &str) -> Result<Self, ParseError> { /* ... */ }
}

impl EmitTarget {
    pub fn parse(raw: &str) -> Result<Self, ParseError> { /* ... */ }
}

impl VerifyProfile {
    pub fn parse(raw: &str) -> Result<Self, ParseError> { /* ... */ }
}
```

---

## 7. SUMMARY

| Category | Violations |
|----------|------------|
| Line Count | 1 (384 > 300) |
| Primitive Obsession | 5+ (u16, PathBuf wrapping, String) |
| DDD Violations | 4+ (standalone parse fns, anemic commands) |
| Repetition | 3+ patterns repeated 9+ times |

**VERDICT**: **REFACTOR REQUIRED**

The file must be split and the parsing logic must be moved onto the domain types themselves. No new features should be added until this is resolved.
