# ARCHITECTURAL DRIFT REPORT: journal.rs

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_cli/src/args/tests/journal.rs`
**Line Count:** 471 (VIOLATION: exceeds 300-line limit by 157%)
**Status:** MUST REFACTOR

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 471 | 300 | **VIOLATION** |
| Excess | 171 lines | 0 | **-57% over limit** |

---

## 2. TEST RESPONSIBILITY MAP

The `journal.rs` file tests argument parsing for **10 distinct commands**:

| Command | Test Count | Lines | Responsibility |
|---------|------------|-------|----------------|
| `inspect` | 3 | 8-66 | Run inspection with optional YAML output |
| `events` | 4 | 69-146 | Event listing with status/limit filters |
| `replay` | 1 | 149-164 | Replay a specific run |
| `trace` | 5 | 167-275 | Trace filtering by step/action/status/seq |
| `retry` | 1 | 278-293 | Retry a failed run |
| `resume` | 1 | 296-311 | Resume an interrupted run |
| `incident` | 1 | 314-329 | File an incident report |
| `answer` | 2 | 332-379 | Provide step answer with value file |
| `diff` | 4 | 382-442 | Compare two runs |
| `doctor` | 2 | 445-471 | System health check (stateless) |

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 `run_id: String` — No Domain Validation

**Location:** All 10 commands use raw `String` for run identifiers.

**Problem:** `run_id` is stored and compared as raw `String`. There is no:
- Format validation (what constitutes a valid run ID?)
- Length bounds
- Type distinction between run IDs and other string fields

**Evidence:**
```rust
// journal.rs:17 - run_id is just a String
assert_eq!(run_id, "42");

// journal.rs:85 - mixed usage: "run-1" as run_id
assert_eq!(run_id, "run-1");

// args.rs:1107 - RunDbArgs holds run_id as raw String
struct RunDbArgs {
    run_id: String,  // <-- Primitive obsession
    db: PathBuf,
    output: OutputFormat,
}
```

**Scott Wlaschin Violation:** A run ID is a **value object**, not a string. It should be:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunId(String);

impl RunId {
    pub fn new(s: &str) -> Result<Self, InvalidRunId> { ... }
}
```

### 3.2 `step: u16` — Unbounded Integer

**Location:** `Answer` command in args.rs:179

**Problem:** Raw `u16` has no application-level bounds. A step number of `65535` is technically valid by type, but may be nonsensical in context.

**Evidence:**
```rust
// args.rs:179
Answer {
    run_id: String,
    step: u16,  // <-- No StepId newtype
    value_file: PathBuf,
    db: PathBuf,
    output: OutputFormat,
}

// journal.rs:372 - test uses raw value
assert_eq!(step, 3);
```

**Fix:** Introduce `StepId(u16)` newtype with bounds checking.

### 3.3 `limit: Option<i64>` — Wrong Signedness + No Bounds

**Location:** `Events` command, args.rs:141

**Problem:** 
1. `i64` allows negative limits which make no sense
2. No maximum bound enforcement
3. `limit` field in `TraceFilters` uses `usize` (correct) but `Events` uses `i64` (incorrect)

**Evidence:**
```rust
// args.rs:141 - Events uses i64 (wrong)
Events {
    run_id: String,
    db: PathBuf,
    output: OutputFormat,
    status: Option<EventStatus>,
    limit: Option<i64>,  // <-- Should be usize
}

// journal.rs:125 - test passes negative? No, but type allows it
assert_eq!(limit, Some(100));
```

### 3.4 `since_seq`, `until_seq: u64` — No Range Contract

**Location:** `TraceFilters` struct (imported from `commands_journal`)

**Problem:** Sequence numbers are raw `u64` with no invariant that `since_seq <= until_seq`.

**Evidence:**
```rust
// journal.rs:179-180
assert_eq!(filters.since_seq, Some(10));
assert_eq!(filters.until_seq, Some(20));
// No test verifies the invariant that since_seq <= until_seq
```

### 3.5 `diff` Command — Two Run IDs Without Pairing Type

**Location:** `Diff` command, args.rs:188-193

**Problem:** Two run IDs `run_a` and `run_b` are just Strings with no relationship enforced.

```rust
// args.rs:188-193
Diff {
    run_a: String,  // <-- Two unrelated Strings
    run_b: String,  // <-- No RunPair or similar type
    db: PathBuf,
    output: OutputFormat,
}
```

**Scott Wlaschin Violation:** This is a **tuple** data structure masquerading as a command. The two run IDs have an implicit relationship (they must be different, they belong to the same comparison) that should be made explicit.

---

## 4. TEST STRUCTURE VIOLATIONS

### 4.1 Repetitive Test Boilerplate

Every test follows an identical 12-line pattern:

```rust
#[test]
fn parse_<command>_<scenario>() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "<command>",
        // ... args ...
    ]));
    if let Ok(Command::<Command> { /* fields */ }) = parsed {
        assert_eq!(field1, expected1);
        assert_eq!(field2, expected2);
        // ...
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}
```

**Problem:** 24 tests × ~12 lines = ~288 lines of pure boilerplate.

**Recommendation:** Use table-driven tests or a builder pattern:

