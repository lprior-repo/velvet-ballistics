# Executable Parity Evidence: vb-gvmt

## Commands

```bash
rtk cargo test -p vb_codegen post_011 -- --nocapture
rtk cargo test -p vb_codegen post_ -- --nocapture
rtk cargo test -p vb_codegen --lib
```

## Results

- `post_011`: PASS, `cargo test: 4 passed, 336 filtered out (3 suites, 0.48s)`
- `post_`: PASS, `cargo test: 33 passed, 307 filtered out (3 suites, 0.64s)`
- `--lib`: PASS, `cargo test: 337 passed (1 suite, 5.35s)`

## POST-011 Coverage

- `post_011_generated_finished_value_taint_and_journal_match_ir_for_expression`: compares generated finished value, result taint, and `SlotWritten`/`RunFinished` journal sequence against IR for an expression fixture.
- `post_011_generated_finished_value_taint_and_journal_match_ir_for_constant_expression`: compares generated finished constant value, taint, and journal against IR.
- `post_011_generated_suspension_matches_ir_for_action_boundary`: compares action suspension with IR `AwaitingAction` and exact generated `ActionScheduled` journal fields.
- `post_011_generated_no_contract_do_taint_violation_matches_runtime_error`: compares generated `DriveError::TaintViolation { step: 0 }` against runtime no-contract taint violation behavior.

## Limits

This is executable representative semantic parity, not exhaustive property generation over every supported workflow shape. `compare_generated_to_ir` remains a static source-pattern/count guard; POST-011 tests are the semantic evidence.
