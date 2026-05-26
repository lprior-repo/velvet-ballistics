# Proof Writer Report — vb-rpch verus-flux-rust-r7

bead: `vb-rpch`  
state: `5 Kani model reduction repair`  
date: 2026-05-24

## Scope executed

Worked only the approved r7 Kani repair sublane for `VFR-R2-KANI-005`, `VFR-R2-KANI-006`, and `VFR-R2-KANI-007`. No production runtime behavior files were edited. The only source edit is the `#[cfg(kani)]` harness module `crates/vb_storage/src/kani_recovery_hydrate.rs`.

## Harness repair summary

- `hydrate_run_frame_precond_kani` (`VFR-R2-KANI-005` / `PO-VB-014`): reduced from symbolic full hydration over bounded vectors to a split proof-surface harness:
  - symbolic domains: `RunId` from `u8`, `EventSeq` in `0..=1`, singleton `RunAccepted` tail, empty exact early-return call;
  - coverage: run match/mismatch, seq after/not-after snapshot, precondition true/false, exact no-data `Err` path.
- `hydrate_run_frame_from_events_precond_kani` (`VFR-R2-KANI-006` / `PO-VB-015`): reduced to events-only precondition surface plus exact empty early-return call:
  - symbolic domains: `bool` emptiness selector, singleton `RunAccepted`, dimension surface checks for `(1,1)`, `(0,1)`, `(1,0)`;
  - coverage: empty/non-empty events, precondition true/false, exact empty `Err` path.
- `replay_events_kani` (`VFR-R2-KANI-007` / `PO-VB-016`): reduced to allocation-light replay proof surfaces plus exact empty replay call:
  - symbolic domains: `attempt = None | Some(1..=2)`, `max_attempt = 1..=2`, fixed `ActionScheduled` and `RunAccepted` events;
  - coverage: absent/present attempt, stale/current attempt, state-effect/inert event predicates, decreasing/nondecreasing step order, exact empty replay `Ok` path.

No Kani safety, overflow, unwind, assertion, or memory checks were disabled. The invalid planned `--no-unwind` flag remains unused.

## Command results

All three exact harness commands were rerun with the required 180s cap. All still timed out. The r7 reductions substantially narrow harness-side symbolic input, but Kani still spends the local budget in dependency/std formatter, string, allocator, `str`, and `std::io::Error` drop/model paths before completion.

See `.beads/vb-rpch/proof-evidence-verus-flux-rust-r7.md` for raw command paths and excerpts.

## Closed blockers

None. No Kani PASS is claimed for `VFR-R2-KANI-005..007`.

## Remaining blockers

- `VFR-R2-KANI-005`: `BLOCKED_RESOURCE_TIMEOUT_R7`. Exact command `cargo kani -p vb_storage --harness hydrate_run_frame_precond_kani` timed out after 180s despite r7 proof-surface split. Non-vacuity markers are present in the harness source for both true/false precondition cases and exact early-return `Err`.
- `VFR-R2-KANI-006`: `BLOCKED_RESOURCE_TIMEOUT_R7`. Exact command `cargo kani -p vb_storage --harness hydrate_run_frame_from_events_precond_kani` timed out after 180s despite r7 proof-surface split. Non-vacuity markers are present for empty/non-empty evidence and exact empty `Err`.
- `VFR-R2-KANI-007`: `BLOCKED_RESOURCE_TIMEOUT_R7`. Exact command `cargo kani -p vb_storage --harness replay_events_kani` timed out after 180s despite finite attempts and fixed event predicates. Non-vacuity markers are present for absent/present and stale/current attempts plus exact empty replay `Ok`.

## Resource recommendation

Do not broaden the harnesses further under the local 180s cap. Next viable options:

1. add an approved Kani-only proof crate or feature-sliced harness target that excludes unrelated crate/dependency initialization and std I/O/drop surfaces; or
2. approve a larger CI Kani budget for these exact harnesses after proof-review accepts the r7 model split; or
3. route to implementation/proof planning for smaller production proof-surface functions that avoid `String`/`Vec` return allocation in the exact harness path.
