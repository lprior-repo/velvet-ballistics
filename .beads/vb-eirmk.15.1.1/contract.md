# Contract: vb-eirmk.15.1.1 - RunFrame step counter overflow exact boundary test

## Behavior

- RunFrame::increment_executed() returns Ok(()) when executed is below u64::MAX.
- RunFrame::increment_executed() returns Err(CoreError::StepCounterOverflow) when executed would exceed u64::MAX.
- The last successful increment leaves executed at exactly u64::MAX.
- The failing increment that returns StepCounterOverflow does not mutate any frame state (atomicity of failure).

## Test scenario

- Create a frame via RunFrame::new(runid, StepIdx::ZERO, 2, 2).
- Write a non-default slot value I64(7) with Taint::DerivedFromSecret.
- Set frame state to Running via mark_running(StepIdx::ZERO) before counter seeding.
- Seed the executed counter to u64::MAX - 1 via the private executed field (allowed from child module).
- Call increment_executed: expect Ok(()), verify executed == u64::MAX.
- Clone the entire frame to capture the last-success state.
- Call increment_executed again: expect Err(CoreError::StepCounterOverflow).
- Assert the frame after the failing increment is equal to the pre-clone snapshot.

## Constraints

- One new #[test] only: frame::counter_boundary_tests::runframe_executed_counter_exact_boundary.
- Test file: crates/vb_core/src/frame/counter_boundary_tests.rs.
- Module declaration in crates/vb_core/src/frame.rs.
- No new public API, helpers, or dependencies.
- No Kani/formal/full reachability claims.
- Test helpers ≤25 logical lines including signature.
- No panic/unwrap/expect/as/index/unchecked math; return CoreResult from test helpers.
- Existing misleading overflow test in tests_and_verification.rs remains untouched.
