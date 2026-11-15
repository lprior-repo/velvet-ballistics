# Contract: reject replay sequence overflow

Bead: `vb-eirmk.75.8.25.54`

## Invariant

Direct public replay accepts any first sequence value, then requires every event sequence to be the checked successor of its predecessor. An event after `u64::MAX` has no representable successor and must return the existing `RecoveryError::ReplayDivergence` before replay mutates its tracker. Physical storage-key validation remains unchanged; this contract does not assert that durable keys admit `u64::MAX`.

## Compatibility

Keep empty and singleton inputs valid, including a singleton at `u64::MAX`. Keep `u64::MAX - 1` followed by `u64::MAX` valid. Preserve current ordinary-gap `ReplayDivergence` detail text and validation precedence. Reject `MAX,MAX` and `MAX,0` with the same existing error variant and a precise overflow detail. Do not change public APIs, persisted formats, event schemas, dependencies, scheduler or current-attempt policy.

## Shared callers and acceptance

The common validator is reached through `replay_events` and `recover_snapshot_plus_tail`; cover both public paths. Retain ordinary contiguous, gap, reverse, and duplicate controls. Add bounded property coverage using existing Proptest for adjacent sequence pairs near the numeric boundary. Keep the patch local to replay sequence validation and its tests. New/modified production functions remain at most 25 raw lines, at most five parameters, and production files at most 300 lines.
