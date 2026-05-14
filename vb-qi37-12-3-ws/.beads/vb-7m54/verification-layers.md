# Verification Layers for vb-7m54

## Primary Verification Layer: Loom

**Tool**: loom (crablang/loom)
**Command**: `cargo xtask loom --model <name>`
**What it verifies**: Concurrent Rust code for ordering violations, use-after-free, data races, and deadlock

## Secondary Verification Layer: Integration Tests

**Tool**: cargo nextest
**Command**: `cargo nextest run -p vb_runtime`
**What it verifies**: Concurrent runtime behavior in integration scenarios

## Evidence Standards

For each loom model, the following evidence is required:
1. Model source code in `models/loom/`
2. Command output showing model execution to completion
3. No panic/abort/timeout in output
4. All ordering assertions passed

## Layer Completeness

| Obligation | Layer 1 | Layer 2 | Layer 3 |
|------------|---------|---------|---------|
| VB-CONC-001 | loom | integration | - |
| VB-CONC-002 | loom | integration | - |
| VB-CONC-003 | loom | integration | - |
| VB-CONC-004 | loom | integration | - |
| VB-CONC-005 | loom | integration | - |
| VB-CONC-XTASK | implementation | command test | - |
