# Test Plan: vb-fzx7 — Core Orchestrator Benchmark Suite

## 1. Behavior Inventory

### 1.1 Evidence Gate Behaviors

| ID | Subject | Action | Outcome | Condition |
|----|---------|--------|---------|-----------|
| EG-001 | `check_evidence_gate` | returns `Ok(())` | `Ok(())` | `baseline_us` and `result_us` present, result within threshold |
| EG-002 | `check_evidence_gate` | returns `Err(MissingBaseline)` | `Err(EvidenceError::MissingBaseline)` | `baseline_us` absent |
| EG-003 | `check_evidence_gate` | returns `Err(MissingResult)` | `Err(EvidenceError::MissingResult)` | `result_us` absent |
| EG-004 | `check_evidence_gate` | returns `Err(MissingEnvironment)` | `Err(EvidenceError::MissingEnvironment)` | `environment` empty |
| EG-005 | `check_evidence_gate` | returns `Err(MissingCommand)` | `Err(EvidenceError::MissingCommand)` | `command` empty |
| EG-006 | `check_evidence_gate` | returns `Err(MissingCommit)` | `Err(EvidenceError::MissingCommit)` | `commit_hash` empty |
| EG-007 | `check_evidence_gate` | returns `Err(RegressionDetected)` | `Err(EvidenceError::RegressionDetected { benchmark, delta })` | `result_us > baseline_us + threshold_pct * baseline_us / 100` |
| EG-008 | `check_evidence_gate` | returns `Err(EmptyBudget)` | `Err(EvidenceError::EmptyBudget)` | `budget_us == 0` |

### 1.2 Metadata Capture Behaviors

| ID | Subject | Action | Outcome | Condition |
|----|---------|--------|---------|-----------|
| MC-001 | `capture_metadata` | returns `BenchmarkMetadata` | `BenchmarkMetadata` with all fields populated | all arguments valid |
| MC-002 | `capture_metadata` | returns `commit_hash` | non-empty ASCII hex string | `commit_hash` argument is valid |
| MC-003 | `capture_metadata` | panics with `"commit_hash must be non-empty ASCII hex"` | `panic!("commit_hash must be non-empty ASCII hex")` | `commit_hash` is empty |
| MC-004 | `budget_utilization_percent` | computes utilization | `u128` between 0 and 10000 | `elapsed` within budget |
| MC-005 | `budget_utilization_percent` | returns `u128::MAX` | `u128::MAX` | `budget_us == 0` |
| MC-006 | `latency_within_budget` | returns `true` | `true` | `elapsed.as_micros() <= budget_us` and `budget_us > 0` |
| MC-007 | `latency_within_budget` | returns `false` | `false` | `budget_us == 0` or `elapsed.as_micros() > budget_us` |
| MC-008 | `result_exceeds_threshold` | returns `true` | `true` | `result > baseline + threshold_pct * baseline / 100` |
| MC-009 | `result_exceeds_threshold` | returns `false` | `false` | `result <= baseline + threshold_pct * baseline / 100` |
| MC-010 | `baseline_within_budget` | returns `true` | `true` | `baseline.as_micros() <= budget_us` |

### 1.3 Benchmark Group Behaviors

| ID | Subject | Action | Outcome | Condition |
|----|---------|--------|---------|-----------|
| BG-001 | `yaml_parse_benches` | registers benchmark group | `Ok(())` | Criterion valid, fixtures exist |
| BG-002 | `yaml_validate_benches` | registers benchmark group | `Ok(())` | Criterion valid, workflow compiles |
| BG-003 | `yaml_compile_benches` | registers benchmark group | `Ok(())` | Criterion valid, fixtures valid |
| BG-004 | `runtime_step_benches` | registers benchmark group | `Ok(())` | Criterion valid, workflow compiled |
| BG-005 | `runtime_primitive_benches` | registers benchmark group | `Ok(())` | Criterion valid, expressions valid |
| BG-006 | `ipc_frame_benches` | registers benchmark group | `Ok(())` | Criterion valid, frames valid |
| BG-007 | `ipc_backpressure_benches` | registers benchmark group | `Ok(())` | Criterion valid, queue bounded |
| BG-008 | `storage_journal_write_benches` | registers benchmark group | `Ok(())` | Criterion valid, journal opens |
| BG-009 | `storage_journal_replay_benches` | registers benchmark group | `Ok(())` | Criterion valid, events exist |
| BG-010 | `recovery_hydration_benches` | registers benchmark group | `Ok(())` | Criterion valid, recovery valid |

### 1.4 Error Variant Behaviors (EvidenceError)

| ID | Subject | Action | Outcome | Condition |
|----|---------|--------|---------|-----------|
| EV-001 | `EvidenceError::MissingBaseline` | displays message | `"missing baseline measurement"` | always |
| EV-002 | `EvidenceError::MissingResult` | displays message | `"missing result measurement"` | always |
| EV-003 | `EvidenceError::MissingEnvironment` | displays message | `"missing environment"` | always |
| EV-004 | `EvidenceError::MissingCommand` | displays message | `"missing command"` | always |
| EV-005 | `EvidenceError::MissingCommit` | displays message | `"missing commit hash"` | always |
| EV-006 | `EvidenceError::RegressionDetected` | displays message | `"regression detected: {benchmark} delta={delta}` | always |
| EV-007 | `EvidenceError::EmptyBudget` | displays message | `"budget not configured"` | always |

### 1.5 Error Variant Behaviors (Benchmark Errors)

