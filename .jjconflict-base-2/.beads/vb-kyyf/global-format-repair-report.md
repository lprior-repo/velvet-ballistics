STATUS: REJECTED

# Global Format Repair Report

Bead: vb-kyyf
Sublane: serial global format repair
Attempt: 1 of 7
Workspace: `/home/lewis/src/bd-vb-kyyf-bdd`

## Scope

Formatting-only repair for `crates/vb_storage/src/kani_recovery_hydrate.rs`.
No behavior change was made.

## Change Made

Rustfmt reordered one import:

```diff
+use crate::RecoveryError;
 use crate::recovery::recover::{
     check_action_abi_digests, check_compiled_ir_digest, check_policy_digests,
     recover_runtime_summary,
 };
-use crate::RecoveryError;
 use vb_core::{ActionId, StepIdx, WorkflowDigest};
```

## Commands And Evidence

```text
$ pwd -P
/home/lewis/src/bd-vb-kyyf-bdd
```

Formatting command run before check:

```text
$ mkdir -p .tmp && TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --all
exit: 0
output: none
```

Required format check:

```text
$ TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check
exit: 0
output: none
```

Required canonical gate after format passed:

```text
$ TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci
exit: 1
classification: BLOCK_REGRESSION / environment-local required-gate failure, not a formatting failure
```

Relevant `moon ci` evidence:

```text
velvet-ballistics:fmt (1s 424ms, c55f72d9)
velvet-ballistics:test | Summary [  90.246s] 11106 tests run: 11106 passed (1 slow), 0 skipped
velvet-ballistics:mutants-smoke | Error: Failed to copy .../.tmp/cargo-mutants-bd-vb-kyyf-bdd-ZN4ptw.tmp/.../target_nosccache/debug/incremental/.../0221tx9d3q4uaorqs6ljdairt.o
velvet-ballistics:mutants-smoke | Caused by:
velvet-ballistics:mutants-smoke |     File name too long (os error 36)
Tasks: 20 completed (3 cached), 1 failed
```

## Classification

- Format repair: PASS.
- `velvet-ballistics:fmt`: PASS inside `moon ci`.
- `moon ci`: FAIL because `velvet-ballistics:mutants-smoke` recursively copied the workspace-local `.tmp/cargo-mutants...` directory until path length exceeded OS limits.
- No Rust behavior changed.
- No forbidden Rust constructs were added.

## Landing Rerun Decision

vb-kyyf landing can rerun only after the controller accepts the formatting repair and either reruns `moon ci` with a non-recursive temp directory strategy or repairs the `mutants-smoke` TMPDIR recursion issue. The original rustfmt import-order blocker is cleared.
