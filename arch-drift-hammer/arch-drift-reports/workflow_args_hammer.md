# Architectural Drift Report: `workflow.rs` (468 lines)

**Agent:** arch-drift-hammer
**File:** `/home/lewis/src/velvet-ballistics/crates/vb_cli/src/args/tests/workflow.rs`
**Status:** CRITICAL DRIFT — MANDATORY REMEDIATION

---

## Executive Summary

| Violation | Severity | Rule |
|-----------|----------|------|
| File size 468 > 300 lines | **CRITICAL** | <300 line rule |
| Primitive obsession: `PathBuf` for workflow paths | HIGH | DDD Value Objects |
| Primitive obsession: raw string arrays for CLI args | HIGH | DDD Value Objects |
| Primitive obsession: flag strings not domain-typed | HIGH | DDD Ubiquitous Language |
| Anemic test structure using `if let` + `else assert` | MEDIUM | Bitter Truth |
| Legacy flag tests co-mingled with primary tests | MEDIUM | Separation of Concerns |
| No test data builders | MEDIUM | DRY / Maintainability |

---

## 1. File Size Violation (CRITICAL)

**Finding:** 468 lines in a single test file violates the architectural <300 line rule.

```
File: workflow.rs
Lines: 468
Limit:  300
Excess: 168 lines (+56%)
```

**Impact:** Unmaintainable, impossible to review, cognitively overloaded.

**Required Action:** Split into atomic test modules per command:
- `workflow_validate.rs` (~50 lines)
- `workflow_explain.rs` (~50 lines)
- `workflow_compile.rs` (~70 lines)
- `workflow_verify.rs` (~70 lines)
- `workflow_graph.rs` (~50 lines)
- `workflow_simulate.rs` (~60 lines)
- `workflow_bench_run.rs` (~50 lines)

---

## 2. Primitive Obsession: `PathBuf` for Workflow Paths

**Location:** All test assertions use `PathBuf::from("workflow.yaml")`

**Problem:** `PathBuf` is a primitive collection type. The domain concept "workflow file" has no identity in the test suite. A workflow file is not merely a path — it has validation requirements (`.yaml` extension, existence check, schema validation).

**Current code:**
```rust
assert_eq!(workflow, PathBuf::from("workflow.yaml"));
```

**Should be:**
```rust
// A WorkflowFile value object with domain validation
assert_eq!(workflow, WorkflowFile::try_from("workflow.yaml").unwrap());
```

**Missing domain type:** `WorkflowFile` or `WorkflowPath` value object that:
- Validates `.yaml` extension
- Optionally validates file existence
- Carries semantic meaning beyond "a path to something"

---

## 3. Primitive Obsession: Raw String Arrays for CLI Arguments

**Location:** Every test builds CLI args via raw string arrays:

```rust
&args(&[
    "velvet-ballistics",
    "validate",
    "workflow.yaml",
    "--emit",
    "yaml",
])
```

**Problem:** 
- `"velvet-ballistics"` — hardcoded binary name, should be `CliBinary::NAME`
- `"validate"`, `"explain"`, `"compile"` — command names not bound to domain type
- `"--emit"`, `"--profile"`, `"--out"` — flag strings typos-prone, not validated against domain

**Missing domain types:**
```rust
struct CliCommand {
    binary: CliBinary,
    subcommand: Subcommand,
    positional: Vec<PositionalArg>,
    flags: BTreeMap<Flag, FlagValue>,
}

enum Subcommand { Validate, Explain, Compile, ... }
enum Flag { Emit, Profile, Out, ... }
```

**Required refactor:** Create a `TestCommandBuilder` that:
- Uses typed `Subcommand` variants
- Validates flag combinations at construction
- Eliminates raw string literals from test bodies

---

## 4. Primitive Obsession: Flag Strings Not Domain-Typed

**Location:** Raw string literals throughout:
- `"--emit"`, `"--profile"`, `"--out"`, `"--json"`, `"--jsonl"`
- `"quick"`, `"standard"`, `"full"` (profile values)
- `"ir"`, `"yaml"`, `"postcard"` (emit targets)

**Problem:** These are domain vocabulary but expressed as untyped strings. Typos compile silently.

**Should be constants from a domain types:**
```rust
mod flags {
    pub const EMIT: &str = "--emit";
    pub const PROFILE: &str = "--profile";
    pub const OUT: &str = "--out";
}

mod values {
    pub const QUICK: &str = "quick";
    pub const STANDARD: &str = "standard";
    pub const FULL: &str = "full";
}
```

Better: Use enums with `as_str()` implementations (already exists for `VerifyProfile::as_str()`) but tests bypass this.

