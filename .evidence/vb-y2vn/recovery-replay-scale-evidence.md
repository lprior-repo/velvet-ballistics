# vb-y2vn RecoveryReplayFull scaled TLC evidence

## Scope

`vb-y2vn` scales the repaired `RecoveryReplayFull` safety model beyond the
`vb-2tpu` representative bound (`RunId={1}`, `StepId={1}`, `ActionId={1}`,
`Attempt={1}`, `MAX_SEQ=2`, `MAX_EVENTS=2`).  The scaled checks keep the same
non-vacuous safety invariants and deadlock checking enabled; they do not use
symmetry, state constraints, `VIEW`, distributed TLC, or simulation.

All commands were run from
`/home/lewis/src/vb-y2vn-recovery-replay-scale-gpt55`.  `TMPDIR` and
`JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=...` pointed inside
`.evidence/vb-y2vn/java-tmp`; no `/tmp` or `/tmp/opencode` path was used.

## Toolchain

- Java: `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java`, OpenJDK
  `26.0.1+8-34`.
- TLC: `TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)`.

Tool-check command:

```bash
command -v java && java --version && if command -v tlc >/dev/null 2>&1; then tlc -version || true; else printf 'tlc absent\n'; fi
```

Exit: `0` for the shell command.  TLC printed its version and rejected the
unsupported `-version` option, so this is tool-availability evidence only, not a
model-check run.

## Model bounds and outcomes

| Config | Bounds changed vs vb-2tpu | TLC outcome | States generated | Distinct states | Queue left | Depth | Raw log |
|---|---:|---|---:|---:|---:|---:|---|
| `RecoveryReplayFull.cfg` | baseline | PASS | 73,058 | 6,147 | 0 | 12 | `.evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-baseline.log` |
| `RecoveryReplayFull.scaled-seq3-events3.cfg` | `MAX_SEQ=3`, `MAX_EVENTS=3` | PASS | 1,719,393 | 120,260 | 0 | 13 | `.evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-scaled-seq3-events3.log` |
| `RecoveryReplayFull.scaled-two-runs-events2.cfg` | `RunId={1,2}` | PASS | 1,104,177 | 76,824 | 0 | 12 | `.evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-scaled-two-runs-events2.log` |
| `RecoveryReplayFull.scaled-two-attempts-events2.cfg` | `Attempt={1,2}` | PASS | 385,300 | 33,985 | 0 | 13 | `.evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-scaled-two-attempts-events2.log` |
| `RecoveryReplayFull.scaled-two-actions-events2.cfg` | `ActionId={1,2}` | PASS | 464,501 | 40,044 | 0 | 14 | `.evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-scaled-two-actions-events2.log` |
| coverage rerun of `scaled-seq3-events3` | `MAX_SEQ=3`, `MAX_EVENTS=3` | PASS with TLC coverage | 1,719,393 | 120,260 | 0 | 13 | `.evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-coverage-scaled-seq3-events3.log` |

All PASS runs ended with `Model checking completed. No error has been found.`
and `0 states left on queue`; deadlock checking was not disabled.

## Exact TLC commands

Baseline:

```bash
TMPDIR="$PWD/.evidence/vb-y2vn/java-tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/.evidence/vb-y2vn/java-tmp" timeout 300 tlc -metadir ".evidence/vb-y2vn/metadir/baseline" -config "specs/tla/RecoveryReplayFull.cfg" "specs/tla/RecoveryReplayFull.tla" > ".evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-baseline.log" 2>&1
```

Exit: `0`.

Scaled sequence/event bound:

```bash
TMPDIR="$PWD/.evidence/vb-y2vn/java-tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/.evidence/vb-y2vn/java-tmp" timeout 300 tlc -metadir ".evidence/vb-y2vn/metadir/scaled-seq3-events3" -config "specs/tla/RecoveryReplayFull.scaled-seq3-events3.cfg" "specs/tla/RecoveryReplayFull.tla" > ".evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-scaled-seq3-events3.log" 2>&1
```

Exit: `0`.

Scaled two-run bound:

```bash
TMPDIR="$PWD/.evidence/vb-y2vn/java-tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/.evidence/vb-y2vn/java-tmp" timeout 300 tlc -metadir ".evidence/vb-y2vn/metadir/scaled-two-runs-events2" -config "specs/tla/RecoveryReplayFull.scaled-two-runs-events2.cfg" "specs/tla/RecoveryReplayFull.tla" > ".evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-scaled-two-runs-events2.log" 2>&1
```

Exit: `0`.

Scaled two-attempt bound:

```bash
TMPDIR="$PWD/.evidence/vb-y2vn/java-tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/.evidence/vb-y2vn/java-tmp" timeout 300 tlc -metadir ".evidence/vb-y2vn/metadir/scaled-two-attempts-events2" -config "specs/tla/RecoveryReplayFull.scaled-two-attempts-events2.cfg" "specs/tla/RecoveryReplayFull.tla" > ".evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-scaled-two-attempts-events2.log" 2>&1
```

Exit: `0`.

Scaled two-action bound:

```bash
TMPDIR="$PWD/.evidence/vb-y2vn/java-tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/.evidence/vb-y2vn/java-tmp" timeout 300 tlc -metadir ".evidence/vb-y2vn/metadir/scaled-two-actions-events2" -config "specs/tla/RecoveryReplayFull.scaled-two-actions-events2.cfg" "specs/tla/RecoveryReplayFull.tla" > ".evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-scaled-two-actions-events2.log" 2>&1
```

