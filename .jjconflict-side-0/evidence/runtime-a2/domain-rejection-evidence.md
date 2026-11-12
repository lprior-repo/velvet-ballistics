# runtime-a2 recovery closure evidence

Raw command logs are tracked under `evidence/runtime-a2/raw/`.

## Durable recovery decision

- Pending action recovery is resumable from durable Fjall evidence when the
  accepted artifact, workflow digest, admission, action contract ABI digest,
  and `ActionScheduledTicket` all match.
- Open ask recovery is resumable only when durable events include
  `AskScheduledEvent` and the compiled workflow proves the ask has no timeout.
- Wait and timed-ask recovery remain typed `CannotResume` products because the
  current event model has no durable `Instant`/deadline/generation authority to
  reconstruct the timer wheel without fabricating process-local state.
- Wait/timed-ask live resume is therefore explicitly scoped to follow-up
  `vb-ytbn1`: do not synthesize `WaitResolved`/`AskTimedOut` resume state until
  durable timer authority is journaled and replayed from Fjall evidence.
- `vb_storage::recovery::RecoveryHydration::FrameSeed` now carries a typed
  `RecoveryFrameSeedProduct`, not a raw `RecoveryFrameSeed`. The raw replay DTO
  remains available for lower-level recovery, but the public hydration product
  is a `CannotResume`/`Resumable` sum type and records the cannot-resume witness
  at construction. The runtime boundary now pattern-matches that product rather
  than erasing it with `into_seed()`, preserving the storage cannot-resume witness
  even when frame shape validation fails.

## Current evidence summary

- Source-length PASS: `over_limit=0` in `raw/source-length.log`.
- Verus production-binding PASS: `VACUUM=0` in
  `raw/verus-production-binding.log`.
- Verus production-inner drift PASS: `Drift findings: 0` in
  `raw/production-inner-drift.log`.

## PASS evidence

```text
raw/git-root.log                         EXIT_STATUS: 0
raw/jj-root.log                          EXIT_STATUS: 0
raw/roots.log                            EXIT_STATUS: 0
raw/fmt-check.log                        EXIT_STATUS: 0
raw/check.log                            EXIT_STATUS: 0
raw/clippy.log                           EXIT_STATUS: 0
raw/verus-production-binding.log         EXIT_STATUS: 0 (WEAK=72, VACUUM=0)
raw/production-inner-drift.log           EXIT_STATUS: 0 (drift findings=0)
raw/storage-typestate-test.log           EXIT_STATUS: 0 (1 passed)
raw/runtime-boundary-typestate-test.log  EXIT_STATUS: 0 (1 passed)
raw/runtime-fjall-test.log               EXIT_STATUS: 0 (16 passed)
raw/runtime-recovery-suite.log           EXIT_STATUS: 0 (125 passed, 1 ignored)
raw/runtime-recovery-unit.log            EXIT_STATUS: 0 (23 passed)
raw/storage-recovery-tests.log           EXIT_STATUS: 0 (253 passed)
raw/source-length.log                    EXIT_STATUS: 0; over_limit=0 all categories
```

## Known non-slice blocker

```text
raw/clippy-tests-blocker.log             EXIT_STATUS: 101
```

The all-test-target clippy command fails in existing `vb_storage` test lint
policy conflicts (`allow(clippy::panic/unwrap/expect/...)` overruled by
command-line forbid), not in the recovery source slice. Source clippy passed.

## Not claimed here

- No `moon ci` status is claimed here; bead closure, Dolt sync, and Git/JJ push
  were not performed in this scoped continuation.
- Durable wait and timed-ask live resume remain intentionally fail-closed until
  follow-up `vb-ytbn1` journals enough deadline/timer authority to reconstruct
  the timer wheel from Fjall evidence.

The current source-length PASS is in `raw/source-length.log`.
