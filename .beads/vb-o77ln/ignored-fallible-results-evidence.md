# vb-o77ln ignored-fallible-results evidence

## Scope

Triage the `moon ci` `ignored-fallible-results` failure that reported:

`UnreadableInput: /home/lewis/src/velvet-ballistics/crates/vb_runtime/src/verification/kani/cancel_kill_lattice.rs`

## Findings

- The stale path does not exist in the current worktree.
- The scanner's current filesystem walk does not reproduce the stale unreadable
  input failure.
- Both direct script execution and Moon's task wrapper pass without code changes.

## Command evidence

- `bash scripts/check-ignored-fallible-results.sh`
  - PASS: self-tests pass, production scan reports `NoViolationFound`.
- `moon run velvet-ballistics:ignored-fallible-results`
  - PASS: task completed in 47.660s with `NoViolationFound`.

## Residual risk

The stale unreadable path was not reproducible after the current filesystem walk.
Full `moon ci` still needs a fresh rerun after the remaining formatting/source
length blockers are handled.
