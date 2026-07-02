# Proof Evidence — vb-rpch verus-flux-rust-r5

## Kani command discovery / planned-command blocker

Command:

```text
cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani --no-unwind
```

Result:

```text
error: unexpected argument '--no-unwind' found
tip: a similar argument exists: '--no-unwinding-checks'
```

Disposition: planned `--no-unwind` flag is invalid for Kani 0.67.0; reruns below intentionally omit it rather than disabling unwinding checks.

## Kani successful harness evidence

Commands run from `/home/lewis/src/vb-jpq7-jj-fix`:

```text
cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani
```

Result excerpt:

```text
VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

```text
cargo kani -p vb_storage --harness recovery_frame_seed_dimensions_kani
```

Result excerpt:

```text
SUMMARY:
 ** 0 of 381 failed (5 unreachable)
VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

```text
cargo kani -p vb_storage --harness action_replay_tracker_monotonic_kani && cargo kani -p vb_storage --harness digest_check_hierarchy_kani
```

Result excerpt:

```text
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
...
VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

## Kani remaining resource blockers

Command attempted:

```text
cargo kani -p vb_storage --harness recovery_frame_seed_dimensions_kani && cargo kani -p vb_storage --harness action_replay_tracker_monotonic_kani && cargo kani -p vb_storage --harness digest_check_hierarchy_kani && cargo kani -p vb_storage --harness hydrate_run_frame_precond_kani && cargo kani -p vb_storage --harness hydrate_run_frame_from_events_precond_kani && cargo kani -p vb_storage --harness replay_events_kani
```

Result: timed out after 240s while processing Kani unwinding / drop paths before reaching hydration/replay harnesses. After r5 repairs, `001..004` were rerun separately and passed; `005..007` remain pending/blocker.

## Proptest evidence

Commands run from `/home/lewis/src/vb-jpq7-jj-fix`:

```text
PROPTEST_CASES=256 rtk cargo test -p vb_storage --test recovery_property_tests -- --nocapture
```

Result:

```text
cargo test: 7 passed (1 suite, 0.01s)
```

Exact planned-name runs at `PROPTEST_CASES=4096`:

```text
PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_unsupported_recovery_state_union --exact --nocapture
PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_seed_dimensions --exact --nocapture
PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_action_replay_tracker_monotonic --exact --nocapture
PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_digest_check_hierarchy --exact --nocapture
PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_hydrate_run_frame_preconditions --exact --nocapture
PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_hydrate_run_frame_from_events_preconditions --exact --nocapture
PROPTEST_CASES=4096 rtk cargo test -p vb_storage --test recovery_property_tests -- proptest_replay_events_attempt_filter --exact --nocapture
```

Observed result for each exact run:

```text
cargo test: 1 passed, 6 filtered out
```

## Fuzz evidence / blocker

Command:

```text
cargo fuzz run vb_rpch_seed_dimensions_fuzz -- -runs=16 -max_len=8
```

Result excerpt:

```text
error: sanitizer is incompatible with statically linked libc, disable it using `-C target-feature=-crt-static`
error[E0463]: can't find crate for `std`
= note: the `x86_64-unknown-linux-musl` target may not be installed
Error: failed to build fuzz script ... --target x86_64-unknown-linux-musl --bin vb_rpch_seed_dimensions_fuzz
```

Disposition: `BLOCKED_TOOLCHAIN_TARGET`; target artifacts are written, execution requires installing/configuring a sanitizer-compatible target/toolchain.

## Formatting

Command:

```text
rtk cargo fmt --check
```

Initial result: formatting diffs in Kani/proptest files.  
Repair command:

```text
rtk cargo fmt
```

Result: no output.
