# Test Plan: Lethal Cross-Cutting Findings C.1–C.25

## Summary

- **Findings**: 25 across 5 categories
- **Behaviors identified**: ~40 testable behaviors across all findings
- **Trophy allocation**: ~25 unit / ~10 integration / ~5 static analysis
- **Fuzz targets**: 8 new + 3 fixes required
- **Proptest invariants**: 6
- **Kani harnesses**: 2
- **Mutation checkpoints**: 12

---

## Finding Categories

| # | Category | Finding |
|---|----------|---------|
| C.1–C.3, C.21–C.25 | Fuzz Infrastructure | Empty stubs, missing targets, discarding results |
| C.4–C.6, C.9–C.10 | CI / moon Infrastructure | Stub tasks, missing coverage, disabled verification |
| C.11–C.12 | Automation Gaps | Density audit, nightly-feature-gate |
| C.13–C.15 | Missing Types / Commands | ShardDirective, evaluate, benchmark args |
| C.7–C.8 | Taint / Helper Coverage | Missing e2e propagation, helper gaps |
| C.16–C.20 | Engineering Rules | crossbeam_channel, unsafe_code, expect/unwrap counts |

---

## Category 1: Fuzz Infrastructure

### C.1 — `property_tests.rs` is EMPTY

**What must exist**
`crates/vb_runtime/src/engine/property_tests.rs` must contain proptest strategies and/or
BDD scenarios exercising the engine's pure functions.

**What test verifies it**
A `#[test]` module inside `property_tests.rs` that invokes all public engine pure functions
with proptest-generated inputs.

**Failure mode**
- `cargo test -p vb_runtime --lib engine::property_tests` returns 0 tests found
- `grep -c fn property_tests.rs` returns 0

**Remediation test**
```rust
// crates/vb_runtime/src/engine/property_tests.rs
#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    proptest! {
        #[test]
        fn execute_node_full_deterministic(a: u8, b: u8) {
            // property: same inputs → same output, no panic
        }
    }
}
```

---

### C.2 — No centralized `property_tests/` directory

**What must exist**
A directory `crates/vb_runtime/src/property_tests/` (or centralized `crates/property_tests/`)
containing shared proptest strategies reused across vb_runtime, vb_core, vb_validate.

**What test verifies it**
`cargo check -p velvet-ballastics-workspace-tests --all-features` succeeds;
no orphaned strategy definitions inside individual `mod.rs` files.

**Failure mode**
Strategies are duplicated in `vb_core/src/workflow/tests.rs`, `vb_runtime/src/engine/tests.rs`,
`vb_validate/src/tests.rs` — violations of DRY in test infrastructure.

**Remediation test**
```bash
# verify no strategy duplication
rg 'proptest::strategy' crates/vb_runtime/src/engine/tests.rs  # should be empty
```

---

### C.3 — No minimization config in `fuzz/Cargo.toml`

**What must exist**
`[package.metadata.cargo-fuzz]` with `sancov_timeout = 60` and `libfuzzer_options = ["-len_control=1"]`
or equivalent minimization directives.

**What test verifies it**
`cargo fuzz build --target <any>` produces a corpus directory with `minimized_*` artifacts
after a `cargo fuzz run --dedup` pass.

**Failure mode**
Fuzz corpus never shrinks; reproducers are maximal (multi-MB) even for single-byte bugs.

**Remediation test**
```bash
# After adding minimization config:
cargo fuzz run taint_propagation -- -minimize_contribs=1
ls fuzz/corpus/taint_propagation/minimized  # must not be empty
```

---

### C.21 — `generated_compare` fuzz is STUB

**What must exist**
`fuzz_lib::fuzz_generated_compare` must:
1. Decode bytes as `WorkflowParts` via postcard
2. Call `vb_core::validate_compiled_workflow(&parts)` and **assert** the validation result
3. Call `vb_core::CompiledWorkflow::try_from_parts(parts)` and **assert** the conversion result
4. Compare the validated workflow's IR against the generated source mapping
5. Assert that the comparison result is stable across two independent decode passes

**What test verifies it**
The fuzz target asserts exact error variants on invalid input and exact equality on valid input.