```rust
fn assert_command_parses(cmd: &str, args: &[&str], expected: Command) {
    let parsed = parse_args(&args(&["velvet-ballistics", cmd]));
    assert_eq!(parsed.unwrap(), expected);
}

#[test]
fn parse_inspect_requires_run_id_and_db() {
    assert_command_parses("inspect", &["42", "--db", "test-db"], 
        Command::Inspect { 
            run_id: RunId("42".into()),  // Using newtype
            db: PathBuf::from("test-db"),
            output: OutputFormat::Text,
        }
    );
}
```

### 4.2 Anti-Pattern: `if let ... else { assert!(parsed.is_ok() ... ) }`

**Location:** Every "happy path" test in journal.rs

**Problem:** This pattern swallows the actual error and provides a misleading assertion message.

```rust
// journal.rs:20-22 - WRONG
if let Ok(Command::Inspect { run_id, db, output }) = parsed {
    // assertions
} else {
    assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
}
```

**Correct pattern:**
```rust
let parsed = parse_args(&args(&[...]));
assert!(parsed.is_ok(), "unexpected error: {parsed:?}");
let Ok(Command::Inspect { run_id, db, output }) = parsed else {
    panic!("wrong variant: {:?}", parsed);
};
assert_eq!(run_id, "42");
```

---

## 5. ERROR TYPE DISPERSION

The `ParseError` enum in `error.rs` has **19 variants**, many specific to individual commands. The journal.rs tests reference only a subset:

| Error Variant | Source File | Used in journal.rs? |
|---------------|-------------|---------------------|
| `MissingArgument` | args.rs | No (but used in impl) |
| `UnknownEventStatus` | args.rs | **Yes** (line 142) |
| `InvalidTraceArgument` | args.rs | **Yes** (lines 237, 254, 271) |
| `InvalidStep` | args.rs | **Yes** (line 344) |

**Problem:** The error types are defined in `args.rs` (the main parsing module), but the tests also import from `error.rs` (a separate file) which has duplicate definitions. This is confusing and may indicate drift between error handling and parsing logic.

**Evidence:**
```rust
// error.rs has ParseError with 19 variants (line 5-23)
// args.rs re-defines ParseError with more variants (line 289-316)
```

---

## 6. MISSING TEST COVERAGE

### 6.1 Diff Command Missing Tests
- No test for `--emit yaml` with diff (lines 408-424)
- No test verifying `run_a != run_b` invariant
- No test for missing `run_b` (partial validation)

### 6.2 Trace Filters Missing Tests
- No test for `since_seq > until_seq` error case
- No test for `limit = 0` behavior
- No test for `step` overflow beyond u16

### 6.3 Answer Command Missing Tests
- No test for `step = 0`
- No test for missing `--value-file`
- No test for missing `--db`

---

## 7. REFACTOR PRESCRIPTIONS

### Priority 1: Split File (CRITICAL)

Split `journal.rs` into one file per command:

```
tests/
  journal/
    mod.rs           (module + helpers)
    inspect.rs       (~40 lines)
    events.rs        (~80 lines)
    replay.rs        (~20 lines)
    trace.rs         (~110 lines)
    retry.rs         (~20 lines)
    resume.rs        (~20 lines)
    incident.rs      (~20 lines)
    answer.rs        (~50 lines)
    diff.rs          (~60 lines)
    doctor.rs        (~30 lines)
```

Each file under 100 lines.

### Priority 2: Introduce RunId NewType

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunId(String);

impl RunId {
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        if s.is_empty() {
            Err(ParseError::InvalidRunId("run id cannot be empty".into()))
        } else {
            Ok(Self(s.into()))
        }
    }
}
```

Replace `run_id: String` with `run_id: RunId` in:
- `RunDbArgs`
- `Command::Inspect`
- `Command::Events`
- `Command::Replay`
- `Command::Trace`
- `Command::Retry`
- `Command::Resume`
- `Command::Incident`
- `Command::Answer`

### Priority 3: Fix limit Type

Change `Events.limit: Option<i64>` to `Option<usize>` and add bounds validation.

### Priority 4: Introduce DiffPair Type

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffPair(pub RunId, pub RunId);

impl DiffPair {
    pub fn new(a: RunId, b: RunId) -> Result<Self, DiffPairError> {
        if a == b {
            Err(DiffPairError::SameRunIds)
        } else {
            Ok(Self(a, b))
        }
    }
}
```

### Priority 5: Add TraceSeqRange Type

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceSeqRange {
    pub since: u64,
    pub until: u64,
}

impl TraceSeqRange {
    pub fn new(since: u64, until: u64) -> Result<Self, TraceSeqRangeError> {
        if since > until {
            Err(TraceSeqRangeError::InvertedRange { since, until })
        } else {
            Ok(Self { since, until })
        }
    }
}
```

---

## 8. SUMMARY

| Issue | Severity | Effort |
|-------|----------|--------|
| 471 lines (>300 limit) | **CRITICAL** | High |
| `run_id: String` primitive obsession | **HIGH** | Medium |
| `step: u16` no bounds | **HIGH** | Low |
| `limit: i64` wrong signedness | **MEDIUM** | Low |
| Diff command lacks pairing type | **MEDIUM** | Medium |
| Test boilerplate repetition | **LOW** | Medium |
| TraceFilters lacks range invariant | **LOW** | Medium |

**Total violations:** 7 critical/high issues requiring refactoring.

---

*Report generated by arch-drift-hammer*
*Workspace: velvet-ballistics*
*Date: 2026-05-29*
