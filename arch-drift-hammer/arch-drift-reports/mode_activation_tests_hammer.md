# ARCHITECTURAL DRIFT HAMMER REPORT

**File**: `crates/vb_cli/src/mode_activation_tests.rs`
**Size**: 966 lines (LIMIT: 300 lines)
**Violation**: 666 lines over limit (322% of allowed size)
**Classification**: GRAVE STRUCTURAL VIOLATION

---

## EXECUTIVE SUMMARY

This file is a monolithic test file that has ballooned to 966 lines through primitive obsession, duplicated Command construction patterns, and failure to use value objects. It should be split into a cohesive test module with supporting domain types.

---

## 1. LINE COUNT VIOLATIONS

| Section | Lines | Problem |
|---------|-------|---------|
| Sections 1-12 | 966 | Entire file is one monolithic block |
| Inline Command construction | ~400 | Massive duplication of Command:: variants |
| Test data arrays | ~150 | Repeated Command arrays without fixtures |

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 RunId as raw String
```rust
// VIOLATION: "1".to_string() is primitive obsession
run_id: "1".to_string(),

// SHOULD BE: RunId type with validation
run_id: RunId::new("1").unwrap(),
```

### 2.2 Step as raw integer
```rust
// VIOLATION: raw u64 for step number
step: 0,

// SHOULD BE: StepIndex or StepNumber value object
step: StepIndex::new(0),
```

### 2.3 ActionId as raw integer
```rust
// VIOLATION: raw u64 for action ID
action_id: 1,

// SHOULD BE: ActionId type
action_id: ActionId::new(1),
```

### 2.4 PathBuf construction scattered everywhere
```rust
// VIOLATION: PathBuf::from() repeated 50+ times
PathBuf::from("/tmp/nonexistent")
PathBuf::from("/data/journal")
PathBuf::from("workflow.yaml")
PathBuf::from("input.bin")

// SHOULD BE: Path constants or domain path types
const TEST_JOURNAL: &str = "/tmp/journal";
const TEST_WORKFLOW: &str = "workflow.yaml";
```

### 2.5 OsString in args() helper
```rust
// VIOLATION: OsString is a primitive; should be CommandArgs type
fn args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|part| OsString::from(*part)).collect()
}

// SHOULD BE: impl Into<CommandArgs> or use Command::test_args()
```

### 2.6 String literals for DurabilityMode
```rust
// VIOLATION: String literals instead of typed DurabilityMode
"strict" => DurabilityMode::Strict,
"journaled" => DurabilityMode::Journaled,
"none" => DurabilityMode::None,

// VIOLATION: regex "[a-z]{1,20}" instead of CommandName type
cmd_name in "[a-z]{1,20}"
```

---

## 3. DDD COHESION VIOLATIONS

### 3.1 No test fixtures or builders
The file constructs Commands inline everywhere instead of using factory methods:

```rust
// CURRENT: Massive inline construction (VIOLATION)
let cmd = Command::Run {
    workflow: PathBuf::from("workflow.yaml"),
    input_bin: PathBuf::from("input.bin"),
    durability: DurabilityMode::Journaled,
    db: Some(PathBuf::from("/tmp/journal")),
    step: None,
    output: OutputFormat::Text,
};
```

**SHOULD BE**:
```rust
// Use a test fixture
let cmd = Command::test_run();
```

### 3.2 Duplicate Command arrays
Sections 6 and 7 duplicate the same Command arrays:
- Pure commands list appears twice (lines ~498-575 and ~722-759)
- Storage commands list appears multiple times
- No shared test fixture module

### 3.3 No ModeActivationMatrix value object
The mode classification logic is tested via `command_mode(&cmd)` but:
- No dedicated `ModeActivationMatrix` type
- No separate test fixtures for mode classification
- The test file is doing both classification AND verification in one place

---

## 4. FILE STRUCTURE VIOLATIONS

### 4.1 Section count: 12 sections in one file
```
SECTION 1: ModeError Enum — Exit Code Mappings (lines 40-174)
SECTION 2: CommandMode Enum (lines 180-208)
SECTION 3: command_mode() — Pure Commands (lines 214-315)
SECTION 4: command_mode() — Storage Commands (lines 321-472)
SECTION 5: command_mode() — Runtime Commands (lines 478-486)
SECTION 6: Mode Activation Matrix Completeness (lines 492-711)
SECTION 7: Pure Mode Invariants (lines 717-780)
SECTION 8: Storage Commands Must Not Be Pure or Runtime or UI (lines 786-822)
SECTION 9: Runtime Commands Must Not Be Pure or Storage or UI (lines 828-843)
SECTION 10: CliExitCode Discriminants (lines 849-871)
SECTION 11: Exit Code Stability (lines 877-886)
SECTION 12: Helper + Proptest Invariants (lines 892-966)
```