**Failure mode (current)**
Lines 361–366 drop both results and call `selected_workflow(data)` which is a no-op stub.
```rust
// Current STUB:
let _validated = vb_core::validate_compiled_workflow(&parts);  // DROPPED
let _workflow = vb_core::CompiledWorkflow::try_from_parts(parts); // DROPPED
let _source = selected_workflow(data); // always returns constant
```

**Specific assertions required**
```rust
// After valid decode:
assert!(validated.is_ok(), "valid WorkflowParts must validate");
assert!(workflow.is_ok(), "valid WorkflowParts must convert");
let w1 = workflow.unwrap();
let w2 = vb_core::CompiledWorkflow::try_from_parts(parts).unwrap();
assert_eq!(w1.digest(), w2.digest(), "independent decode must yield same digest");
```

---

### C.22 — `compiled_ir` fuzz is STUB

**What must exist**
`fuzz_lib::fuzz_compiled_ir` must:
1. Decode bytes as `WorkflowParts` via postcard
2. Call `vb_core::CompiledWorkflow::try_from_parts(parts)`
3. **Assert** that `try_from_parts` returns the same digest as the decoded parts
4. **Assert** that the resulting workflow's node count matches the parts
5. **Assert** that all node indices are valid within the workflow

**What test verifies it**
`cargo fuzz run compiled_ir -- -runs=100000` with corpus covering valid and invalid
postcard-encoded `WorkflowParts`.

**Failure mode (current)**
Line 336: `let _workflow = vb_core::CompiledWorkflow::try_from_parts(parts);` — result dropped.
The workflow is never validated; panics in `try_from_parts` are invisible.

**Specific assertions required**
```rust
if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
    let digest_before = parts.digest;
    let node_count_before = parts.nodes.len();
    let workflow = vb_core::CompiledWorkflow::try_from_parts(parts.clone())
        .expect("valid WorkflowParts must not fail conversion");
    assert_eq!(workflow.digest(), digest_before);
    assert_eq!(workflow.node_count(), node_count_before);
    // Assert all slot indices in all nodes are within slot_count
    for node in workflow.nodes() {
        assert!(node.output.map_or(true, |s| s.get() < parts.slot_count));
    }
}
```

---

### C.23 — `ipc_frame` fuzz discards decode results

**What must exist**
`fuzz_lib::fuzz_ipc_frame` (line 192) must assert on the result of `decode_frame_payload`.

**What test verifies it**
The fuzz target must distinguish `Ok` vs `Err` paths and assert on the decoded frame's
field invariants (e.g., `header.payload_len` consistency with actual payload length).

**Failure mode (current)**
Line 213–216:
```rust
match decode_frame_payload(&header, payload) {
    Ok(_) | Err(_) => {}  // ALL RESULTS DISCARDED
}
```
A malformed payload that causes `decode_frame_payload` to return an error variant instead
of `Ok` is invisible to the fuzzer. Any panic inside `decode_frame_payload` is also
invisible because it would crash the fuzzer without an oracle.

**Specific assertions required**
```rust
match decode_frame_payload(&header, payload) {
    Ok(decoded) => {
        assert!(decoded.payload.len() <= usize::from(header.payload_len),
            "decoded payload length must not exceed header payload_len");
        // Assert frame sequence numbers are valid
    }
    Err(e) => {
        // Error variants are acceptable but must be typed
        assert!(matches!(e, IpcError::PayloadTooLarge {..}) ||
                matches!(e, IpcError::FrameChecksumMismatch {..}) ||
                // ... exhaustive error variants
        ));
    }
}
```

---

### C.24 — `expression` fuzz discards eval results

**What must exist**
`fuzz_lib::fuzz_expression` (line 316) must assert on the result of `eval_expr_program`.

**What test verifies it**
The fuzz target must verify:
1. Evaluation returns `Result<(SlotValue, Taint), EvalError>` — never panics
2. Output taint ≥ max input taint (monotonicity)
3. Clean inputs produce Clean output
4. Type errors are returned as typed `EvalError` variants

**Failure mode (current)**
Line 330: `let _result = vb_expr::eval::eval_expr_program(&program, &[], &constants);`
Result is assigned to `_result` but never asserted. A logic error in the evaluator that
produces the wrong `SlotValue` or wrong `Taint` is invisible.

