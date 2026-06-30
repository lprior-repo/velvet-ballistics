# Machine Gate Report - vb-qi37.5.3

STATUS: PASS

## Passing Gates

- `rtk cargo fmt --check`: PASS.
- `rtk cargo test -p vb_runtime admission::tests::admit_artifact_run`: PASS, 7 passed.
- `rtk cargo test -p vb_storage admission::tests::submit_artifact`: PASS, 7 passed.
- `rtk cargo test -p vb_runtime -p vb_storage --lib admission::tests`: PASS, 49 passed.
- `rtk cargo clippy -p vb_runtime -p vb_storage --lib -- -D warnings -D clippy::unwrap_used -D clippy::panic -D clippy::expect_used`: PASS, no issues found.
- `rtk cargo kani -p vb_compile --harness idempotency_gate_parity`: PASS, `VERIFICATION:- SUCCESSFUL`.
- `moon ci`: PASS, 20 tasks completed.

## Raw Output Pointers

- Kani truncated raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e3586c17c001j0YJW5Sgg35ve7`.
- Moon CI truncated raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e3588bc2d00136LBHB3jhmmQt2`.
- All-target clippy full output: `~/.local/share/rtk/tee/1779014397_cargo_clippy.log`.
