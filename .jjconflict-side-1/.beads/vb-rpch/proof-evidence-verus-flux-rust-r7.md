# Proof Evidence — vb-rpch verus-flux-rust-r7

## Artifact edited

Artifact: `crates/vb_storage/src/kani_recovery_hydrate.rs`  
Obligations: `VFR-R2-KANI-005`, `VFR-R2-KANI-006`, `VFR-R2-KANI-007`

Change: r7 split the three timeout harnesses away from broad symbolic full hydration/replay and onto finite proof-surface predicates plus one exact early-return/empty-call per harness. No production-reachable file was edited.

No safety, unwinding, overflow, assertion, or memory checks were disabled.

## Non-vacuity / coverage markers in source

- `hydrate_run_frame_precond_kani`:
  - `kani::cover!(tail_run == run_id, "tail run match covered")`
  - `kani::cover!(tail_run != run_id, "tail run mismatch covered")`
  - `kani::cover!(tail_seq > snapshot.seq, "tail seq after snapshot covered")`
  - `kani::cover!(tail_seq <= snapshot.seq, "tail seq not after snapshot covered")`
  - `kani::cover!(preconditions, "snapshot-tail preconditions true covered")`
  - `kani::cover!(!preconditions, "snapshot-tail preconditions false covered")`
  - `kani::cover!(no_data_result.is_err(), "hydrate_run_frame no-data Err covered")`
- `hydrate_run_frame_from_events_precond_kani`:
  - `kani::cover!(events.is_empty(), "empty events covered")`
  - `kani::cover!(!events.is_empty(), "non-empty events covered")`
  - `kani::cover!(preconditions, "events preconditions true covered")`
  - `kani::cover!(!preconditions, "events preconditions false covered")`
  - `kani::cover!(result.is_err(), "hydrate_run_frame_from_events empty Err covered")`
- `replay_events_kani`:
  - `kani::cover!(attempt.is_none(), "absent attempt default covered")`
  - `kani::cover!(attempt.is_some(), "present attempt covered")`
  - `kani::cover!(replay_attempt_is_current(attempt, max_attempt), "current attempt covered")`
  - `kani::cover!(replay_attempt_is_stale(attempt, max_attempt), "stale attempt covered")`
  - `kani::cover!(events.is_empty(), "empty replay covered")`
  - `kani::cover!(result.is_ok(), "replay_events Ok path covered")`

These markers are source-level non-vacuity intent only; because all exact Kani commands timed out, no Kani-generated cover report is claimed.

## Exact Kani harness attempts

Command:

```text
cargo kani -p vb_storage --harness replay_events_kani
```

Result:

```text
Timed out after 180000 ms.
Output continued to show std fmt/string/allocator model expansion despite r7 finite attempts and fixed events, including:
Not unwinding recursion std::fmt::Formatter::<'_>::pad_integral iteration 6
aborting path on assume(false) ... alloc::raw_vec::RawVecInner::grow_amortized
```

Raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e5d03a5420010Rlvrlzq54DEbK`

Command:

```text
cargo kani -p vb_storage --harness hydrate_run_frame_precond_kani
```

Result:

```text
Timed out after 180000 ms.
Output continued to show str/slice-error model expansion despite r7 precondition split, including:
core::str::slice_error_fail_rt
core::str::<impl str>::floor_char_boundary
```

Raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e5d06a552001ir4Rj8tKo50zzO`

Command:

```text
cargo kani -p vb_storage --harness hydrate_run_frame_from_events_precond_kani
```

Result:

```text
Timed out after 180000 ms.
Output continued to show std::io::Error/drop and raw_vec deallocation model expansion, including:
std::ptr::drop_in_place::<std::io::Error>
std::io::error::repr_bitpacked::decode_repr
alloc::raw_vec::RawVecInner::deallocate
```

Raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e5d09b39c001SaFGuSs3mlm1Q9`

Disposition: `VFR-R2-KANI-005..007` remain `BLOCKED_RESOURCE_TIMEOUT_R7`; no Kani PASS claimed.

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