| ID | Subject | Action | Outcome | Condition |
|----|---------|--------|---------|-----------|
| BE-001 | `YamlBenchmarkError::ParseFailure` | wraps `YamlError` | `"YAML parse failed: {inner}"` | YAML parse fails |
| BE-002 | `YamlBenchmarkError::ValidationFailure` | wraps error | `"workflow validation failed: {inner}"` | validation fails |
| BE-003 | `StorageBenchmarkError::JournalOpenFailure` | wraps `JournalError` | `"journal open failed: {inner}"` | journal open fails |
| BE-004 | `StorageBenchmarkError::AppendFailure` | wraps `JournalError` | `"journal append failed: {inner}"` | append fails |
| BE-005 | `IpcBenchmarkError::EncodeFailure` | wraps `IpcError` | `"frame encode failed: {inner}"` | encode fails |
| BE-006 | `IpcBenchmarkError::DecodeFailure` | wraps `IpcError` | `"frame decode failed: {inner}"` | decode fails |
| BE-007 | `RecoveryBenchmarkError::HydrationFailure` | wraps error | `"recovery hydration failed: {inner}"` | hydration returns None or error |

---

## 2. Trophy Allocation

| Layer | Allocation | Rationale |
|-------|------------|-----------|
| **Static** (clippy, types) | ~5% | `INV-003` no UI dep via `cargo tree -i vb_ui`, format checks |
| **Unit** (`#[cfg(test)]`) | ~25% | Evidence gate logic, metadata capture, budget arithmetic, error variants |
| **Integration** (`tests/`) | ~55% | All 10 benchmark groups, fixture validation, error path handling |
| **E2E** (`cargo bench`) | ~10% | Full criterion runs, regression gate, evidence JSON emission |
| **Fuzz** (`cargo-fuzz`) | ~5% | YAML parse path, malformed input |

### Trophy Distribution Justification

- **Integration > Unit**: Benchmark groups are inherently integration tests — they exercise real kernel APIs with real fixtures.
- **Unit for Calc Layer**: Pure arithmetic (`budget_utilization_percent`, `latency_within_budget`, `result_exceeds_threshold`) belongs in unit tests.
- **Static for INV-003**: No UI dependency is a `cargo tree` check, not a runtime test.
- **Fuzz for YAML**: Malformed YAML is the primary fuzz vector for parse benchmarks.

---

## 3. BDD Scenarios

### 3.1 Evidence Gate Scenarios

```gherkin
Feature: Evidence Gate Acceptance

  Scenario: Evidence gate accepts complete metadata within threshold
    Given a BenchmarkMetadata with baseline_us=100_000, result_us=105_000, budget_us=200_000
    And environment="linux-x86_64"
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456"
    And threshold_pct=20
    When check_evidence_gate is called
    Then the result is Ok(())

  Scenario: Evidence gate rejects missing baseline
    Given a BenchmarkMetadata with result_us=105_000 but no baseline_us
    And budget_us=200_000
    And environment="linux-x86_64"
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456"
    When check_evidence_gate is called
    Then the result is Err(EvidenceError::MissingBaseline)

  Scenario: Evidence gate rejects missing result
    Given a BenchmarkMetadata with baseline_us=100_000 but no result_us
    And budget_us=200_000
    And environment="linux-x86_64"
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456"
    When check_evidence_gate is called
    Then the result is Err(EvidenceError::MissingResult)

  Scenario: Evidence gate rejects missing environment
    Given a BenchmarkMetadata with baseline_us=100_000, result_us=105_000
    But environment is empty
    And budget_us=200_000
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456"
    When check_evidence_gate is called
    Then the result is Err(EvidenceError::MissingEnvironment)

  Scenario: Evidence gate rejects missing command
    Given a BenchmarkMetadata with baseline_us=100_000, result_us=105_000
    And environment="linux-x86_64"
    But command is empty
    And budget_us=200_000
    And commit_hash="abc123def456"
    When check_evidence_gate is called
    Then the result is Err(EvidenceError::MissingCommand)

  Scenario: Evidence gate rejects missing commit hash
    Given a BenchmarkMetadata with baseline_us=100_000, result_us=105_000
    And environment="linux-x86_64"
    And command="cargo bench yaml_parse"
    But commit_hash is empty
    And budget_us=200_000
    When check_evidence_gate is called
    Then the result is Err(EvidenceError::MissingCommit)

  Scenario: Evidence gate detects regression above threshold
    Given a BenchmarkMetadata with baseline_us=100_000, result_us=130_000
    And environment="linux-x86_64"
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456"
    And budget_us=200_000
    And threshold_pct=20
    When check_evidence_gate is called
    Then the result is Err(EvidenceError::RegressionDetected { benchmark: "yaml_parse", delta: 30000 })

  Scenario: Evidence gate accepts regression within threshold
    Given a BenchmarkMetadata with baseline_us=100_000, result_us=115_000
    And environment="linux-x86_64"
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456"
    And budget_us=200_000
    And threshold_pct=20
    When check_evidence_gate is called
    Then the result is Ok(())

  Scenario: Evidence gate rejects zero budget
    Given a BenchmarkMetadata with baseline_us=100_000, result_us=105_000
    And environment="linux-x86_64"
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456"
    And budget_us=0
    When check_evidence_gate is called
    Then the result is Err(EvidenceError::EmptyBudget)
```

### 3.2 Metadata Capture Scenarios

