# vb-8lm51 Kani target-dir isolation evidence

## Failure observed

- Command: `moon ci`
- Output: `/home/lewis/.local/share/opencode/tool-output/tool_e74a550aa0015DuGfi0V5rL06e`
- Result: failed; `Tasks: 31 completed (4 cached), 1 failed`
- Failing task: `velvet-ballistics:verify-kani-vb-validate`
- Symptom: task elapsed `10m 11s 504ms` and exceeded the previous `timeout 10m` envelope while running under full-CI Cargo artifact/cache contention.

## Repair

- File: `.moon/tasks/kani.yml`
- `verify-kani` now sets `CARGO_TARGET_DIR=target/kani-vb-core`.
- `verify-kani-vb-validate` now sets `CARGO_TARGET_DIR=target/kani-vb-validate`.
- `verify-kani-vb-validate` per-harness timeout is widened from `10m` to `15m`.

This is a CI resource isolation/envelope repair only. It is not a proof claim and does not weaken any Kani assertion.

## Post-repair evidence

- Command: `moon run velvet-ballistics:verify-kani-vb-validate`
- Result: passed
- Summary: `Tasks: 1 completed`, `Time: 9m 9s 628ms`

## Final CI evidence

- Command: `moon ci`
- Output: `/home/lewis/.local/share/opencode/tool-output/tool_e74e68f1b001LCKcBMlrqNIFXT`
- Result: passed
- Summary: `Tasks: 32 completed (4 cached)`, `Time: 9m 57s 296ms`