**Specific assertions required**
```rust
let result = vb_expr::eval::eval_expr_program(&program, &[], &constants);
assert!(result.is_ok(), "eval_expr_program must not panic for valid compiled expr");
let (value, taint) = result.unwrap();
// Invariant: if all input slots are Clean, output taint must be Clean
if input_taint == Taint::Clean {
    assert_eq!(taint, Taint::Clean, "clean inputs must produce clean output");
}
// Invariant: output taint must be >= any input taint
assert!(taint.level() >= max_input_taint.level(), "taint must be monotonic");
```

---

### C.25 — `collect_page` pagination fuzz MISSING

**What must exist**
A new fuzz target `fuzz_collect_page_pagination` in `fuzz/src/bin/collect_page_pagination.rs`
that exercises `vb_runtime::primitives::collect::collect_page` with:
- Arbitrary list source values
- Page size boundaries (0, 1, max, overflow)
- Cursor positions across page boundaries
- Non-list collector types (must error)
- Empty list (edge case)

**What test verifies it**
`cargo fuzz run collect_page_pagination -- -runs=50000` with assertions:
1. `collect_page` returns `Result`, never panics
2. Output page count is consistent with list length and page size
3. Each page's item count ≤ page_size
4. Non-list inputs return `RuntimeError::CollectPageNotList` or typed error

**Failure mode**
Missing entirely — `collect_page` has no fuzz coverage despite being a complex
state machine with pagination state.

**Fuzz target specification**
```rust
pub fn fuzz_collect_page_pagination(data: &[u8]) {
    // Derive list length, page size, cursor from data
    // Build a list of SlotValues of derived length
    // Call collect_page with various page_size and cursor values
    // Assert: page_count = ceil(list_len / page_size)
    // Assert: each page item count <= page_size
    // Assert: last page may have fewer items
    // Assert: empty list returns single empty page
}
```

---

## Category 2: CI / Moon Infrastructure

### C.4 — `moon coverage` task is a STUB

**What must exist**
`moon coverage` task must run `cargo llvm-cov test --workspace --all-features --lcov`
producing a full workspace coverage report.

**What test verifies it**
`moon run :coverage` exits 0 and produces `target/llvm-cov/lcov.info` with
>80% line coverage across all workspace crates.

**Failure mode (current)**
Line 281: runs only `-p vb_core --lib --all-features` with a single test filter.
Coverage report covers one package, one test.

**Specific assertions required**
```bash
# After fix:
moon run :coverage
# Verify:
test $(rg 'SF:' target/llvm-cov/lcov.info | wc -l) -gt 1000  # many source files
test $(rg 'LH:' target/llvm-cov/lcov.info | grep -v '0$' | wc -l) -gt 500  # many hit lines
```

---

### C.5 — `vb_core` fails compilation under coverage

**What must exist**
`cargo llvm-cov test -p vb_core --lib --all-features` must compile and run
without errors.

**What test verifies it**
`moon run :coverage` must not fail due to `vb_core` compilation errors.

**Failure mode**
Coverage instrumentation (`-C instrument-coverage`) exposes missing `#[derive(Clone)]`
or `impl` bounds in `vb_core` when compiled with coverage flags.

**Remediation test**
```bash
RUSTFLAGS="-C instrument-coverage" cargo build -p vb_core --all-features  # must compile
```

---

### C.6 — No llvm-cov workspace coverage report

**What must exist**
`target/llvm-cov/` contains `lcov.info` and `coverage.json` with per-crate breakdowns.

**What test verifies it**
A CI gate that fails if any crate falls below threshold:
```bash
cargo llvm-cov report --lcov --output-path target/llvm-cov/lcov.info
# Each crate must have >70% line coverage
```

**Failure mode**
Only `vb_core` coverage is measured. Other crates (`vb_runtime`, `vb_validate`,
`vb_ipc`, `vb_storage`) have unknown coverage.

---

### C.9 — `moon ci miri` runs only 3 tests

**What must exist**
Miri runs all tests marked `#[cfg_attr(miri, test)]` or all tests in `vb_core --lib`.

**What test verifies it**
`moon run :miri` runs >100 miri test iterations (currently hardcoded to 3 specific tests).

