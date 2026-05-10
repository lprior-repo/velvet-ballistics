# Contract Specification: vb-fzx7

## Context

- **Feature**: Add core orchestrator benchmark suite with budgets and metadata capture for YAML, runtime, IPC, storage, and recovery paths.
- **Domain terms**:
  - `BenchmarkMetadata` — struct capturing baseline, result, command, commit, environment, and threshold/budget for a single benchmark run
  - `EvidenceGate` — acceptance gate that rejects performance claims lacking baseline/result evidence
  - `LatencyBudget` — u64 microsecond threshold for a benchmark group
  - `RegressionDelta` — difference between baseline and result indicating performance regression
  - `FixtureWorkflow` — canonical YAML workflow used as benchmark input
  - `JournalReplay` — storage replay path for recovery benchmarks
  - `FrameBackpressure` — IPC backpressure measurement path
- **Assumptions**:
  - Existing `benches/velvet_ballastics.rs` provides the benchmark harness substrate
  - `vb_yaml::parse_yaml_events`, `vb_compile::compile_workflow`, `vb_core::run_until_blocked` are the primary kernel APIs under test
  - `vb_storage::FjallJournal` provides the storage backend
  - `vb_ipc::frame::encode_frame`/`decode_frame` provide the IPC frame path
  - `vb_runtime::recover_run_admission_from_events` provides the recovery path
- **Open questions**:
  - Where precisely should benchmark evidence JSON files be stored for release gates (`.benchmark-evidence/` directory proposed)?
  - Should the evidence gate be a compile-time assert or a runtime CI check?

## Preconditions

- PRE-001: Benchmark implementation must read and identify all real YAML, validation, runtime, IPC, and storage APIs before adding benchmarks for them.
- PRE-002: Benchmark fixtures must use real workflow YAML or canonical fixture workflows, not fake placeholders.
- PRE-003: The evidence metadata schema must be defined before benchmarks emit metadata.

## Postconditions

- POST-001: A benchmark group exists for `yaml_parse` covering `parse_yaml_events` with small and 1MB fixtures.
- POST-002: A benchmark group exists for `yaml_validate` covering `validate_compiled_workflow` with small and large fixtures.
- POST-003: A benchmark group exists for `yaml_compile` covering `compile_workflow` with small, 1000-step, and 1MB fixtures.
- POST-004: A benchmark group exists for `runtime_step` covering `run_until_blocked` with save-chain and finish workflows.
- POST-005: A benchmark group exists for `runtime_primitive` covering scalar expression evaluation (Add, Mul, Compare).
- POST-006: A benchmark group exists for `ipc_frame` covering `encode_frame` and `decode_frame` with varying payload sizes.
- POST-007: A benchmark group exists for `ipc_backpressure` covering frame submission under bounded queue depth.
- POST-008: A benchmark group exists for `storage_journal_write` covering `FjallJournal::append_journaled` with 100 and 1000 events.
- POST-009: A benchmark group exists for `storage_journal_replay` covering replay of N journal events.
- POST-010: A benchmark group exists for `recovery_hydration` covering `recover_run_admission_from_events`.
- POST-011: Each benchmark group emits `BenchmarkMetadata` containing `baseline_us`, `result_us`, `command`, `commit_hash`, `environment`, and `budget_us` fields.
- POST-012: Every `Result<T, Error>` from fallible benchmark setup must be handled without `unwrap`/`expect`.
- POST-013: The evidence gate returns `Err(EvidenceError::MissingBaseline)` when baseline metadata is absent.
- POST-014: The evidence gate returns `Err(EvidenceError::RegressionDetected { benchmark, delta })` when result exceeds baseline by more than configured threshold.

## Invariants