```gherkin
Feature: Benchmark Metadata Capture

  Scenario: capture_metadata produces complete metadata record
    Given name="yaml_parse_small"
    And baseline=Some(Duration::from_micros(50000))
    And result=Duration::from_micros(55000)
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456789"
    And environment="linux-x86_64"
    And budget_us=100_000
    When capture_metadata is called
    Then the returned BenchmarkMetadata has name="yaml_parse_small"
    And baseline_us=50000
    And result_us=55000
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456789"
    And environment="linux-x86_64"
    And budget_us=100_000

  Scenario: capture_metadata accepts None baseline for new benchmarks
    Given name="yaml_parse_1mb"
    And baseline=None
    And result=Duration::from_micros(5_000_000)
    And command="cargo bench yaml_parse"
    And commit_hash="abc123def456789"
    And environment="linux-x86_64"
    And budget_us=10_000_000
    When capture_metadata is called
    Then the returned BenchmarkMetadata has baseline_us=0 and result_us=5_000_000

  Scenario: capture_metadata panics when commit_hash is empty
    Given name="yaml_parse_small"
    And baseline=Some(Duration::from_micros(50000))
    And result=Duration::from_micros(55000)
    And command="cargo bench yaml_parse"
    And commit_hash=""
    And environment="linux-x86_64"
    And budget_us=100_000
    When capture_metadata is called
    Then the process panics with message "commit_hash must be non-empty ASCII hex"

  Scenario: latency_within_budget returns true for within-budget elapsed
    Given elapsed=Duration::from_micros(50000)
    And budget_us=100_000
    When latency_within_budget is called
    Then the result is true

  Scenario: latency_within_budget returns false for over-budget elapsed
    Given elapsed=Duration::from_micros(150000)
    And budget_us=100_000
    When latency_within_budget is called
    Then the result is false

  Scenario: latency_within_budget returns false for zero budget
    Given elapsed=Duration::from_micros(50000)
    And budget_us=0
    When latency_within_budget is called
    Then the result is false

  Scenario: budget_utilization_percent computes correct percentage
    Given elapsed=Duration::from_micros(75000)
    And budget_us=100_000
    When budget_utilization_percent is called
    Then the result is 75

  Scenario: budget_utilization_percent returns MAX for zero budget
    Given elapsed=Duration::from_micros(75000)
    And budget_us=0
    When budget_utilization_percent is called
    Then the result is u128::MAX

  Scenario: result_exceeds_threshold returns true for significant regression
    Given result=Duration::from_micros(130000)
    And baseline=Duration::from_micros(100000)
    And threshold_pct=20
    When result_exceeds_threshold is called
    Then the result is true

  Scenario: result_exceeds_threshold returns false for within-threshold
    Given result=Duration::from_micros(115000)
    And baseline=Duration::from_micros(100000)
    And threshold_pct=20
    When result_exceeds_threshold is called
    Then the result is false

  Scenario: baseline_within_budget returns true when baseline is under budget
    Given baseline=Duration::from_micros(80000)
    And budget_us=100_000
    When baseline_within_budget is called
    Then the result is true
```

### 3.3 Benchmark Group Scenarios

```gherkin
Feature: YAML Parse Benchmark Group

  Scenario: yaml_parse_benches registers small workflow benchmark
    Given a valid Criterion runner
    And SMALL_WORKFLOW fixture
    When yaml_parse_benches is called with the runner
    Then the benchmark group "yaml_parse" is registered
    And includes input "parse_yaml_small" with SMALL_WORKFLOW
    And returns Ok(())

  Scenario: yaml_parse_benches registers 1MB workflow benchmark
    Given a valid Criterion runner
    And generated 1MB workflow fixture
    When yaml_parse_benches is called with the runner
    Then the benchmark group "yaml_parse" is registered
    And includes input "parse_yaml_1mb" with 1MB workflow
    And returns Ok(())

Feature: YAML Validate Benchmark Group

  Scenario: yaml_validate_benches registers small workflow validation
    Given a valid Criterion runner
    And SMALL_WORKFLOW fixture
    When yaml_validate_benches is called with the runner
    Then the benchmark group "yaml_validate" is registered
    And bench "validate_minimal" exists
    And returns Ok(())

Feature: YAML Compile Benchmark Group

  Scenario: yaml_compile_benches registers 1000-step workflow compile
    Given a valid Criterion runner
    And 1000-step workflow fixture
    When yaml_compile_benches is called with the runner
    Then the benchmark group "yaml_compile" is registered
    And includes "compile_ir_1000_steps" benchmark
    And returns Ok(())

Feature: Runtime Step Benchmark Group

  Scenario: runtime_step_benches registers save-chain workflow
    Given a valid Criterion runner
    And compiled save-chain workflow
    When runtime_step_benches is called with the runner
    Then the benchmark group "runtime_step" is registered
    And includes "runtime_save_chain" benchmark
    And returns Ok(())

Feature: Runtime Primitive Benchmark Group

  Scenario: runtime_primitive_benches registers Add expression benchmark
    Given a valid Criterion runner
    When runtime_primitive_benches is called with the runner
    Then the benchmark group "runtime_primitive" is registered
    And includes "runtime_expr_add" benchmark
    And returns Ok(())

Feature: IPC Frame Benchmark Group

  Scenario: ipc_frame_benches registers encode/decode benchmarks
    Given a valid Criterion runner
    When ipc_frame_benches is called with the runner
    Then the benchmark group "ipc_frame" is registered
    And includes "ipc_encode_1kb" and "ipc_decode_1kb" benchmarks
    And returns Ok(())

Feature: IPC Backpressure Benchmark Group

  Scenario: ipc_backpressure_benches registers bounded queue benchmark
    Given a valid Criterion runner
    When ipc_backpressure_benches is called with the runner
    Then the benchmark group "ipc_backpressure" is registered
    And includes "ipc_backpressure_queue_depth_100" benchmark
    And returns Ok(())

Feature: Storage Journal Write Benchmark Group

  Scenario: storage_journal_write_benches registers 100-event append
    Given a valid Criterion runner
    And temporary journal path
    When storage_journal_write_benches is called with the runner
    Then the benchmark group "storage_journal_write" is registered
    And includes "storage_append_100_events" benchmark
    And returns Ok(())

Feature: Storage Journal Replay Benchmark Group

  Scenario: storage_journal_replay_benches registers replay benchmark
    Given a valid Criterion runner
    And journal with 1000 pre-populated events
    When storage_journal_replay_benches is called with the runner
    Then the benchmark group "storage_journal_replay" is registered
    And includes "storage_replay_1000_events" benchmark
    And returns Ok(())

Feature: Recovery Hydration Benchmark Group

  Scenario: recovery_hydration_benches registers hydration benchmark
    Given a valid Criterion runner
    And recovery event sequence
    When recovery_hydration_benches is called with the runner
    Then the benchmark group "recovery_hydration" is registered
    And includes "recovery_hydrate_1000_events" benchmark
    And returns Ok(())
```

