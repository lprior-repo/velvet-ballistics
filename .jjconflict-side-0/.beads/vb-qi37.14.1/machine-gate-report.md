# Machine Gate Report - vb-qi37.14.1

## Bead
- **Bead ID**: vb-qi37.14.1
- **Title**: cli: Add single-step run command
- **Date**: 2026-05-18

## Machine Gates Executed

### cargo check --workspace
```
cargo check --workspace
Finished `dev` profile [unoptimized + debuginfo] target(s)
```

### cargo clippy --workspace
```
cargo clippy --workspace
No issues found
```

### cargo test --workspace
```
cargo test --workspace
10962 passed, 44 ignored (160 suites, 12.29s)
```

### cargo test --package vb_cli --test vb_qi37_14_1_run_step
```
cargo test --package vb_cli --test vb_qi37_14_1_run_step
25 passed (1 suite)
```

## Formal Verification Status

### Kani (BLOCKED_TOOLING)
- 6 Kani harnesses compile but timeout due to SlotValue symbolic complexity
- Compensating evidence: 4 Verus proofs PASS for same invariants

### Verus
- 55 lemmas verified across 3 Verus files
- No errors

## Regression Analysis
No regressions detected. Full workspace tests pass.

## Classification
- **BLOCK_LOCAL**: None
- **BLOCK_REGRESSION**: None
- **STATUS**: PASS