- INV-001: No performance claim is accepted by the evidence gate without measured `baseline_us` and `result_us` evidence.
- INV-002: Benchmarks must be deterministic enough for regression gating — no non-deterministic inputs (random, time-of-day, thread-scheduling) unless explicitly seeded and documented.
- INV-003: The benchmark suite must not depend on UI code (`vb_ui`, `makepad`, `flow-editor-makepad` crates).
- INV-004: Every benchmark group must define a `budget_us` before the evidence gate can accept the result.
- INV-005: `BenchmarkMetadata` must always contain a valid `commit_hash` (non-empty ASCII hex string) when recorded from a benchmark run.

## Error Taxonomy

- `EvidenceError::MissingBaseline` — raised when `BenchmarkMetadata` lacks `baseline_us` at the evidence gate.
- `EvidenceError::MissingResult` — raised when `BenchmarkMetadata` lacks `result_us` at the evidence gate.
- `EvidenceError::MissingEnvironment` — raised when `BenchmarkMetadata` lacks `environment` at the evidence gate.
- `EvidenceError::MissingCommand` — raised when `BenchmarkMetadata` lacks `command` at the evidence gate.
- `EvidenceError::MissingCommit` — raised when `BenchmarkMetadata` lacks `commit_hash` at the evidence gate.
- `EvidenceError::RegressionDetected` — raised when `result_us > baseline_us + threshold_pct * baseline_us / 100`.
- `EvidenceError::EmptyBudget` — raised when `budget_us` is zero at the evidence gate.
- `YamlBenchmarkError::ParseFailure` — YAML parse returned an error in benchmark context.
- `YamlBenchmarkError::ValidationFailure` — workflow validation failed in benchmark context.
- `StorageBenchmarkError::JournalOpenFailure` — `FjallJournal::open` returned an error.
- `StorageBenchmarkError::AppendFailure` — `append_journaled` returned an error.
- `IpcBenchmarkError::EncodeFailure` — `encode_frame` returned an error.
- `IpcBenchmarkError::DecodeFailure` — `decode_frame` returned an error.
- `RecoveryBenchmarkError::HydrationFailure` — recovery hydration returned `None` or error.

## Contract Signatures

```rust
// Evidence gate API
pub fn check_evidence_gate(metadata: &BenchmarkMetadata, threshold_pct: u64) -> Result<(), EvidenceError>;
pub fn baseline_within_budget(baseline: Duration, budget_us: u64) -> bool;
pub fn result_exceeds_threshold(result: Duration, baseline: Duration, threshold_pct: u64) -> bool;

// Metadata capture API
pub fn capture_metadata(
    name: &str,
    baseline: Option<Duration>,
    result: Duration,
    command: &str,
    commit_hash: &str,
    environment: &str,
    budget_us: u64,
) -> BenchmarkMetadata;

// Benchmark group stubs (all return Result<(), E>)
pub fn yaml_parse_benches(c: &mut Criterion) -> Result<(), YamlBenchmarkError>;
pub fn yaml_validate_benches(c: &mut Criterion) -> Result<(), YamlBenchmarkError>;
pub fn yaml_compile_benches(c: &mut Criterion) -> Result<(), YamlBenchmarkError>;
pub fn runtime_step_benches(c: &mut Criterion) -> Result<(), RuntimeBenchmarkError>;
pub fn runtime_primitive_benches(c: &mut Criterion) -> Result<(), RuntimeBenchmarkError>;
pub fn ipc_frame_benches(c: &mut Criterion) -> Result<(), IpcBenchmarkError>;
pub fn ipc_backpressure_benches(c: &mut Criterion) -> Result<(), IpcBenchmarkError>;
pub fn storage_journal_write_benches(c: &mut Criterion) -> Result<(), StorageBenchmarkError>;
pub fn storage_journal_replay_benches(c: &mut Criterion) -> Result<(), StorageBenchmarkError>;
pub fn recovery_hydration_benches(c: &mut Criterion) -> Result<(), RecoveryBenchmarkError>;
```

## Non-goals

- Benchmarking UI rendering paths.
- Proving asymptotic complexity theorems for IPC/storage internals.
- Formal proof of the entire criterion measurement pipeline.
- Proving benchmarks are statistically significant (criterion handles this).