Exit: `0`.

Coverage rerun:

```bash
TMPDIR="$PWD/.evidence/vb-y2vn/java-tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/.evidence/vb-y2vn/java-tmp" timeout 300 tlc -coverage 1 -metadir ".evidence/vb-y2vn/metadir/coverage-scaled-seq3-events3" -config "specs/tla/RecoveryReplayFull.scaled-seq3-events3.cfg" "specs/tla/RecoveryReplayFull.tla" > ".evidence/vb-y2vn/logs/tlc-RecoveryReplayFull-coverage-scaled-seq3-events3.log" 2>&1
```

Exit: `0`.

Expected-fail non-vacuity probe:

```bash
TMPDIR="$PWD/.evidence/vb-y2vn/java-tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/.evidence/vb-y2vn/java-tmp" timeout 300 tlc -metadir ".evidence/vb-y2vn/metadir/nonvacuity-badseq" -config ".evidence/vb-y2vn/nonvacuity/RecoveryReplayBadSeq.cfg" ".evidence/vb-y2vn/nonvacuity/RecoveryReplayBadSeq.tla" > ".evidence/vb-y2vn/logs/tlc-nonvacuity-badseq.log" 2>&1; rc=$?; test "$rc" -eq 12
```

Exit: `0` for the wrapper; inner TLC exit was `12` as expected for invariant
violation.

## Invariants checked

Every scaled PASS config checks the same invariants as the representative model:

- `TypeOK`
- `TailCausalAfterSnapshot`
- `ReplaySeqOrder`
- `OnlyIncompleteRuns`
- `NoResolvedReExecution`
- `DigestVerificationOrder`

The safety spec remains `Spec == Init /\ [][Next]_vars`.  No temporal liveness
property is claimed for `RecoveryReplayFull`; the `vb-2tpu` split
`RecoveryReplayErrors` model remains the liveness/error-coverage evidence.

## Non-vacuity and coverage

- The expected-fail probe `.evidence/vb-y2vn/nonvacuity/RecoveryReplayBadSeq.tla`
  violates `ReplaySeqOrder` with `<<[seq |-> 1], [seq |-> 0]>>`; TLC reported
  `Error: Invariant ReplaySeqOrder is violated`, with `2 states generated, 2
  distinct states found`, depth `2`.
- The coverage rerun for `scaled-seq3-events3` shows core recovery actions were
  evaluated and generated real successors, including `AppendEvent` (`42357`),
  `SetSnapshot` (`172`), `DiscoverIncomplete` (`895`), `ReplayEvents`
  (`12115`), `CheckWorkflowDigest` (`38832`), and `CheckIrDigest` (`25888`).
- PASS runs have non-trivial distinct-state counts and complete depths; the
  scaled `MAX_SEQ=3`/`MAX_EVENTS=3` run is about `19.6x` the representative
  distinct-state count (`120,260` vs `6,147`).

## Rust refinement map retained from vb-2tpu

- `journal` and event sequence order refine `FjallJournal::events_for_run`
  consumers in `crates/vb_storage/src/recovery/replay/core.rs` and recovery
  summary/frame hydration in `crates/vb_storage/src/recovery/recover.rs`.
- `snapshot_seq` and `TailCausalAfterSnapshot` refine snapshot-plus-tail checks
  and tail-event validation in `crates/vb_storage/src/recovery/replay/core.rs`
  and `crates/vb_storage/src/recovery/hydrate.rs`.
- `tracker` and `NoResolvedReExecution` refine `ActionReplayTracker` and replay
  code that records completed/failed actions.
- `recovered_runs` and `OnlyIncompleteRuns` refine incomplete-run discovery.
- `digest_level`, `workflow_verified`, and `ir_verified` refine digest-check
  ordering before IR verification.

## Proof-reviewer checklist application

- Type invariant present and checked in every config: yes.
- Domain-specific invariants beyond type shape: yes (`TailCausalAfterSnapshot`,
  `ReplaySeqOrder`, `OnlyIncompleteRuns`, `NoResolvedReExecution`,
  `DigestVerificationOrder`).
- Deadlock stance: deadlock checking enabled; no disabled-deadlock config.
- Finite bounds explicit: yes, each `.cfg` enumerates finite constants.
- Symmetry/state constraints/views: none used.
- Simulation reported as proof: no.
- Non-vacuity: expected-fail invariant probe plus TLC coverage run.
- Raw command evidence: logs and metadirs under `.evidence/vb-y2vn/`.

## Formal-verifier closure summary

The scoped TLA+ safety obligation has raw TLC evidence for each scaled model and
records exact commands, bounds, invariants, state counts, deadlock stance, and
logs.  There are no accepted waivers and no pending TLA+ execution for this
bead.

## Residual scaling limits

This closes the requested scale-up beyond representative bounds, but it is still
finite bounded evidence, not a proof over production-sized `MAX_SEQ=100` /
`MAX_EVENTS=20` or all cross-products of enlarged run/action/attempt domains.
Further scale would require a specific model-reduction follow-up such as a safe
safety-only symmetry argument, an action-specific abstraction, or a refinement
split that avoids combining larger event length with multiple identifier axes in
one TLC state space.
