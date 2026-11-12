# Test Plan: vb-qi37.2 State 7

STATUS: APPROVED

## Required Checks

- Kani aggregate admission parity: exact `PO-010` and `PO-011` harnesses.
- Kani ValueStore cap diagnostic parity: exact `PO-012` harness.
- Miri ValueStore scoped UB check: `MIRIFLAGS=-Zmiri-disable-isolation cargo +nightly-2025-11-21 miri test -p vb_core value_store -- --nocapture`.
- Focused budget regression tests: `rtk cargo test -p vb_core budget -- --nocapture`.
- ResourceContract parity tests: `rtk cargo test -p vb_core resource_contract -- --nocapture` and `rtk cargo test -p velvet-ballistics-workspace resource_contract -- --nocapture`.
- Fuzz and `moon ci` remain State 11 blockers until environment/tooling is repaired.