**Failure mode (current)**
Lines 262–264: hardcoded to 3 specific test paths:
```
-p vb_core --lib --all-features action::tests::validate_action_outcome_failed_always_succeeds
-p vb_expr --lib --all-features bytecode::tests::constant_folds_addition
-p vb_validate --lib --all-features type_taint::tests::validate_taint_returns_secret_result_leak_for_secret_in_finish
```

**Remediation test**
```bash
# Should run all tests:
TMPDIR=target/miri-tmp cargo miri test -p vb_core --lib --all-features
# Verify: test count > 50 (not 3)
```

---

### C.10 — All `verify-*` tasks have `runInCI: false`

**What must exist**
At minimum `verify-standard` or `verify-fast` must have `runInCI: true`.

**What test verifies it**
`moon ci` runs verification gates. `moon run :verify-standard` exits 0.

**Failure mode**
Formal verification is never run in CI. Verification debt accumulates silently.

**Specific assertions required**
In `.moon/tasks/all.yml`:
```yaml
verify-fast:
  # ...
  options:
    runInCI: true   # was: false
```

---

## Category 3: Automation Gaps

### C.11 — Density audit Tier 0 not automated

**What must exist**
A script `scripts/check-density-audit.sh` that:
1. Enumerates all `crates/*/src/**/*.rs` files
2. Reports files >300 lines
3. Fails CI if any production file exceeds 300 lines

**What test verifies it**
`bash scripts/check-density-audit.sh` returns non-zero if any file >300 lines.

**Failure mode**
Architectural drift goes undetected; files grow beyond 300 lines without enforcement.

**Remediation test**
```bash
# Should fail on a file with 301+ lines
echo "// line 301" >> crates/vb_core/src/errors.rs
bash scripts/check-density-audit.sh
# Must exit non-zero
```

---

### C.12 — `nightly-feature-gate` missing `$attempt` restriction check

**What must exist**
`scripts/check-nightly-features.sh` must detect `feature(try_blocks)` appearing more
than `permitted_attempts = 3` times in the same file (preventing copy-paste abuse).

**What test verifies it**
```bash
# Create a file with 4+ try_blocks usages:
echo '#![feature(try_blocks)]' >> /tmp/test_file.rs
echo '#![feature(try_blocks)]' >> /tmp/test_file.rs
echo '#![feature(try_blocks)]' >> /tmp/test_file.rs
echo '#![feature(try_blocks)]' >> /tmp/test_file.rs
bash scripts/check-nightly-features.sh /tmp/test_file.rs  # must fail
```

**Failure mode**
Copy-pasting `try_blocks` across many files bypasses the "3 attempts per file" rule.

---

## Category 4: Missing Types / Commands

### C.13 — `ShardDirective` enum MISSING

**What must exist**
`crates/vb_runtime/src/shard/directive.rs` with:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardDirective {
    Continue,
    Suspend,
    Cancel,
    Barrier,
}
```

**What test verifies it**
`cargo check -p vb_runtime --all-features` compiles without errors.
A unit test asserts all 4 variants exist and are accessible.

**Failure mode**
All grep results for `ShardDirective` return 0. Any code relying on this type
fails to compile.

**Specific assertions required**
```rust
#[test]
fn shard_directive_variants_exist() {
    assert!(matches!(ShardDirective::Continue, ShardDirective::Continue));
    assert!(matches!(ShardDirective::Suspend, ShardDirective::Suspend));
    assert!(matches!(ShardDirective::Cancel, ShardDirective::Cancel));
    assert!(matches!(ShardDirective::Barrier, ShardDirective::Barrier));
}
```

---

### C.14 — `evaluate` command not implemented

**What must exist**
`crates/vb_cli/src/commands_evaluate.rs` with `cmd_evaluate` function that:
1. Parses `--expression` argument
2. Lexes/parses/compiles the expression using `vb_expr`
3. Evaluates against a provided or empty slot context
4. Prints the result value and taint level
5. Returns `ExitCode::SUCCESS` on valid evaluation, `ExitCode::FAILURE` on parse/eval error

**What test verifies it**
```bash
velvet-ballastics evaluate --expression "1 + 2"
# Expected output: result=3, taint=clean

