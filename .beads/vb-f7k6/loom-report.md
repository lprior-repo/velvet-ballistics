# Loom Report — vb-f7k6 State 11 Retry

STATUS: PASS

- command: `cargo xtask loom --model timer_fired_cancel`
- exit: 0
- result: PASS
- xtask delegated command: `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime timer_fired_cancel`

## Model Results

- `timer_fired_cancel_ordering`: ok
- `timer_fired_replace_ordering`: ok
- `timer_fired_terminal_ordering`: ok
- summary: `3 passed; 0 failed`
- xtask final line: `PASS: Loom model 'timer_fired_cancel' completed successfully`

Warnings from unrelated loom model modules were compiler unused/dead-code warnings only; they did not fail this required model lane.
