# vb-wg64 TLA+ Applicability Review

## Decision

A TLA+ temporal model is not applicable for this state.

## Reason

The bead repairs deterministic repository CI failures: rustfmt drift, clippy violations, module resolution, and unused test warnings. The required contract has no distributed protocol, scheduler, retry loop, concurrent state machine, lease, queue, or temporal liveness/safety property.

## Safety Properties Covered Without TLA+

- No production behavior change except lint-safe helper/import/module exposure.
- Test-only cleanup preserves assertions and setup effects.
- No broad lint allowlist without local justification.
- Canonical forced CI passes in a clean workspace.

## Replacement Verification

The applicable verification layer is command evidence plus diff review:

- `rtk cargo fmt --all -- --check`
- `rtk cargo clippy -p xtask --all-targets -- -D warnings`
- `rtk cargo clippy -p vb_cli --all-targets -- -D warnings`
- `rtk cargo check -p vb_storage --tests`
- `moon ci --base HEAD --head HEAD --force`

## Reconsideration Trigger

Introduce a TLA+ model only if future repair work changes CI orchestration semantics, task scheduling, retries, cache invalidation, or any concurrent workflow behavior. This State 3 contract forbids those changes.