velvet-ballastics evaluate --expression "load_slot(0)"
# Error if no slot context provided: "ERROR: slot 0 not found"

velvet-ballastics evaluate --expression "load_slot(0)" --slots "slot_0=I64(42)"
# Expected output: result=42, taint=clean
```

**Failure mode**
`cargo build -p vb_cli` succeeds but `velvet-ballastics evaluate` prints
`error: unrecognized command 'evaluate'"`.

---

### C.15 — `benchmark` command lacks `--iterations/--warmup`

**What must exist**
`crates/vb_cli/src/bench.rs` `cmd_bench_run` must accept:
- `--iterations <N>`: run the workflow N times and report mean/median/p95
- `--warmup <N>`: run N warmup iterations before measurement

**What test verifies it**
```bash
velvet-ballastics bench --iterations 100 --warmup 10 workflow.vy
# Must report: mean, median, p95, p99 latencies
# Must run exactly 10 warmup + 100 measured iterations
```

**Failure mode (current)**
Line 9: `cmd_bench_run(workflow: &Path)` accepts only a workflow path.
No iteration or warmup flags exist.

**Specific assertions required**
```rust
#[test]
fn bench_run_accepts_iterations_and_warmup() {
    let result = cmd_bench_run_with_opts(
        workflow_path,
        Iterations::new(100),
        Warmup::new(10),
    );
    assert!(result.is_ok());
    let output = parse_output(result.unwrap());
    assert!(output.contains("mean"));
    assert!(output.contains("p95"));
    assert!(output.iteration_count() == 100);
    assert!(output.warmup_count() == 10);
}
```

---

## Category 5: Taint / Helper Coverage

### C.7 — No end-to-end taint propagation test

**What must exist**
An integration test in `crates/workspace_tests/tests/` that:
1. Creates a workflow with a taint-carrying expression
2. Submits it to the runtime
3. Runs to completion
4. Asserts the result's taint level matches the expected propagation

**What test verifies it**
```rust
#[test]
fn taint_propagates_end_to_end() {
    // Given: a workflow that loads a secret slot and adds 1
    let workflow = build_workflow_with_secret_slot();
    // When: submitted and run to completion
    let result = run_workflow(workflow);
    // Then: result taint == DerivedFromSecret
    assert_eq!(result.taint(), Taint::DerivedFromSecret);
}
```

**Failure mode**
`fuzz_taint_propagation` exists (line 458 in fuzz/src/lib.rs) but no integration-level
test verifies taint propagation through the full runtime loop.

---

### C.8 — 7/10 helpers have edge/error gaps

**What must exist**
Each helper in `crates/vb_runtime/src/shard/helpers.rs` must have:
1. A happy-path unit test
2. An error-path unit test for each error variant it can return
3. A boundary-value test for numeric arguments

**What test verifies it**
`cargo test -p vb_runtime --lib shard::helpers -- --include-ignored` runs all helper tests.

**Current gap inventory** (from finding description):
- `validate_ticket_attempt`: missing `attempt > capacity` error test
- `seed_input_slots`: missing `frame.write_slot_with_taint` error path test
- `validate_action_completion`: missing `StepState` mismatch error test
- Other helpers (up to 7 total): similar gaps

**Specific assertions required**
```rust
// Example for validate_action_completion error path:
#[test]
fn validate_action_completion_returns_error_when_step_not_running() {
    let mut state = RunState { frame, workflow, step: StepIdx::new(0) };
    // Set step state to Completed (not Running)
    state.frame.set_step_state(StepIdx::new(0), StepState::Completed);
    let ticket = ActionTicket { step: StepIdx::new(0), action: action_id, attempt: 1, capacity: 3 };
    let result = validate_action_completion(&state, ticket);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RuntimeError::InvalidActionCompletion));
}
```

---

## Category 6: Engineering Rules

### C.16 — `crossbeam_channel` used (Section 50 violation)

**What must exist**
`crates/vb_ipc/src/ingress.rs` and `crates/vb_ipc/src/tests.rs` must use
`std::sync::mpsc` or `flume` instead of `crossbeam_channel`.