---

## 5. Anemic Test Structure: `if let` + `else assert` Anti-Pattern

**Location:** Every test uses this pattern:

```rust
if let Ok(Command::Validate { output, .. }) = parsed {
    assert_eq!(output, OutputFormat::Yaml);
} else {
    assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
}
```

**Problem:** 
- Hides failures — the `else` branch masks the actual error
- No distinction between `Err` and `Ok(wrong variant)`
- Uses `is_ok()` in assertion message which defeats the purpose

**Should use `let...else` or `matches!` with clear error messages:**

```rust
let Ok(Command::Validate { output, .. }) = parsed else {
    panic!("expected Validate command, got {parsed:?}");
};
assert_eq!(output, OutputFormat::Yaml);
```

Or for error cases:
```rust
let Err(ParseError::UnknownFlag { command: "validate", .. }) = parsed else {
    panic!("expected UnknownFlag error, got {parsed:?}");
};
```

---

## 6. Legacy Flag Tests Co-mingled

**Location:** Lines with `--json` and `--jsonl` flags:

- Line 94: `parse_explain_legacy_jsonl_flag_keeps_text_output`
- Line 173: `parse_compile_legacy_json_flag_keeps_text_output`
- Line 192: `parse_compile_legacy_jsonl_flag_keeps_text_output`
- Line 349: `parse_graph_legacy_json_flag_keeps_text_output`
- Line 375: `parse_simulate_legacy_json_flag_keeps_text_output`
- Line 390: `parse_simulate_legacy_jsonl_flag_keeps_text_output`

**Problem:** Legacy behavior tests mixed with current behavior tests. These should be:
1. Clearly marked as legacy tests
2. Moved to a separate `legacy_flags.rs` module
3. Or removed if legacy flags are deprecated and no longer relevant

---

## 7. No Test Data Builders

**Location:** Every test manually constructs `&["velvet-ballistics", "validate", "workflow.yaml"]`

**Problem:** Violates DRY, makes common patterns invisible, hard to update when CLI changes.

**Missing builder:**
```rust
struct CliTestBuilder {
    subcommand: Subcommand,
    workflow: Option<&'static str>,
    emit: Option<OutputFormat>,
    profile: Option<VerifyProfile>,
    out: Option<&'static str>,
    legacy_flags: Vec<&'static str>,
}

impl CliTestBuilder {
    fn validate(workflow: &'static str) -> Self { ... }
    fn with_emit(mut self, format: OutputFormat) -> Self { ... }
    fn with_profile(mut self, profile: VerifyProfile) -> Self { ... }
    fn build(&self) -> Vec<OsString> { ... }
}
```

---

## 8. Missing Domain Model for `EmitTarget` and `VerifyProfile`

**Problem:** Tests verify enum variants parse correctly but don't test domain behavior:

- `EmitTarget` (Ir, Yaml, Postcard) — no tests for invalid combinations
- `VerifyProfile` (Quick, Standard, Full) — no tests for what "full" actually verifies
- `OutputFormat` (Text, Yaml, Postcard) — tested for parsing but not for downstream behavior

**These are anemic domain types.** The tests verify parsing infrastructure, not domain behavior.

---

## Required Refactoring (Priority Order)

### Phase 1: File Split (Immediate)
```
args/tests/workflow.rs  →  args/tests/workflow/
    ├── mod.rs           (re-exports)
    ├── validate.rs     (~50 lines)
    ├── explain.rs      (~50 lines)
    ├── compile.rs      (~70 lines)
    ├── verify.rs       (~70 lines)
    ├── graph.rs        (~50 lines)
    ├── simulate.rs     (~60 lines)
    └── bench_run.rs    (~50 lines)
```

### Phase 2: Test Helpers (Short-term)
- Create `CliTestBuilder` in `tests/mod.rs`
- Move flag string constants to a shared location
- Replace `if let ... else { assert!(is_ok()) }` with `let...else`

### Phase 3: Domain Types (Medium-term)
- Create `WorkflowFile` value object
- Ensure all CLI types have proper `as_str()` / `try_from()` implementations
- Add property-based tests for enum parsing

### Phase 4: Legacy Separation
- Extract legacy flag tests to `legacy_flags.rs`
- Document deprecation timeline

---

## Conclusion

This file is in **severe architectural drift**. The combination of:
1. 56% over the line limit
2. Systematic primitive obsession
3. Anemic test patterns

Makes this file a **high-risk maintenance liability**.

**Recommendation:** Immediate refactor required before any new features touch this code.

---

*Report generated: 2026-05-29*
*Agent: arch-drift-hammer*