### 3.4 Error Variant Scenarios

```gherkin
Feature: YAML Benchmark Error Handling

  Scenario: YAML parse failure is wrapped as ParseFailure
    Given malformed YAML input
    When parse benchmark executes
    Then the error is YamlBenchmarkError::ParseFailure
    And the inner error is accessible

  Scenario: YAML validation failure is wrapped as ValidationFailure
    Given invalid workflow YAML
    When validate benchmark executes
    Then the error is YamlBenchmarkError::ValidationFailure
    And the error message matches "workflow validation failed: .+"
    And the inner error is accessible

Feature: Storage Benchmark Error Handling

  Scenario: Journal open failure is wrapped as JournalOpenFailure
    Given invalid journal path
    When storage benchmark setup executes
    Then the error is StorageBenchmarkError::JournalOpenFailure
    And the inner JournalError is accessible

  Scenario: Append failure is wrapped as AppendFailure
    Given disk full condition
    When storage benchmark append executes
    Then the error is StorageBenchmarkError::AppendFailure
    And the inner JournalError is accessible

Feature: IPC Benchmark Error Handling

  Scenario: Encode failure is wrapped as EncodeFailure
    Given invalid frame payload
    When IPC encode benchmark executes
    Then the error is IpcBenchmarkError::EncodeFailure
    And the inner IpcError is accessible

  Scenario: Decode failure is wrapped as DecodeFailure
    Given corrupted frame bytes
    When IPC decode benchmark executes
    Then the error is IpcBenchmarkError::DecodeFailure
    And the inner IpcError is accessible

Feature: Recovery Benchmark Error Handling

  Scenario: Hydration failure is wrapped as HydrationFailure
    Given invalid recovery event sequence
    When recovery benchmark executes
    Then the error is RecoveryBenchmarkError::HydrationFailure
    And the inner error is accessible

  Scenario: Hydration returns None is wrapped as HydrationFailure
    Given recovery event sequence yielding None
    When recovery benchmark executes
    Then the error is RecoveryBenchmarkError::HydrationFailure
    And message indicates None was returned
```

---

## 4. Proptest Invariants

### 4.1 Evidence Gate Arithmetic Invariants

```rust
// INVARIANT: result_exceeds_threshold is reflexive for no-change
prop_compose! {
    fn arb_duration()(d in 1u64..1_000_000_000) -> Duration {
        Duration::from_micros(d)
    }
}

proptest! {
    #[test]
    fn result_exceeds_threshold_false_when_result_equals_baseline(d in arb_duration()) {
        let threshold_pct = 20u64;
        // result == baseline should never exceed threshold
        assert!(!result_exceeds_threshold(d, d, threshold_pct));
    }

    #[test]
    fn result_exceeds_threshold_true_when_result_significantly_greater(d in arb_duration()) {
        let threshold_pct = 20u64;
        // result > baseline * (100 + threshold_pct) / 100 should exceed threshold
        let baseline = d;
        let result = Duration::from_micros(d.saturating_mul(200)); // 2x baseline
        assert!(result_exceeds_threshold(result, baseline, threshold_pct));
    }

    #[test]
    fn latency_within_budget_false_when_elapsed_greater_than_budget(d in arb_duration()) {
        let budget_us = d / 2;
        let elapsed = Duration::from_micros(d);
        // elapsed > budget should return false
        assert!(!latency_within_budget(elapsed, budget_us));
    }

    #[test]
    fn latency_within_budget_true_when_elapsed_less_than_budget(d in arb_duration()) {
        let elapsed = Duration::from_micros(d / 2);
        let budget_us = d;
        assert!(latency_within_budget(elapsed, budget_us));
    }

    #[test]
    fn budget_utilization_percent_never_exceeds_10000_for_valid_budget(d in arb_duration()) {
        let budget_us = d.max(1); // avoid zero
        let elapsed = Duration::from_micros(d);
        let utilization = budget_utilization_percent(elapsed, budget_us);
        // 100% = 10000 basis points, utilization should be <= 10000 for elapsed <= budget
        if elapsed.as_micros() <= budget_us as u64 {
            assert!(utilization <= 10000);
        }
    }

    #[test]
    fn budget_utilization_percent_returns_max_for_zero_budget(d in arb_duration()) {
        let elapsed = Duration::from_micros(d);
        assert_eq!(budget_utilization_percent(elapsed, 0), u128::MAX);
    }

    #[test]
    fn baseline_within_budget_consistency_with_latency_within_budget(d in arb_duration()) {
        let budget_us = d.max(1);
        let baseline = Duration::from_micros(d / 2);
        // baseline_within_budget(baseline, budget_us) should equal
        // latency_within_budget(baseline, budget_us)
        assert_eq!(
            baseline_within_budget(baseline, budget_us),
            latency_within_budget(baseline, budget_us)
        );
    }
}
```

