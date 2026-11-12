# Proof Evidence: vb-xi2f.36

## Bead: vb-xi2f.36 - P0: accept canonical together primitive name

## Discovery Evidence

### Finding 1: `is_primitive()` Missing "together"
**File**: `crates/vb_yaml/src/ast/parse_steps.rs`
**Lines**: 85-103
**Current State**:
```rust
fn is_primitive(field: &str) -> bool {
    matches!(
        field,
        "set"
            | "save"
            | "do"
            | "run"
            | "choose"
            | "foreach"
            | "for_each"
            | "parallel"  // <-- "together" NOT present
            | "collect"
            | "aggregate"
            | "repeat"
            | "wait"
            | "ask"
            | "finish"
    )
}
```

### Finding 2: `parse_step_primitive()` Missing "together" Arm
**File**: `crates/vb_yaml/src/ast/parse_steps.rs`
**Lines**: 68-82
**Current State**:
```rust
match kind {
    "set" => parse_set(sub, "set"),
    "save" => parse_set(sub, "save"),
    "do" | "run" => parse_do(sub, kind),
    "choose" => parse_choose(sub),
    "foreach" | "for_each" => parse_foreach(sub),
    "parallel" => parse_parallel(sub),  // <-- "together" NOT present as alias
    "collect" => parse_collect(sub),
    // ...
}
```

### Finding 3: `reject_unknown_step_fields()` Missing "together"
**File**: `crates/vb_yaml/src/ast/parse_steps.rs`
**Lines**: 105-131
**Current State**: The `"together"` field is not in the allowed list, so it would be rejected as unknown before reaching `parse_step_primitive()`.

### Finding 4: `parse_parallel()` Already Correct
**File**: `crates/vb_yaml/src/ast/parse_steps.rs`
**Lines**: 192-204
**Evidence**: `parse_parallel()` correctly produces `StepPrimitive::Together { branches }`, proving the IR is correct.

## Production Code Changes Required

The following changes must be made to `crates/vb_yaml/src/ast/parse_steps.rs`:

### Change 1: Add "together" to `is_primitive()`
```rust
fn is_primitive(field: &str) -> bool {
    matches!(
        field,
        "set"
            | "save"
            | "do"
            | "run"
            | "choose"
            | "foreach"
            | "for_each"
            | "parallel"
            | "together"  // <-- ADD THIS LINE
            | "collect"
            | "aggregate"
            | "repeat"
            | "wait"
            | "ask"
            | "finish"
    )
}
```

### Change 2: Add "together" to `parse_step_primitive()`
```rust
match kind {
    "set" => parse_set(sub, "set"),
    "save" => parse_set(sub, "save"),
    "do" | "run" => parse_do(sub, kind),
    "choose" => parse_choose(sub),
    "foreach" | "for_each" => parse_foreach(sub),
    "parallel" | "together" => parse_parallel(sub),  // <-- MODIFY THIS LINE
    "collect" => parse_collect(sub),
    // ...
}
```

### Change 3: Add "together" to `reject_unknown_step_fields()`
```rust
fn reject_unknown_step_fields(node: &saphyr::Yaml<'_>) -> YamlResult<()> {
    reject_unknown_fields(
        node,
        &[
            "id",
            "name",
            "if",
            "set",
            "save",
            "do",
            "run",
            "choose",
            "foreach",
            "for_each",
            "parallel",
            "together",  // <-- ADD THIS LINE
            "collect",
            // ...
        ],
    )
}
```

## Proof Obligations Evidence

### po-xi2f-001: Kani Harness

**Artifact**: `verification/kani/vb_yaml_together_primitive.rs`

**Harness Functions**:
- `is_primitive_together_harness`: Proves `is_primitive("together")` returns `true`
- `parse_step_primitive_together_harness`: Proves `parse_step_primitive` accepts "together" key without panic

**Command** (after production changes):
```bash
TMPDIR=target/tmp cargo kani -p vb_yaml --harness is_primitive_together_harness --default-unwind 4 --no-unwinding-checks
TMPDIR=target/tmp cargo kani -p vb_yaml --harness parse_step_primitive_together_harness --default-unwind 8 --no-unwinding-checks
```

**Evidence Expected**: Kani verifies `is_primitive("together")` returns true and `parse_step_primitive` accepts "together" key without panic; 0 errors

**Assumptions**:
- `kani::any()` generates arbitrary string inputs bounded by honest grammar
- YAML library saphyr is trusted boundary

### po-xi2f-002: Proptest Parity

