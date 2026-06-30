# Proof Evidence — TLC Fix Round 3 for `vb-rpch`

Status: raw command evidence index only. This is not proof approval.

## Tool discovery

Command run:

```sh
command -v tlc && tlc -version || true
```

Observed:

- `tlc` path: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`
- `tlc -version` reported `TLC2 Version 2.19 of 08 August 2024` and then an unrecognized option usage message.

## TLC commands run

Stale-claim cleanup (`TLC-R2-004`, `TLC-R2-009`, `TLC-R2-017`):

- Removed obsolete `UnmodeledRecoveryErrors` / `RecoveryErrorExhaustiveScopedPartial` operators from `specs/tla/RecoveryReplayFull.tla`.
- Added `ModeledRecoveryErrors == ErrorDomain \ {NoneError}` and `RecoveryErrorWitnessCoverageClaim` enumerating all nine modeled non-`None` error variants.
- Updated `AllNonVacuityWitnessesReached` to require `ReachResolvedActionGuardAntecedent` and removed the unused weak resolved-action predicate.

Primary bounded safety (`TLC-R2-001`, `TLC-R2-004`):

```sh
tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull.tlc.log 2>&1
```

Evidence: `evidence/specs/RecoveryReplayFull.tlc.log`

- Result in log: `Model checking completed. No error has been found.`
- Final stats: `31707925 states generated`, `3448392 distinct states found`, `0 states left on queue`, depth `12`.

Combined non-vacuity smoke witness after guard strengthening (`TLC-R2-009`, `TLC-R2-017`):

```sh
tlc -config specs/tla/RecoveryReplayFull-nonvacuity.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity.tlc.log 2>&1
```

Evidence: `evidence/specs/RecoveryReplayFull-nonvacuity.tlc.log`

- Expected witness violation: `NotAllNonVacuityWitnessesReached`.
- Trace includes `ActionCompleted`, then `MarkReplayCandidate` with same action/step/attempt, satisfying the stronger `ReachResolvedActionGuardAntecedent`.
- Final log stats before witness stop: `3364285 states generated`, `596841 distinct states found`, depth `7`.

No-reexecution non-vacuity witness (`TLC-R2-009`):

```sh
tlc -config specs/tla/RecoveryReplayFull-nonvacuity-no-reexecution.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log 2>&1
```

Evidence: `evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log`

- Expected witness violation: `NotReachResolvedActionGuardAntecedent`.
- Trace includes `ActionCompleted`, then `MarkReplayCandidate` with same action/step/attempt.

Corrupt snapshot witness (`TLC-R2-014`):

```sh
tlc -config specs/tla/RecoveryReplayFull-error-corrupt-snapshot.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-corrupt-snapshot.tlc.log 2>&1
```

Evidence: `evidence/specs/RecoveryReplayFull-error-corrupt-snapshot.tlc.log`

- Expected witness violation: `NotReachErrorCorruptSnapshot`.
- Trace shows `snapshot_inputs = {"Corrupt"}`, `SetSnapshot` to `snapshot_seq = 0`, then `LoadCorruptSnapshot`.

Action ABI mismatch witness (`TLC-R2-015`):

```sh
tlc -config specs/tla/RecoveryReplayFull-error-action-abi-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-action-abi-mismatch.tlc.log 2>&1
```

Evidence: `evidence/specs/RecoveryReplayFull-error-action-abi-mismatch.tlc.log`

- Expected witness violation: `NotReachErrorActionAbiMismatch`.
- Trace shows `abi_expected = 1`, `abi_found = 2`, then `CheckActionAbiDigest` records `ActionAbiMismatch`.

Policy digest mismatch witness (`TLC-R2-016`):

```sh
tlc -config specs/tla/RecoveryReplayFull-error-policy-digest-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-policy-digest-mismatch.tlc.log 2>&1
```

Evidence: `evidence/specs/RecoveryReplayFull-error-policy-digest-mismatch.tlc.log`

- Expected witness violation: `NotReachErrorPolicyDigestMismatch`.
- Trace shows `policy_expected = 1`, `policy_found = 2`, then `CheckPolicyDigest` records `PolicyDigestMismatch`.

Non-idempotent action blocked witness (`TLC-R2-017`):

```sh
tlc -config specs/tla/RecoveryReplayFull-error-non-idempotent-action-blocked.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-non-idempotent-action-blocked.tlc.log 2>&1
```

Evidence: `evidence/specs/RecoveryReplayFull-error-non-idempotent-action-blocked.tlc.log`

- Expected witness violation: `NotReachErrorNonIdempotentActionBlocked`.
- Trace includes `ActionCompleted`, then `MarkReplayCandidate`, then `DetectNonIdempotentResolved` recording `NonIdempotentActionBlocked`.

## Source/evidence sync (`TLC-R2-020`)

Command run:

```sh
cp specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla; cp specs/tla/RecoveryReplayFull*.cfg evidence/specs/; for src in specs/tla/RecoveryReplayFull.tla specs/tla/RecoveryReplayFull*.cfg; do dst="evidence/specs/$(basename "$src")"; cmp -s "$src" "$dst" || { printf 'MISMATCH %s %s\n' "$src" "$dst"; exit 1; }; done > evidence/specs/RecoveryReplayFull-sync.cmp.log 2>&1
```

Evidence: `evidence/specs/RecoveryReplayFull-sync.cmp.log`

- Result: command completed with no mismatch output.

## Bounds and assumptions recorded

- Primary CFG: singleton run/step/action/attempt, all event types, `MAX_SEQ = 3`, `MAX_EVENTS = 3`, equal ABI/policy inputs, no corrupt snapshot input.
- Witness CFGs: tiny singleton domains; expected invariant violations are reachability witnesses, not safety passes.
- ABI/policy mismatch and corrupt snapshot are finite abstract input markers. Runtime lookup and byte-decoding remain out of this TLA proof scope.
- `RecoveryErrorWitnessCoverageClaim` documents finite TLA error-domain coverage only; individual per-error witness logs are the causal reachability evidence.
