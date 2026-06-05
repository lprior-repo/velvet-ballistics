# ADR 019 (v1): Performance Evidence

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Every speed claim needs measured baseline and result evidence for the current IR-interpreter path unless a future ADR reopens generated execution.

## Invariants

- Benchmark scaffolds that only compile are not performance evidence.
- Before/after claims include baseline command, result command, workload, source revision, host details, and threshold or rationale.
- Performance work must not add forbidden hot-path APIs or unbounded allocation.
- Generated Rust, maxperf, PGO, and native CPU workflows are not current acceptance evidence.

## Master Anchors

- Section 6: Current Performance Rules, IR Interpreter Scope
- Section 39: Mandatory Benchmarks
- Section 41: Removed PGO and Maxperf Build
- Section 71: Competitive Performance Targets
- Section 77.13: Performance Regression Gates
- Section 77.14: Allocation Tracing Gates