### 4.2 Metadata Field Invariants

```rust
proptest! {
    #[test]
    fn commit_hash_must_be_nonempty_ascii_hex(commit in "[a-fA-F0-9]{1,40}") {
        let metadata = capture_metadata(
            "test_bench",
            Some(Duration::from_micros(100)),
            Duration::from_micros(110),
            "cargo bench",
            &commit,
            "test-env",
            1000
        );
        assert!(!metadata.commit_hash.is_empty());
        assert!(metadata.commit_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn environment_must_be_nonempty(environment in "[a-zA-Z0-9_-]{1,64}") {
        let metadata = capture_metadata(
            "test_bench",
            Some(Duration::from_micros(100)),
            Duration::from_micros(110),
            "cargo bench",
            "abc123",
            &environment,
            1000
        );
        assert!(!metadata.environment.is_empty());
    }

    #[test]
    fn command_must_be_nonempty(command in "[a-zA-Z0-9_- ]{1,128}") {
        let metadata = capture_metadata(
            "test_bench",
            Some(Duration::from_micros(100)),
            Duration::from_micros(110),
            &command,
            "abc123",
            "test-env",
            1000
        );
        assert!(!metadata.command.is_empty());
    }
}
```

### 4.3 Regression Threshold Invariants

```rust
proptest! {
    #[test]
    fn regression_delta_computed_correctly(baseline_us in 1u64..1_000_000u64, delta_pct in 1u64..50u64) {
        let baseline = Duration::from_micros(baseline_us);
        let threshold_pct = delta_pct;
        let result = Duration::from_micros(baseline_us.saturating_add(baseline_us * threshold_pct / 100));
        
        // result == baseline + threshold should NOT exceed threshold (boundary)
        let does_exceed = result_exceeds_threshold(result, baseline, threshold_pct);
        // At the boundary (100 + threshold_pct)%, result does NOT exceed
        assert!(!does_exceed);
    }

    #[test]
    fn regression_delta_1_percent_over_triggers(d in arb_duration()) {
        let baseline = d;
        let threshold_pct = 20u64;
        // Just over the 20% threshold: baseline * 121 / 100
        let result = Duration::from_micros((baseline.as_micros().saturating_mul(121) / 100).max(baseline.as_micros() + 1));
        
        if result.as_micros() > baseline.as_micros() {
            let delta = result.as_micros().saturating_sub(baseline.as_micros());
            let threshold_delta = baseline.as_micros().saturating_mul(threshold_pct) / 100;
            if delta > threshold_delta {
                assert!(result_exceeds_threshold(result, baseline, threshold_pct));
            }
        }
    }
}
```

---

## 5. Fuzz Targets

### 5.1 YAML Parse Fuzz Targets

| Target | Input Type | Corpus Seeds | Risk Class | Rationale |
|--------|------------|--------------|------------|-----------|
| `parse_yaml_events` | `&str` | Valid YAML workflows from fixtures | **HIGH** | Parse path must not panic on any input |
| `yaml_compile` | `&[u8]` | Valid workflow YAML bytes | **HIGH** | Compile path must not panic on valid input |

### 5.2 Fuzz Target Specifications

```rust
// fuzz/targets/yaml_parse.rs
#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_yaml::parse_yaml_events;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        // Parse must not panic - errors are Ok, panics are bugs
        let _ = parse_yaml_events(text);
    }
});

// fuzz/targets/yaml_compile.rs
#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_compile::compile_workflow;

fuzz_target!(|data: &[u8]| {
    // Compile must not panic - errors are Ok, panics are bugs
    let _ = compile_workflow(data);
});
```

### 5.3 IPC Frame Fuzz Targets

| Target | Input Type | Corpus Seeds | Risk Class |
|--------|------------|--------------|------------|
| `decode_frame` | `&[u8]` | Valid frame bytes | **MEDIUM** | Decode must not panic on malformed bytes |

```rust
// fuzz/targets/ipc_decode.rs
#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_ipc::frame::decode_frame;

fuzz_target!(|data: &[u8]| {
    // Decode must not panic - errors are Ok, panics are bugs
    let _ = decode_frame(data);
});
```

---

## 6. Kani Harnesses

### 6.1 Arithmetic Invariant Proofs

