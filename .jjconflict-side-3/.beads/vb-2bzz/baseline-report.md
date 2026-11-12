bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 1
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Source / Isolated Workspace

- **source_checkout**: `/home/lewis/src/velvet-ballistics`
- **isolated_workspace**: `/home/lewis/src/go-skill-vb-2bzz`
- **isolation proof**: git worktree at `/home/lewis/src/velvet-ballistics/.git/worktrees/go-skill-vb-2bzz` pointing to `main`
- **path guard**: isolated path is a sibling of source checkout, not nested within it

## Current State

- **Bead status**: claimed (in_progress)
- **Current state**: 1 (Isolation and baseline)
- **Next gate**: State 2 (Explore)

## Baseline Summary

`RecoveryError` already defines `ActionAbiMismatch { action_id }` and `PolicyDigestMismatch { step }` in `crates/vb_storage/src/recovery/types.rs`, but neither variant is ever returned by any public recovery API.

`recover_full_journal` in `crates/vb_storage/src/recovery/replay/core.rs` takes `(journal, run, tracker)` with no ABI or policy digest inputs.

`verify_digests` in `crates/vb_storage/src/recovery/recover.rs` has explicit GAP-3 comment: "Action ABI and policy digest verification is deferred pending lookup function implementation."

Two ignored tests in `crates/vb_storage/tests/recovery_bdd_tests.rs`:
- `action_abi_mismatch_returns_typed_error` (#[ignore = "LETHAL-3"])
- `policy_digest_mismatch_returns_typed_error` (#[ignore = "LETHAL-3"])

Both tests have hollow Ok(_) arms that accept the current broken behavior.

## Baseline Gate Evidence

```bash
cargo test -p vb_storage --test recovery_bdd_tests -- --ignored
```
These ignored tests exist but pass trivially via Ok(_) arms. They are not executable proof.
