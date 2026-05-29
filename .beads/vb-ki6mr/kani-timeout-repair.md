# vb-ki6mr Kani Timeout Repair Evidence

## Scope

Repaired the remaining `moon ci` blocker after `source-length` went green: `velvet-ballistics:verify-kani-vb-validate` exited 124 from a 5-minute per-harness timeout.

## Failure Evidence

- Latest pre-repair full CI output: `/home/lewis/.local/share/opencode/tool-output/tool_e73b1ee4d001ZACDANgmtGVTth`.
  - Summary: `Tasks: 31 completed (7 cached), 1 failed`.
- Focused failure command:
  - `moon run velvet-ballistics:verify-kani-vb-validate`
  - Result: FAIL, `Process set failed: exit code 124` after about 5 minutes.

## Changes

- `crates/vb_core/src/kani_workflow_arbitrary.rs`
  - Replaced `format!("step_{}", i)` in the Kani-only `WorkflowParts` arbitrary generator with bounded static step names via `kani_step_name`.
  - Purpose: remove expensive symbolic string-formatting paths from CBMC state space.
- `crates/vb_validate/src/kani_gate_08_structural.rs`
  - Changed `kani_gate_08_arbitrary_parts_valid_accessors_pass` to use arbitrary `WorkflowParts`, then install bounded symbolic valid accessor cases.
  - Changed root-OOB and symbol-OOB negative harnesses to install one guaranteed invalid accessor instead of mutating conditionally when arbitrary accessors happen to exist.
  - Kept structural `WorkflowParts` generation via `kani::any()`; no hardcoded full `WorkflowParts` dummy shape was introduced.

## Kani Context

- Kani version: `cargo-kani 0.67.0`.
- Rust toolchain: `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)`, LLVM `22.1.2`.
- Task command path: `.moon/tasks/kani.yml` `verify-kani-vb-validate`.
- Harnesses in task:
  - `kani_gate_08_valid_zero_accessors_pass`
  - `kani_gate_08_arbitrary_parts_valid_accessors_pass`
  - `kani_gate_08_arbitrary_parts_root_oob_rejected`
  - `kani_gate_08_arbitrary_parts_symbol_oob_rejected`
- Bounds are the harness/source bounds, including `#[kani::unwind(5)]`, `slot_count <= 16`, `symbols_count <= 64`, `root < slot_count`, and `index != u32::MAX` where relevant.
- Resource control: repo task uses `timeout 5m` around each harness plus `flock --shared target/moon-locks/source-mutation.lock`; no extra cgroup was added for this focused CI repair.

## Verification

- `moon run velvet-ballistics:fmt`
  - Result: PASS.
- `moon run velvet-ballistics:verify-kani-vb-validate`
  - Result: PASS.
  - Summary: `Tasks: 1 completed`; `Time: 10m 6s 466ms`.
- `bash scripts/kani-list.sh vb_validate`
  - Result: PASS.
  - Output artifact: `.evidence/kani-list/vb_validate.json`.
  - Noted unsupported constructs from Kani inventory generation: `caller_location (1)`, `foreign function (2)`; required task harnesses still passed.
- `moon ci`
  - Result: PASS.
  - Raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e73fbf17d001r51zaLrLkLraiX`.
  - Summary: `Tasks: 32 completed (5 cached)`; `Time: 9m 47s 332ms`.
  - Kani line: `velvet-ballistics:verify-kani-vb-validate (9m 39s 331ms, 16f466ed)`.

## Residual Risk

- Kani evidence is bounded model checking evidence only; it proves the named harnesses under their stated bounds and unwind limits.
- The valid-accessor structural harness remains intentionally broad and takes several minutes, but no longer times out in canonical CI.
- No performance claim was made.