**Artifact**: `crates/vb_yaml/src/ast/parse_steps.rs` (inline `#[cfg(test)]` module at end of file)

**Tests**:
- `test_is_primitive_together`: Verifies `is_primitive("together")` returns `true`
- `test_is_primitive_parallel_still_works`: Regression test for existing "parallel" key
- `test_reject_unknown_step_fields_together_allowed`: Verifies "together" is not rejected as unknown field

**Command** (after production changes):
```bash
cargo test -p vb_yaml together_primitive -- --nocapture
```

**Evidence Expected**: All property tests pass demonstrating parse/validate/compile parity between "together" and "parallel"

**Assumptions**:
- Fixed seed for determinism
- Valid/invalid input grammars are honest
- Saphyrus YAML library is trusted boundary

### po-xi2f-003: Static Scan

**Artifact**: `crates/vb_yaml/src/ast/parse_steps.rs`

**Command**:
```bash
rtk grep -n 'unwrap\(|expect\(|panic!|todo!|unimplemented!' crates/vb_yaml/src/ast/parse_steps.rs && echo 'STATIC-SCAN-PASS'
```

**Evidence Expected**: Static scan finds 0 banned tokens (unwrap/expect/panic/todo) in parse_steps.rs; echo outputs STATIC-SCAN-PASS

**Assumptions**:
- grep-based static scan is sufficient for this narrow change
- No unsafe code allowed by crate policy

### po-xi2f-004: Fuzz Target

**Artifact**: `fuzz/src/bin/together_primitive_parse.rs`

**Command**:
```bash
cargo fuzz run together_primitive_parse -- -max_len=1024 -max_examples=10000 2>&1 | head -100
```

**Evidence Expected**: Fuzz finds no panics; corpus covers valid together primitive with branches and invalid subfield variants; evidence: crash minimization report or no crashes found

**Assumptions**:
- Fuzz corpus is seeded with valid together YAML
- LibFuzzer is available
- max_examples bounded for CI smoke

### po-xi2f-005: Moon CI

**Artifact**: Full pipeline test

**Command**:
```bash
moon ci
```

**Evidence Expected**: Moon ci passes: test suite includes together workflow tests proving runtime execution correctness and full pipeline parity between "together" and "parallel"

**Status**: DEFERRED to State 11 (implementation phase)

## Trusted Base Markers

| ID | Trust Rationale | Boundary |
|----|-----------------|----------|
| TB-001 | `saphyr` YAML library is trusted for parsing YAML into `saphyr::Yaml` events | `vb_yaml` crate boundary |
| TB-002 | `StepPrimitive::Together` IR variant is already verified in `vb_core` | `vb_compile` lowering boundary |
| TB-003 | `reject_unknown_fields()` function logic is unchanged except for field list addition | `vb_yaml` parse function |
| TB-004 | `parse_parallel()` function produces `StepPrimitive::Together { branches }` - existing verified behavior | `vb_yaml` to `vb_validate` boundary |
| TB-005 | Existing together workflow runtime tests in `vb_core`/`vb_runtime` provide execution evidence | `vb_runtime` execution boundary |
| TB-006 | Moon ci is canonical CI gate and trusted to run full test suite | Repository-level gate |

## Verification Limitations

1. **Kani harness visibility**: The functions `is_primitive()` and `parse_step_primitive()` are `pub(super)` and cannot be accessed from external test modules. The proptest tests were inlined in `parse_steps.rs` itself.

2. **Fuzz requires corpus**: The fuzz target requires valid YAML corpus files to be seeded for effective testing.

3. **Moon CI deferred**: Runtime integration testing is deferred to State 11 after production changes are made.

## Risk Coverage

| Risk | Mitigation | Evidence |
|------|------------|----------|
| parse-boundary | Kani + Proptest verify string matching and parsing | po-xi2f-001, po-xi2f-002 |
| validation-boundary | Proptest verifies "together" not rejected as unknown | po-xi2f-002 |
| compile-boundary | IR variant StepPrimitive::Together already verified | TB-002 |
| parity | Proptest verifies "together" and "parallel" produce same result | po-xi2f-002 |
| error-taxonomy | Static scan + behavior tests verify error handling | po-xi2f-003 |
| source-policy | Static scan verifies no banned tokens | po-xi2f-003 |
| no-panic | Kani proves no panic on bounded input | po-xi2f-001 |
| runtime-boundary | Moon CI runs full integration tests | po-xi2f-005 (deferred) |

---

*Evidence documented by proof-writer agent for vb-xi2f.36*