**SHOULD BE**:
```
mode_activation_tests/
├── mod.rs           (reexports)
├── mode_error_tests.rs
├── command_mode_tests.rs
├── invariants_tests.rs
└── fixtures.rs      (shared test helpers)
```

### 4.2 Proptest mixed with unit tests
Lines 900-966 mix proptest properties with unit tests. Proptest invariants should be in a separate file with clear property definitions.

---

## 5. SPECIFIC CODE DUPLICATION

### 5.1 Pure command construction repeated 11 times
```rust
// Lines 215-315: 11 separate test functions for each pure command
// Each does identical pattern with different Command variant
fn command_mode_validate_is_pure() { ... }
fn command_mode_verify_is_pure() { ... }
fn command_mode_explain_is_pure() { ... }
// ... 8 more
```

**SHOULD BE**: Parametrized test
```rust
#[test]
fn command_mode_pure_commands() {
    let pure_commands = [Command::Validate{...}, Command::Verify{...}, ...];
    for cmd in pure_commands {
        assert_eq!(command_mode(&cmd), CommandMode::Pure);
    }
}
```

### 5.2 Storage command construction repeated 14 times
Lines 321-472 repeat the same pattern 14 times for storage commands.

---

## 6. PRESCRIPTION

### 6.1 Immediate actions
1. **Split into module**: Create `mode_activation_tests/` directory with:
   - `mod.rs`
   - `mode_error_tests.rs` (ModeError enum tests)
   - `command_mode_tests.rs` (command_mode() tests)
   - `invariant_tests.rs` (mode invariant tests)
   - `fixtures.rs` (shared Command builders)

2. **Create test fixtures** in `fixtures.rs`:
   ```rust
   pub fn test_workflow_path() -> PathBuf { PathBuf::from("workflow.yaml") }
   pub fn test_journal_path() -> PathBuf { PathBuf::from("/tmp/journal") }
   pub fn test_run_id() -> RunId { RunId::new("1").unwrap() }
   pub fn test_pure_commands() -> Vec<Command> { ... }
   pub fn test_storage_commands() -> Vec<Command> { ... }
   ```

3. **Create value objects**:
   - `RunId` wrapping String with validation
   - `StepIndex` wrapping u64
   - `ActionId` wrapping u64

### 6.2 Target line counts per module
| Module | Target Lines |
|--------|-------------|
| mod.rs | 20 |
| mode_error_tests.rs | 150 |
| command_mode_tests.rs | 180 |
| invariant_tests.rs | 120 |
| fixtures.rs | 80 |
| **Total** | **550** (still over limit) |

### 6.3 Further splitting needed
Even 550 lines is still 183% of the 300-line limit. Further split:
- `command_mode_tests.rs` → `pure_mode_tests.rs`, `storage_mode_tests.rs`, `runtime_mode_tests.rs`
- `invariant_tests.rs` → `pure_invariants.rs`, `storage_invariants.rs`, `runtime_invariants.rs`

---

## 7. SUMMARY SCORECARD

| Criterion | Status | Score |
|-----------|--------|-------|
| Line count | VIOLATION | 0/100 |
| Primitive obsession | GRAVE | 5/100 |
| DDD cohesion | FAIL | 20/100 |
| Test duplication | GRAVE | 10/100 |
| File structure | VIOLATION | 15/100 |
| **Overall** | **UNACCEPTABLE** | **10/100** |

---

## 8. MANDATORY REFACTORING

**This file MUST be refactored before acceptance.**

Priority 1 (Critical):
- [ ] Extract Command fixtures to `fixtures.rs`
- [ ] Split into at least 4 module files
- [ ] Introduce `RunId`, `StepIndex`, `ActionId` value objects

Priority 2 (High):
- [ ] Parametrize repeated pure/storage command tests
- [ ] Move proptest to separate file
- [ ] Create `ModeActivationMatrix` domain type

**ARCH-DRIFT ENFORCEMENT: REJECTED**
