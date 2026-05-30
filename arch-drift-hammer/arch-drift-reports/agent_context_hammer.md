# Architectural Drift Report: `agent_context/mod.rs`

**File:** `crates/vb_cli/src/agent_context/mod.rs`
**Line Count:** 303 (OVERFLOW: +3 lines past 300-line limit)
**Severity:** CRITICAL

---

## Executive Summary

This file is a **JSON schema factory** masquerading as a domain module. It suffers from severe primitive obsession, responsibility conflation, and data-driven design that violates Scott Wlaschin's DDD principles. The file builds CLI surface metadata entirely through stringly-typed JSON construction, with zero domain types, zero behavior encapsulation, and zero meaningful abstraction.

---

## Violation 1: LINE COUNT OVERFLOW

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 303 | 300 | **+3 OVERFLOW** |

The file exceeds the architectural limit by 3 lines. This is a boundary violation requiring immediate refactor.

---

## Violation 2: Primitive Obsession — Domain Types

### 2.1 `ExitCode` is stringly-typed

**Location:** `exit_codes()` (lines 79–91), `known_blockers()` (lines 54–77)

```rust
// CURRENT: Primitive string keys mapping to exit codes
fn exit_codes() -> Value {
    serde_json::json!({
        "0": "success",
        "1": "validation failed",
        // ...
    })
}
```

**Problem:** Exit codes like `1`, `2`, `3` are raw integers encoded as string keys. There is no `ExitCode` enum, no `ExitCodeCategory` value object, no type-level guarantee that only valid exit codes exist.

**Required DDD types:**
- `enum ExitCode { Success, ValidationFailed, VerificationFailed, ... }`
- `enum ExitCodeCategory { Policy, Resource, Capability }`
- `struct Blocker { category: ExitCodeCategory, exit_code: ExitCode }`

### 2.2 `EmitFormat` is a string array

**Location:** `enums()` line 95, `output_emit_flag()` line 280

```rust
// CURRENT: String array for emit formats
"emit": ["ir", "yaml", "postcard"]

fn output_emit_flag() -> Value {
    serde_json::json!({"type": "enum", "values": ["text", "yaml", "postcard"], "default": "text"})
}
```

**Problem:** `EmitFormat` is an unbounded string set. The valid values `ir`, `yaml`, `postcard`, `text` exist only as comments and string literals. No compile-time enforcement.

**Required DDD type:**
```rust
enum EmitFormat { Ir, Yaml, Postcard, Text }
impl EmitFormat {
    fn variants() -> &'static [&'static str] { &["ir", "yaml", "postcard", "text"] }
}
```

### 2.3 `Durability` is a string array

**Location:** `enums()` line 96, `--durability` flag in multiple commands

```rust
"durability": ["strict", "journaled", "none"]
```

**Problem:** `Durability` is a raw string with no type safety. Commands like `run`, `run-compiled`, `submit` all pass `"strict"`, `"journaled"`, `"none"` as raw strings.

**Required DDD type:**
```rust
enum Durability { Strict, Journaled, None }
```

### 2.4 `VerifyProfile` is a string array

**Location:** `enums()` line 97, `--profile` flag in `verify` command

```rust
"verify_profile": ["quick", "standard", "full"]
```

**Problem:** `VerifyProfile` should be a typed enum, not a string array.

### 2.5 `Category` strings are primitive obsession

**Location:** `known_blockers()` lines 56–75

```rust
{"category": "validation_failed", "exit_code": 1},
{"category": "verification_failed", "exit_code": 2},
// ...
{"category": "slot_count_exceeded"},
{"category": "input_index_out_of_range"},
```

**Problem:** All categories are raw strings. `"validation_failed"`, `"slot_count_exceeded"`, `"unregistered_action"` should be typed enum variants.

---

## Violation 3: Primitive Obsession — Command Schema Building

### 3.1 `CommandName` is `&str`

**Location:** `command()` function, line 268–270

```rust
fn command(name: &str, value: Value) -> (String, Value) {
    (name.to_owned(), value)
}
```

**Problem:** Commands like `"agent-context"`, `"validate"`, `"verify"`, `"run"` are raw strings. There should be a `CommandName` newtype.

### 3.2 `FlagName` is `&str`

**Location:** Throughout `commands()` building JSON inline

```rust
"--profile": {"type": "enum", "values": ["quick", "standard", "full"], "default": "standard"},
"--emit": output_emit_flag(),
"--out": {"type": "path", "required": true}
```

**Problem:** Flag names like `"--emit"`, `"--profile"`, `"--out"`, `"--db"` are primitive strings.

### 3.3 `PositionalName` is `&str`

**Location:** Throughout `commands()` building positionals

```rust
"positionals": ["workflow.yaml"],
"positionals": ["run_id"],
"positionals": ["run_a", "run_b"],
```

**Problem:** Positional argument names are raw strings. Should be typed.

---

## Violation 4: Responsibility Conflation

This file conflates THREE distinct responsibilities:

1. **Agent Contract Definition** — Lines 7–42 (`build()` function)
2. **CLI Schema Generation** — Lines 44–300 (all helper functions)
3. **Planned Primitives Declaration** — Lines 292–300 (`planned_agent_primitives()`)

These should be SEPARATE modules with SEPARATE types:
- `AgentContract` — the core contract types
- `CliSchema` — CLI command definitions
- `PlannedPrimitives` — future agent capabilities

---

## Violation 5: Data Pump Anti-Pattern

The file has **zero meaningful behavior**. Every function is a pure constructor of `serde_json::Value`. There are no:
- State transitions
- Validation logic
- Business rules
- Domain invariants

This is a **data transfer object (DTO) factory**, not a domain module. It should be generated from typed domain definitions, not built by hand with raw JSON.

---

## Violation 6: Stringly-Typed Banned Vocabulary

**Location:** `build()` lines 32–33

```rust
"banned_verbs": ["info", "ls"],
"banned_flags": ["--json", "--jsonl", "--format=json", "--output=json", "--skip-confirmations"]
```

These banned verbs and flags should be typed enums, not string arrays.

---

## Summary of Required Refactors

| Issue | Current | Target |
|-------|---------|--------|
| Exit codes | `&str` keys in JSON | `enum ExitCode` with variants |
| Emit format | String array | `enum EmitFormat` |
| Durability | String array | `enum Durability` |
| Verify profile | String array | `enum VerifyProfile` |
| Categories | Raw strings | `enum BlockerCategory`, `enum ResourceCategory`, `enum CapabilityCategory` |
| Command names | `&str` | `struct CommandName(NonEmptyString)` |
| Flag names | `&str` | `struct FlagName(String)` |
| Line count | 303 | ≤300 |
| Responsibilities | 3 conflated | 3 separate modules |

---

## Recommended Extraction Plan

```
agent_context/
├── mod.rs           # <= 300 lines, orchestrates domain types
├── agent_contract.rs # AgentContract, vocabulary policy types
├── cli_schema.rs    # CliCommand, CliFlag, CliPositional types
├── primitives.rs    # PlannedAgentPrimitives
└── exit_code.rs     # ExitCode enum, BlockerCategory enum
```

Each extracted module would contain **typed domain values**, not raw JSON construction.
