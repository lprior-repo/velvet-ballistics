# Trusted Base Plan — TLC Fix Round 2 for `vb-rpch`

Status: planning only. No verifier result is claimed here.

## Trusted execution base

- TLC CLI command shape: `tlc -config <cfg> specs/tla/RecoveryReplayFull.tla` from `/home/lewis/src/vb-jpq7-jj-fix`.
- Java/TLC toolchain availability must be rechecked by formal-verifier before execution; planner does not claim tool availability.
- Raw TLC logs under `evidence/specs/*.tlc.log` are the only acceptable execution evidence.
- `cmp -s` byte identity checks are required between `specs/tla` and `evidence/specs` copies after proof-writer edits.

## Trusted modeling reductions introduced by this plan

### Feasible primary cfg reduction

Round 2 intentionally downgrades the primary cfg from the previous nonterminating large search to a finite proof bound expected to close:

- `RunId = {1}`
- `StepId = {1}`
- `ActionId = {1}`
- `Attempt = {1}`
- all event types enabled
- `MAX_SEQ = 3`
- `MAX_EVENTS = 3`

This is a bounded abstraction of the recovery contract. It is trusted as a minimal non-empty domain check, not a universal proof over arbitrary domain cardinality or journal length.

### Large cfg handling

The old large cfg must be renamed or copied to `specs/tla/RecoveryReplayFull-large-stress.cfg`. It is trusted only as a stress/bug-finding target unless it finishes with queue drained. Previous partial BFS output must not be used as approval evidence.

### Digest abstraction

The repaired model should trust `digest_stage` as an abstraction of successful workflow/IR verification state. It does not model cryptographic digest computation; it models ordering of checks and mismatch branching.

Required invariant bridge:

- `IrChecked => WorkflowChecked` maps to `verify_digests` requiring workflow verification before compiled IR verification.

### Error-domain abstraction

Round 2 should add causal TLA transitions for every current TLA `ErrorDomain` non-`None` variant. Abstract transitions for `ActionAbiMismatch` and `PolicyDigestMismatch` are trusted only as typed error-domain witnesses because contract.md marks runtime lookup as GAP-3 deferred.

If the proof-writer does not add `Journal` or `TerminalStateMismatch` to TLA `ErrorDomain`, reports must state that those Rust taxonomy variants are outside this TLA model domain. If the proof-writer adds them, causal transitions and witness cfgs are mandatory.

### Snapshot abstraction

`CorruptSnapshot` may be represented by an abstract corrupt marker or mismatch input. This is not a proof of byte-level snapshot decoding.

### Non-vacuity witnesses

Independent witness cfgs may narrow event types and domains. Those reductions are acceptable only because each witness proves one specific antecedent/error reachability, not full safety. Each narrowed witness must record exact constants and event set in raw evidence/reporting.

## Required residual limitations to report honestly

- Bounded primary proof does not cover journals beyond three events.
- Bounded primary proof does not cover multiple runs, steps, actions, or attempts.
- Stress cfg is not required to finish and is not proof unless it finishes.
- No liveness/fairness claim is planned.
- No cryptographic digest computation is modeled.
- No payload-level snapshot parsing/decoding is modeled.
- GAP-3 ABI/policy runtime lookup remains an implementation/bridge caveat even if TLA witnesses show typed variants are causally reachable in the abstract model.

## Waiver candidates

No behavior-affecting waiver is planned. The only planned downgrade is scope disclosure: old large cfg becomes optional stress evidence, while the primary proof claim is limited to the new explicit small bounds.