**What test verifies it**
```bash
rg 'crossbeam_channel' crates/vb_ipc/src/  # must return 0 results
cargo check -p vb_ipc --all-features  # must still compile
```

**Failure mode**
`crossbeam_channel` is in the dependencies of `vb_ipc` despite Section 50 of the
Engineering Rules prohibiting it. A `cargo deny` check would catch this.

**Remediation test**
```bash
# Must fail before fix:
rg 'crossbeam_channel' crates/vb_ipc/src/  # returns lines
# After fix:
rg 'crossbeam_channel' crates/vb_ipc/src/  # returns empty
```

---

### C.17 — 4 crates missing `#![forbid(unsafe_code)]`

**What must exist**
These crates must have `#![forbid(unsafe_code)]` at the top of their `src/lib.rs`:
- `crates/vb_proof_kernels/src/lib.rs` — currently absent
- `crates/vb_benchmark/src/lib.rs` — currently absent
- `crates/xtask/src/lib.rs`
- `crates/workspace_tests/src/lib.rs`

**What test verifies it**
```bash
for crate in vb_proof_kernels vb_benchmark xtask workspace_tests; do
  first_line=$(head -1 crates/$crate/src/lib.rs)
  if [ "$first_line" != '#![forbid(unsafe_code)]' ]; then
    echo "FAIL: $crate missing #![forbid(unsafe_code)]"
    exit 1
  fi
done
```

**Failure mode**
`cargo check -p vb_proof_kernels` succeeds but the crate does not enforce
no `unsafe` at compile time. Any future `unsafe` code in these crates will not
be caught by the linter.

---

### C.18 — 7 crates have 418+ `expect()` calls

**What must exist**
Each of the 7 crates must have `expect()` calls reduced to <418 via:
1. Replacing `expect()` with `expect()` + documented rationale, OR
2. Converting to proper error propagation with `?`

**What test verifies it**
```bash
for crate in $(cat /tmp/high_expect_crates.txt); do
  count=$(rg 'expect\(\)' crates/$crate/src/ | wc -l)
  if [ $count -ge 418 ]; then
    echo "FAIL: $crate has $count expect() calls (threshold: 418)"
    exit 1
  fi
done
```

**Failure mode**
High `expect()` density indicates likely hidden panics. `clippy::expect_used` linter
can catch them but they are currently allowed by the lint configuration.

---

### C.19 — 7 crates have 518+ `unwrap()` calls

**What must exist**
Same as C.18 but for `unwrap()`.

**What test verifies it**
```bash
for crate in $(cat /tmp/high_unwrap_crates.txt); do
  count=$(rg '\.unwrap\(\)' crates/$crate/src/ | wc -l)
  if [ $count -ge 518 ]; then
    echo "FAIL: $crate has $count unwrap() calls (threshold: 518)"
    exit 1
  fi
done
```

---

### C.20 — 5 crates CLEAN (reference)

**What must exist**
5 crates serve as reference for low expect/unwrap density:
`vb_doc`, `vb_ui_snapshot`, `vb_ui_makepad`, `vb_ui_model`, `vb_boundary_inventory`.

**What test verifies it**
```bash
for crate in vb_doc vb_ui_snapshot vb_ui_makepad vb_ui_model vb_boundary_inventory; do
  expect_count=$(rg 'expect\(\)' crates/$crate/src/ | wc -l)
  unwrap_count=$(rg '\.unwrap\(\)' crates/$crate/src/ | wc -l)
  echo "$crate: expect=$expect_count unwrap=$unwrap_count"
done
# All should show counts < 100
```

---

## Open Questions

1. **C.13**: What is the intended behavior of `ShardDirective::Barrier`? Should it block
   all other shards or only specific resource types?
2. **C.14**: Should `evaluate` support `--slots` to inject arbitrary slot values, or
   only operate on an empty context?
3. **C.18/C.19**: What are the exact 7 crate names with high expect/unwrap counts?
   The finding description references "helper coverage plan" which was not provided.
4. **C.8**: Which 3 helpers out of 10 are fully tested? Need the helper coverage plan
   document to complete this finding's test specification.
5. **C.5**: What is the exact compilation error when `vb_core` is compiled with coverage?
   The root cause is needed to write a regression test.
