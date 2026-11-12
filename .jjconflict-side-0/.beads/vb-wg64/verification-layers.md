# vb-wg64 Verification Layers

## Layer 0: Scope Guard

- Confirm work occurs only in `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64`.
- Confirm State 3 modifies only `.beads/vb-wg64/*` artifacts and `STATE.md` state records.
- Confirm production/test code is untouched in State 3.

## Layer 1: Diff Classification

- Classify every future code diff as formatting-only, import cleanup, unused test cleanup, lint-safe helper restructuring, or module exposure for existing tests.
- Reject unrelated runtime behavior changes.
- Reject broad lint allowlists without documented local justification.

## Layer 2: Targeted Preflight Gates

Run targeted gates after implementation states:

```bash
rtk cargo fmt --all -- --check
rtk cargo clippy -p xtask --all-targets -- -D warnings
rtk cargo clippy -p vb_cli --all-targets -- -D warnings
rtk cargo check -p vb_storage --tests
```

Expected result: all commands exit 0, or any residual failure is proven unrelated and tracked before final CI.

## Layer 3: Assertion Preservation Review

- Review `crates/vb_storage/tests/recovery_bdd_tests.rs` cleanup.
- Confirm no meaningful assertion is removed.
- Confirm setup expressions with side effects are retained, even if bound to `_name`.
- Confirm BDD scenario names and behavioral intent remain intact.

## Layer 4: Output Behavior Review

- Review `commands_ai_context::json_out` repair.
- Confirm JSON, JSONL, and text output behavior remains equivalent.
- Confirm error paths still report failed writes as before.

## Layer 5: Canonical Forced CI

Run the required acceptance gate:

```bash
moon ci --base HEAD --head HEAD --force
```

Expected result: exit 0 in the isolated clean workspace.

## Layer 6: Bead Closure Evidence

- Close `vb-wg64` only after canonical forced CI passes.
- Run `bd close vb-wg64 --force`.
- Run `bd dolt push`.
- Record final command evidence in later state artifacts.
