# Proof Evidence — vb-rpch verus-flux-rust-r6

## Kani r6 model reduction

Artifact: `crates/vb_storage/src/kani_recovery_hydrate.rs`  
Obligations: `VFR-R2-KANI-005`, `VFR-R2-KANI-006`, `VFR-R2-KANI-007`

Change: `bounded_event_vec()` now generates contiguous event vectors of length `0..=2`, with finite event shapes: `RunAccepted`, `StepStarted`, `StepSucceeded`, `ActionScheduled`, `ActionCompletedEvent`, `ActionFailedEvent`, `SlotWrittenEvent`, and `RunFailedEvent`. IDs, attempts, slots, actions, and steps are bounded to small integer domains. Added `kani::cover!` markers for empty/non-empty input and Ok/Err result reachability where applicable.

No Kani safety, unwind, or default checks were disabled. The invalid planned `--no-unwind` flag was not used.

## Kani exact harness attempts

Command:

```text
cargo kani -p vb_storage --harness replay_events_kani
```

Result:

```text
Timed out after 180000 ms.
Output included allocator/formatter unwinding/resource growth, including:
Not unwinding recursion std::fmt::Formatter::<'_>::pad_integral iteration 6
aborting path on assume(false) ... alloc::raw_vec::RawVecInner::grow_amortized
```

Raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e5cf34bb6001xZPzDmYLgnn5i1`

Command:

```text
cargo kani -p vb_storage --harness hydrate_run_frame_precond_kani
```

Result:

```text
Timed out after 180000 ms.
Output included formatter/string/allocator unwinding, including:
aborting path on assume(false) ... core::str::slice_error_fail_rt
aborting path on assume(false) ... alloc::raw_vec::RawVecInner::grow_amortized
```

Raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e5cf775960018HJYHB4W6GJzTG`

Command:

```text
cargo kani -p vb_storage --harness hydrate_run_frame_from_events_precond_kani
```

Result:

```text
Timed out after 180000 ms.
Output included formatter/string unwinding, including:
Unwinding loop core::str::<impl str>::floor_char_boundary ...
aborting path on assume(false) ... core::str::slice_error_fail_rt
```

Raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e5cfa9948001pdgmqEJ55lZNCe`

Disposition: `VFR-R2-KANI-005..007` remain `BLOCKED_RESOURCE_TIMEOUT_R6`; no Kani PASS claimed.

## Fuzz target discovery and musl blocker classification

Command:

```text
cargo fuzz list
```

Result:

```text
vb_rpch_hydrate_events_fuzz
vb_rpch_hydrate_snapshot_tail_fuzz
vb_rpch_replay_events_fuzz
vb_rpch_seed_dimensions_fuzz
```

Command:

```text
cargo fuzz run vb_rpch_hydrate_snapshot_tail_fuzz -- -runs=16 -max_len=8
```

Result:

```text
error: sanitizer is incompatible with statically linked libc, disable it using `-C target-feature=-crt-static`
error[E0463]: can't find crate for `core`
= note: the `x86_64-unknown-linux-musl` target may not be installed
Error: failed to build fuzz script ... --target x86_64-unknown-linux-musl --bin vb_rpch_hydrate_snapshot_tail_fuzz
```

Disposition: default cargo-fuzz musl path remains blocked in this environment. Repo `fuzz/README.md` sanctions `--target x86_64-unknown-linux-gnu` for Linux sanitizer compatibility.

## Fuzz GNU-target smoke evidence

Command:

```text
cargo fuzz run vb_rpch_seed_dimensions_fuzz --target x86_64-unknown-linux-gnu -- -runs=16 -max_len=8
```

Result excerpt:

```text
Running `.../vb_rpch_seed_dimensions_fuzz ... -runs=16 -max_len=8 ...`
#16 DONE cov: 34 ft: 35 corp: 2/4b lim: 4 exec/s: 0 rss: 47Mb
Done 16 runs in 0 second(s)
```

Command:

```text
cargo fuzz run vb_rpch_hydrate_snapshot_tail_fuzz --target x86_64-unknown-linux-gnu -- -runs=16 -max_len=8
cargo fuzz run vb_rpch_hydrate_events_fuzz --target x86_64-unknown-linux-gnu -- -runs=16 -max_len=8
cargo fuzz run vb_rpch_replay_events_fuzz --target x86_64-unknown-linux-gnu -- -runs=16 -max_len=8
```

Result excerpts:

```text
vb_rpch_hydrate_snapshot_tail_fuzz: #16 DONE cov: 14 ft: 15 corp: 1/1b lim: 4 ... Done 16 runs in 0 second(s)
vb_rpch_hydrate_events_fuzz: #16 DONE cov: 25 ft: 26 corp: 1/1b lim: 4 ... Done 16 runs in 0 second(s)
vb_rpch_replay_events_fuzz: #16 DONE cov: 14 ft: 15 corp: 1/1b lim: 4 ... Done 16 runs in 0 second(s)
```

Disposition: `VFR-R2-FUZZ-001..004` have target smoke execution evidence under the repo-sanctioned GNU sanitizer target. This is bounded smoke evidence only (`runs=16`, `max_len=8`), not a deep fuzz campaign.

## Formatting

Command:

```text
rtk cargo fmt --check
```

Initial result: formatting diff in `crates/vb_storage/src/kani_recovery_hydrate.rs`.

Repair command:

```text
rtk cargo fmt
```

Final command:

```text
rtk cargo fmt --check
```

Final result: no output.