```rust
// kani/proofs/budget_arithmetic.rs

/// Proof: budget_utilization_percent never returns value > 10000 when elapsed <= budget
#[kani::proof]
fn budget_utilization_bounded_by_10000() {
    // Arbitrary but valid inputs
    let elapsed_us: u128 = kani::any();
    let budget_us: u64 = kani::any();
    
    // Assumption: budget is non-zero (zero budget is handled separately)
    kani::assume(budget_us > 0);
    
    // Assumption: elapsed <= budget (the "within budget" case)
    kani::assume(elapsed_us <= u128::from(budget_us));
    
    let elapsed = Duration::from_micros(elapsed_us as u64);
    
    let utilization = budget_utilization_percent(elapsed, budget_us);
    
    // Assert: utilization cannot exceed 10000 (100%) when within budget
    assert!(utilization <= 10000);
}

/// Proof: latency_within_budget returns true iff elapsed <= budget and budget > 0
#[kani::proof]
fn latency_within_budget_correctness() {
    let elapsed_us: u64 = kani::any();
    let budget_us: u64 = kani::any();
    
    let elapsed = Duration::from_micros(elapsed_us);
    
    let result = latency_within_budget(elapsed, budget_us);
    
    // Correctness: latency_within_budget == (budget_us > 0 && elapsed_us <= budget_us)
    let expected = budget_us > 0 && elapsed_us <= budget_us;
    assert_eq!(result, expected);
}

/// Proof: result_exceeds_threshold is false when result <= baseline
#[kani::proof]
fn regression_false_when_result_lte_baseline() {
    let baseline_us: u64 = kani::any();
    let result_us: u64 = kani::any();
    let threshold_pct: u64 = kani::any();
    
    // Assumption: result <= baseline
    kani::assume(result_us <= baseline_us);
    
    let baseline = Duration::from_micros(baseline_us);
    let result = Duration::from_micros(result_us);
    
    let exceeds = result_exceeds_threshold(result, baseline, threshold_pct);
    
    // If result <= baseline, regression cannot be detected
    assert!(!exceeds);
}

/// Proof: result_exceeds_threshold is true when result > baseline + threshold
#[kani::proof]
fn regression_true_when_result_exceeds_threshold() {
    let baseline_us: u64 = kani::any();
    let threshold_pct: u64 = kani::any();
    
    // Assumption: threshold is reasonable (0-100%)
    kani::assume(threshold_pct <= 100);
    
    // result = baseline + baseline * threshold_pct / 100 + 1
    let baseline = Duration::from_micros(baseline_us);
    let threshold_delta = baseline_us.saturating_mul(threshold_pct) / 100;
    let result = Duration::from_micros(baseline_us.saturating_add(threshold_delta).saturating_add(1));
    
    let exceeds = result_exceeds_threshold(result, baseline, threshold_pct);
    
    // Must exceed threshold
    assert!(exceeds);
}
```

### 6.2 Commit Hash Validation Proof

```rust
// kani/proofs/commit_hash_validation.rs

/// Proof: BenchmarkMetadata::commit_hash is always valid (non-empty ASCII hex)
/// when constructed via capture_metadata with valid input
#[kani::proof]
fn capture_metadata_commit_hash_invariant() {
    let name: &str = kani::any();
    let baseline: Option<Duration> = kani::any();
    let result: Duration = kani::any();
    let command: &str = kani::any();
    // commit_hash is valid ASCII hex string
    let commit_hash: &str = kani::any_where(|s| 
        !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
    );
    let environment: &str = kani::any();
    let budget_us: u64 = kani::any();
    
    let metadata = capture_metadata(
        name,
        baseline,
        result,
        command,
        commit_hash,
        environment,
        budget_us,
    );
    
    // Invariant: commit_hash is always non-empty and ASCII hex
    assert!(!metadata.commit_hash.is_empty());
    assert!(metadata.commit_hash.chars().all(|c| c.is_ascii_hexdigit()));
}

/// Proof: Evidence gate returns MissingCommit for empty commit_hash
#[kani::proof]
fn evidence_gate_rejects_empty_commit() {
    let metadata = BenchmarkMetadata {
        name: "test".into(),
        baseline_us: Some(100_000),
        result_us: 110_000,
        command: "cargo bench".into(),
        commit_hash: "".into(), // Empty!
        environment: "test".into(),
        budget_us: 200_000,
    };
    
    let result = check_evidence_gate(&metadata, 20);
    
    match result {
        Err(EvidenceError::MissingCommit) => assert!(true),
        _ => assert!(false, "Expected MissingCommit error"),
    }
}
```

---

## 7. Mutation Testing Checkpoints

### 7.1 Evidence Gate Mutations

| Checkpoint | Mutation | Kill Test | Target |
|------------|----------|-----------|--------|
| EG-M1 | Remove `MissingBaseline` check | `check_evidence_gate_accepts_missing_baseline` | POST-013 |
| EG-M2 | Remove `MissingResult` check | `check_evidence_gate_accepts_missing_result` | POST-013 |
| EG-M3 | Remove `MissingEnvironment` check | `check_evidence_gate_accepts_missing_environment` | POST-013 |
| EG-M4 | Remove `MissingCommand` check | `check_evidence_gate_accepts_missing_command` | POST-013 |
| EG-M5 | Remove `MissingCommit` check | `check_evidence_gate_accepts_missing_commit` | POST-013 |
| EG-M6 | Remove `EmptyBudget` check | `check_evidence_gate_accepts_empty_budget` | POST-014 |
| EG-M7 | Invert regression comparison `>` to `>=` | `check_evidence_gate_accepts_at_threshold` | POST-014 |
| EG-M8 | Change threshold calculation to `/` instead of `* threshold_pct / 100` | `check_evidence_gate_threshold_calculation` | POST-014 |

### 7.2 Metadata Capture Mutations

| Checkpoint | Mutation | Kill Test | Target |
|------------|----------|-----------|--------|
| MC-M1 | Set `commit_hash` to empty string | `capture_metadata_rejects_empty_commit` | INV-005 |
| MC-M2 | Set `environment` to empty string | `capture_metadata_allows_empty_environment` | INV-001 |
| MC-M3 | Set `command` to empty string | `capture_metadata_allows_empty_command` | INV-001 |
| MC-M4 | Set `budget_us` to 0 in output | `capture_metadata_preserves_budget` | INV-004 |
| MC-M5 | Set `baseline_us` incorrectly | `capture_metadata_preserves_baseline` | POST-011 |
| MC-M6 | Set `result_us` incorrectly | `capture_metadata_preserves_result` | POST-011 |

### 7.3 Budget Arithmetic Mutations

