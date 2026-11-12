bead_id: vb-8cw4
bead_title: quality: Capture supply public API and perf evidence
phase: 3
updated_at: 2026-05-17T00:00:00Z
attempt: 1-of-7

# Contract Spec

## Requirements

### R1: Supply-Chain Gate Evidence
The system SHALL capture cargo audit, cargo deny, cargo vet, cargo geiger, and cargo machete results as structured evidence. Each tool's output, exit status, and timestamp must be preserved.

### R2: Public API Compatibility Evidence
The system SHALL record the public API surface of each first-party crate. Evidence must include crate name, version, and exported item count.

### R3: Semver Stability Evidence
The system SHALL record the current version of each first-party crate and whether breaking changes are detected relative to the previous version.

### R4: Binary Bloat Analysis
The system SHALL run cargo bloat analysis on the release binary and record the top contributors to binary size.

### R5: Benchmark Evidence
The system SHALL capture benchmark results with required metadata:
- baseline: named baseline identifier (e.g., "vb-current")
- result: measured performance value
- environment: host CPU, OS, toolchain version
- command: exact command used to produce the result

### R6: Performance Evidence for Kernel Paths
The system SHALL verify that all kernel benchmark groups have evidence records. Kernel groups are: yaml_parse, compile_validate, expression, runtime_core, storage_ipc, generated_mode.

## Invariants

### I1: Audit Failure Blocks Final Gate
If any supply-chain audit tool reports a failure (non-zero exit, or known-failure pattern), the final evidence gate MUST NOT approve.

### I2: Missing Benchmark Baseline Blocks Speed Claim
If a benchmark evidence record lacks baseline metadata, no speed claim may be accepted.

### I3: Evidence Completeness
All six evidence categories (R1-R6) must have non-empty evidence records before the gate passes.

## Assumptions

- A1: The moon CI pipeline is the canonical gate; evidence capture supplements but does not replace it.
- A2: Pre-existing supply-chain failures (fxhash, MPL-2.0 crates) are DEFERRED_GLOBAL and do not block this bead.
- A3: Benchmark harnesses already exist and produce Criterion output; this bead captures and validates that output.

## Verification Layers

| Clause | Verifier | Evidence |
|--------|----------|----------|
| R1 | xtask evidence-gate + cargo audit/deny/vet | Structured JSON evidence file |
| R2 | xtask evidence-gate + cargo metadata | API surface record |
| R3 | xtask evidence-gate + version tracking | Semver record |
| R4 | xtask evidence-gate + cargo bloat | Bloat analysis record |
| R5 | xtask evidence-gate + cargo bench | Benchmark evidence with metadata |
| R6 | xtask evidence-gate + benchmark validation | Kernel path coverage record |
| I1 | Evidence gate test | Test assertion |
| I2 | Evidence gate test | Test assertion |
| I3 | Evidence gate test | Test assertion |
