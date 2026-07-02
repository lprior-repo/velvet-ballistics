bead_id: vb-tw3b
phase: 11

# Machine gate report

Initial focused runs using workspace-local `target/` failed due local environment/resource failures:
- concurrent link attempts: `collect2: fatal error: ld terminated with signal 7 [Bus error]`
- subsequent incremental write: `Disk quota exceeded (os error 122)`

Repair delta: `cargo clean -p vb_codegen`, then rerun with isolated cache target and incremental disabled.

Passing active-context commands:

```text
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/home/lewis/.cache/opencode/vb-tw3b-target rtk cargo test -p vb_codegen generated_expression_primitives_match_interpreter_finish -- --nocapture
=> cargo test: 1 passed, 373 filtered out (3 suites, 0.13s)

CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/home/lewis/.cache/opencode/vb-tw3b-target rtk cargo test -p vb_codegen expression_generated_parity -- --nocapture
=> cargo test: 2 passed, 372 filtered out (3 suites, 0.12s)

CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/home/lewis/.cache/opencode/vb-tw3b-target rtk cargo test -p vb_codegen post_011_generated_finished_value_taint_and_journal_match_ir_for_expression -- --nocapture
=> cargo test: 1 passed, 373 filtered out (3 suites, 0.12s)

CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/home/lewis/.cache/opencode/vb-tw3b-target rtk cargo test -p vb_codegen generated_drive_error_covers_all_step_error_paths -- --nocapture
=> cargo test: 1 passed, 373 filtered out (3 suites, 0.00s)
```

STATUS: PASS