| Checkpoint | Mutation | Kill Test | Target |
|------------|----------|-----------|--------|
| BA-M1 | `budget_utilization_percent` returns 0 for zero budget | `budget_utilization_percent_max_for_zero_budget` | INV-004 |
| BA-M2 | `latency_within_budget` returns true for zero budget | `latency_within_budget_false_for_zero_budget` | INV-004 |
| BA-M3 | `result_exceeds_threshold` inverts comparison | `result_exceeds_threshold_boundary` | POST-014 |
| BA-M4 | `baseline_within_budget` uses wrong field | `baseline_within_budget_uses_result_instead` | INV-001 |

### 7.4 Benchmark Group Mutations

| Checkpoint | Mutation | Kill Test | Target |
|------------|----------|-----------|--------|
| BG-M1 | `yaml_parse_benches` returns `Ok(())` without registering | `yaml_parse_group_exists` | POST-001 |
| BG-M2 | `yaml_validate_benches` returns `Ok(())` without registering | `yaml_validate_group_exists` | POST-002 |
| BG-M3 | `yaml_compile_benches` returns `Ok(())` without registering | `yaml_compile_group_exists` | POST-003 |
| BG-M4 | `runtime_step_benches` returns `Ok(())` without registering | `runtime_step_group_exists` | POST-004 |
| BG-M5 | `runtime_primitive_benches` returns `Ok(())` without registering | `runtime_primitive_group_exists` | POST-005 |
| BG-M6 | `ipc_frame_benches` returns `Ok(())` without registering | `ipc_frame_group_exists` | POST-006 |
| BG-M7 | `ipc_backpressure_benches` returns `Ok(())` without registering | `ipc_backpressure_group_exists` | POST-007 |
| BG-M8 | `storage_journal_write_benches` returns `Ok(())` without registering | `storage_journal_write_group_exists` | POST-008 |
| BG-M9 | `storage_journal_replay_benches` returns `Ok(())` without registering | `storage_journal_replay_group_exists` | POST-009 |
| BG-M10 | `recovery_hydration_benches` returns `Ok(())` without registering | `recovery_hydration_group_exists` | POST-010 |

**Mutation Kill Rate Target: ≥90%**

---

## 8. Combinatorial Coverage Matrix

### 8.1 Evidence Gate Input Space

| Scenario | Baseline | Result | Budget | Threshold | Expected Output | Layer |
|----------|----------|--------|--------|-----------|-----------------|-------|
| happy path | 100_000 | 105_000 | 200_000 | 20% | `Ok(())` | unit |
| missing baseline | absent | 105_000 | 200_000 | 20% | `Err(MissingBaseline)` | unit |
| missing result | 100_000 | absent | 200_000 | 20% | `Err(MissingResult)` | unit |
| missing environment | 100_000 | 105_000 | 200_000 | 20% | `Err(MissingEnvironment)` | unit |
| missing command | 100_000 | 105_000 | 200_000 | 20% | `Err(MissingCommand)` | unit |
| missing commit | 100_000 | 105_000 | 200_000 | 20% | `Err(MissingCommit)` | unit |
| regression above | 100_000 | 130_000 | 200_000 | 20% | `Err(RegressionDetected)` | unit |
| regression at boundary | 100_000 | 120_000 | 200_000 | 20% | `Ok(())` | unit |
| zero budget | 100_000 | 105_000 | 0 | 20% | `Err(EmptyBudget)` | unit |
| no baseline (new bench) | None | 105_000 | 200_000 | 20% | `Err(EvidenceError::MissingBaseline)` | unit |

### 8.2 Benchmark Groups Coverage

| Benchmark Group | API Under Test | Fixture(s) | Expected Output | Layer |
|----------------|----------------|------------|-----------------|-------|
| `yaml_parse` | `vb_yaml::parse_yaml_events` | SMALL_WORKFLOW, 1MB | `BenchmarkMetadata` with timing | integration |
| `yaml_validate` | `vb_core::validate_compiled_workflow` | SMALL_WORKFLOW, large | `BenchmarkMetadata` with timing | integration |
| `yaml_compile` | `vb_compile::compile_workflow` | SMALL_WORKFLOW, 1000-step, 1MB | `BenchmarkMetadata` with timing | integration |
| `runtime_step` | `vb_core::run_until_blocked` | save-chain, finish | `BenchmarkMetadata` with timing | integration |
| `runtime_primitive` | scalar expression evaluation | Add, Mul, Compare | `BenchmarkMetadata` with timing | unit |
| `ipc_frame` | `vb_ipc::frame::encode_frame/decode_frame` | 1B, 1KB, 1MB payloads | `BenchmarkMetadata` with timing | integration |
| `ipc_backpressure` | frame submission under load | bounded queue depth | `BenchmarkMetadata` with timing | integration |
| `storage_journal_write` | `FjallJournal::append_journaled` | 100, 1000 events | `BenchmarkMetadata` with timing | integration |
| `storage_journal_replay` | replay of N events | 100, 1000 events | `BenchmarkMetadata` with timing | integration |
| `recovery_hydration` | `recover_run_admission_from_events` | event sequences | `BenchmarkMetadata` with timing | integration |

### 8.3 Error Variant Coverage

| Error Type | Input Class | Expected Behavior | Test Layer |
|------------|-------------|-------------------|------------|
| `YamlBenchmarkError::ParseFailure` | malformed YAML | error wrapped, not panicked | fuzz + unit |
| `YamlBenchmarkError::ValidationFailure` | invalid workflow | error wrapped, not panicked | unit |
| `StorageBenchmarkError::JournalOpenFailure` | invalid path | error wrapped, not panicked | unit |
| `StorageBenchmarkError::AppendFailure` | disk full, corrupted | error wrapped, not panicked | unit |
| `IpcBenchmarkError::EncodeFailure` | invalid payload | error wrapped, not panicked | unit + kani |
| `IpcBenchmarkError::DecodeFailure` | corrupted bytes | error wrapped, not panicked | fuzz + unit |
| `RecoveryBenchmarkError::HydrationFailure` | invalid events, None | error wrapped, not panicked | unit |

---

## 9. Proof Obligations Mapping

All 27 proof obligations from `proof-obligations.jsonl` are addressed:

| Obligation | Layer | Test Strategy |
|------------|-------|---------------|
| PRE-001 (API identification) | manual-qa | Code review checklist in `research_notes.md` |
| PRE-002 (real fixtures) | manual-qa | Code review checklist in `research_notes.md` |
| PRE-003 (metadata schema) | lean | Schema validated via unit tests in `test_metadata_capture.rs` |
| POST-001 to POST-010 (benchmark groups) | gauntlet-fast | `cargo bench` runs and validates group registration |
| POST-011 (metadata emission) | cargo-mutants | Mutation testing on `capture_metadata` |
| POST-012 (no unwrap/expect) | static-scan | `moon run :lint-src` + `cargo-mutants` |
| POST-013 (MissingBaseline) | cargo-mutants | Mutation kills EG-M1 |
| POST-014 (RegressionDetected) | cargo-mutants | Mutation kills EG-M7 |
| INV-001 (evidence required) | gauntlet-fast | Unit tests verify evidence gate rejects missing evidence |
| INV-002 (deterministic) | cargo-mutants | Seeds documented, mutations verify no random inputs |
| INV-003 (no UI dep) | static-scan | `cargo tree -i vb_ui vb_ui_makepad flow-editor-makepad` |
| INV-004 (budget defined) | gauntlet-fast | Unit tests for zero budget handling |
| INV-005 (commit_hash valid) | kani | Proof harness validates non-empty ASCII hex |
| ERR-YAML-001 (parse fuzz) | cargo-fuzz | Fuzz target for `parse_yaml_events` |
| ERR-STORE-001 (journal open) | gauntlet-fast | Unit test with invalid path |
| ERR-IPC-001 (encode) | kani | Proof harness for encode error paths |
| ERR-IPC-002 (decode) | kani | Proof harness + fuzz for decode error paths |
| ERR-RECOV-001 (hydration) | gauntlet-fast | Unit test with invalid events |

---

## 10. Test File Layout

```
vb-fzx7/
├── benches/
│   └── velvet_ballastics.rs          # Main benchmark harness (existing, 93.9K)
├── src/
│   ├── evidence_gate.rs             # Evidence gate implementation (NEW)
│   ├── benchmark_metadata.rs        # Metadata capture implementation (NEW)
│   └── lib.rs                       # Crate root (NEW)
├── tests/
│   ├── test_evidence_gate.rs        # Evidence gate unit tests
│   ├── test_metadata_capture.rs     # Metadata capture unit tests
│   ├── test_budget_arithmetic.rs    # Budget math unit tests
│   ├── test_error_variants.rs       # Error variant tests
│   └── test_invariants.rs          # INV-001 to INV-005 tests
├── fuzz/
│   ├── Cargo.toml
│   └── targets/
│       ├── yaml_parse.rs            # YAML parse fuzz target
│       ├── yaml_compile.rs          # YAML compile fuzz target
│       └── ipc_decode.rs           # IPC decode fuzz target
├── kani/
│   └── proofs/
│       ├── budget_arithmetic.rs      # Budget math Kani proofs
│       └── commit_hash_validation.rs # Commit hash Kani proofs
└── research_notes.md                 # Manual QA verification notes
```

---

## 10.5 Planned Unit Test Count

The 5× density requirement mandates a minimum of `14 public functions × 5 = 70` unit tests.

| Test File | Planned Count |覆盖内容 |
|-----------|------------|---------|
| `test_evidence_gate.rs` | 9 | EG-001–EG-008 (one per EvidenceError variant + happy path) |
| `test_metadata_capture.rs` | 11 | MC-001–MC-010 (10 BDD scenarios + 1 new MC-003 panic test) |
| `test_budget_arithmetic.rs` | 12 | `latency_within_budget`, `budget_utilization_percent`, `result_exceeds_threshold`, `baseline_within_budget` with boundary + proptest cases |
| `test_error_variants.rs` | 8 | BE-001–BE-007 (one per BenchmarkError variant) + EV-001–EV-007 |
| `test_invariants.rs` | 30 | INV-001–INV-005 proptest cases + Kani harness equivalents |
| **Total** | **70** | 5× density (14 functions × 5) ✓ |

Density audit: 70 planned / 14 functions = **5.0×** (meets minimum).

---

## 11. Exit Criteria

- [ ] All 27 proof obligations have assigned test strategies
- [ ] Every `EvidenceError` variant has a BDD scenario and unit test
- [ ] Every `BenchmarkMetadata` field has a proptest invariant
- [ ] Every benchmark group has an integration test confirming registration
- [ ] Fuzz targets exist for `parse_yaml_events` and `decode_frame`
- [ ] Kani harnesses exist for arithmetic invariants and commit_hash validation
- [ ] Mutation testing checkpoints target ≥90% kill rate
- [ ] Planned unit test count ≥ 70 (5× density for 14 public functions) — see Section 10.5
- [ ] No `is_ok()` or `is_err()` assertions in test functions — all assertions are specific value checks
- [ ] Combinatorial coverage matrix shows all significant input combinations
