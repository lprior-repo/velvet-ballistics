# Proof Evidence: vb-core-atomic-admission

State 5 attempt: 3-of-7
Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission`

## Path Guard

Command: `pwd -P`
Exit: 0

```text
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission
```

## TLA+ Evidence For TLA-ATOM-001

Command: `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`
Exit: nonzero before syntax repair

```text
***Parse Error***
Precedence conflict between ops <> in block line 123, col 67 to line 123, col 68 of module AtomicAcceptedRunAdmission and =.
Error: Parsing or semantic analysis failed.
```

Repair: parenthesized `<>(readback_decision = "accepted")`.

Command: `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`
Exit: 0

```text
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Computed 2 initial states...
Finished computing initial states: 4 distinct states generated at 2026-05-15 16:14:45.
Progress(12) at 2026-05-15 16:14:45: 6,828 states generated, 1,080 distinct states found, 0 states left on queue.
Checking 2 branches of temporal properties for the complete state space with 2160 total distinct states at (2026-05-15 16:14:45)
Finished checking temporal properties in 00s at 2026-05-15 16:14:45
Model checking completed. No error has been found.
6828 states generated, 1080 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 12.
Finished in 00s at (2026-05-15 16:14:45)
```

Checked invariants/properties from `AtomicAcceptedRunAdmission.cfg`:

- `AllRecordsOrNoAcceptedRun`
- `NoPartialAfterFailure`
- `IndexesOnlyCommitted`
- `ReadbackOnlyCommitted`
- `NoAckBeforeCommit`
- `NoRuntimeAllocationBeforeCommit`
- `EventuallyAckOrFail`
- `EventuallyReadableAfterCommit`

Deadlock evidence: `AtomicAcceptedRunAdmission.cfg` no longer contains `CHECK_DEADLOCK FALSE`, so TLC default deadlock checking is enabled.

Repair marker command: `/usr/bin/rg -n "EventuallyReadableAfterCommit|CHECK_DEADLOCK|WF_vars\\(Readback|PROPERTY" verification/tla/AtomicAcceptedRunAdmission.*`
Exit: 0

```text
verification/tla/AtomicAcceptedRunAdmission.cfg:11:PROPERTY EventuallyAckOrFail
verification/tla/AtomicAcceptedRunAdmission.cfg:12:PROPERTY EventuallyReadableAfterCommit
verification/tla/AtomicAcceptedRunAdmission.tla:100:  /\ WF_vars(ReadbackAccepted)
verification/tla/AtomicAcceptedRunAdmission.tla:123:EventuallyReadableAfterCommit == [](commit_state = "committed" => <>(readback_decision = "accepted"))
```

## Verus Evidence For VERUS-PRE-001 Through VERUS-ERR-006

Command: `verus verification/verus/accepted_run_atomic_admission.rs`
Exit: 0

```text
verification results:: 6 verified, 0 errors
```

Verified proof functions:

- `proof_valid_input_has_required_families`
- `proof_coherent_input_refs`
- `proof_sequence_binding_preserves_truth`
- `proof_raw_workflow_parts_rejected`
- `proof_index_precondition_decomposition`
- `proof_error_taxonomy_exhaustive`

Important non-claims:

- `VERUS-IDX-005` proves pure index precondition decomposition only; no runtime key derivation pass is claimed.
- `VERUS-ERR-006` proves modeled failure causes classify to an `Err` outcome only inside the pure model; no production `Result` propagation pass is claimed.

## Tool Discovery Evidence

```text
which java
exit=0
/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java

which tlc
exit=0
/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc

which verus
exit=0
/home/lewis/.local/bin/verus

cargo kani --version
exit=0
cargo-kani 0.67.0

cargo fuzz --version
exit=0
cargo-fuzz 0.13.1

cargo flux --version
exit=101
error: no such command: `flux`

cargo +nightly miri --version
exit=0
miri 0.1.0 (e0e95a7187 2026-04-04)
```

## Waived, Deferred, And Not-Run Obligations

- `KANI-PROP-007`: WAIVED_NOT_RUN per repaired plan; exact harness missing and owner is State 8 before State 12. Kani is installed, but no pass is claimed.
- `FUZZ-ART-008`: WAIVED_NOT_RUN per repaired plan; exact target missing and owner is State 8 before State 12. cargo-fuzz is installed, but no pass is claimed.
- `MIRI-CODEC-009`: NOT_RUN until implementation touches strict codec/readback paths or targeted tests exist. Miri is installed, but no pass is claimed.
- `MUT-ERR-010`, `STATIC-SCAN-011`, `INTEG-FAIL-012`, `API-COMPAT-013`, and `ERR-INVALID-015` through `ERR-INDEX-022`: NOT_RUN because they are owner State 12 machine/test/formal-verifier obligations after implementation/test artifacts exist.
- `PERF-NONGOAL-014`: NOT_APPLICABLE while no performance claim exists.

## Trusted Boundaries

- Fjall batch commit is modeled as an atomic durable primitive.
- TLA+ finite model covers two runs and two workflows.
- Verus pure predicates trust conversion from runtime records, storage bytes, parser outputs, and CLI/runtime effects.
- Byte-level decoder behavior, actual hash/proof validation, Kani production harnesses, fuzz targets, mutation testing, and integration scenarios remain later-state evidence.

---

## State 5 Attempt 3 Evidence: Restart/Readback Repair

Inputs: State 6 attempt 3 `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, and `contract-verification-review.md` rejected `TLA-ATOM-001` because restart/readback determinism was claimed but not executable in TLA+, and because the `RecordKinds` abstraction lacked explicit family mapping.

### Path And Artifact Gates

Command: `pwd -P`
Exit: 0

```text
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission
```

Command: `test -s ".beads/vb-core-atomic-admission/STATE.md" && test -s ".beads/vb-core-atomic-admission/proof-review.md" && test -s ".beads/vb-core-atomic-admission/proof-findings.jsonl" && test -s ".beads/vb-core-atomic-admission/proof-repair-guide.md" && test -s ".beads/vb-core-atomic-admission/contract-verification-review.md" && test -s ".beads/vb-core-atomic-admission/proof-obligations.planned.jsonl" && test -s "verification/tla/AtomicAcceptedRunAdmission.tla" && test -s "verification/tla/AtomicAcceptedRunAdmission.cfg"`
Exit: 0

Command: `jq -c . ".beads/vb-core-atomic-admission/proof-findings.jsonl" >/dev/null && jq -c . ".beads/vb-core-atomic-admission/proof-obligations.planned.jsonl" >/dev/null`
Exit: 0

### TLA+ Repair Summary

- Added `restarted` to `VARIABLES` and `vars`.
- Added `Restart` action. If committed durable full records exist, restart sets `readback_decision' = "accepted"`. Otherwise restart erases staged/durable partial state, marks failed, and sets `readback_decision' = "absent"`.
- Added `WF_vars(Restart)` to the behavior spec.
- Added `RestartReadbackDeterministic` invariant to prove post-restart accepted readback depends only on full durable records.
- Added `EventuallyRestartReadbackAfterCommit` temporal property to prove committed states eventually reach restarted accepted readback under fairness.
- Removed unused `EXTENDS Naturals, FiniteSets` after TLC quota failures showed the unused standard-module extraction path was blocking execution; no model operator from those modules was used.

### TLC Command Evidence

Command: `tlc -config verification/tla/AtomicAcceptedRunAdmission.cfg verification/tla/AtomicAcceptedRunAdmission.tla`
Exit: nonzero

```text
java.io.IOException: Disk quota exceeded
Parsing file /tmp/Naturals.tla
Error: Parsing or semantic analysis failed. Module-Table lookup failure for module name AtomicAcceptedRunAdmission derived from AtomicAcceptedRunAdmission file name.
```

Classification: `BLOCK_LOCAL_TOOLING_RETRIED`. This was an environment/temp extraction failure, not a model counterexample.

Command: `java -Djava.io.tmpdir="/tmp/opencode/vb-core-atomic-admission-tlc/tmp" -cp "/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tla2tools.jar" tlc2.TLC -metadir "/tmp/opencode/vb-core-atomic-admission-tlc/states" -config "verification/tla/AtomicAcceptedRunAdmission.cfg" "verification/tla/AtomicAcceptedRunAdmission.tla"`
Exit: nonzero

```text
java.io.IOException: Disk quota exceeded
Parsing file /tmp/opencode/vb-core-atomic-admission-tlc/tmp/Naturals.tla
Error: Parsing or semantic analysis failed. Module-Table lookup failure for module name AtomicAcceptedRunAdmission derived from AtomicAcceptedRunAdmission file name.
```

Classification: `BLOCK_LOCAL_TOOLING_RETRIED`. Dedicated `/tmp/opencode` metadata still hit quota.

Command: `java -Djava.io.tmpdir="/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission/verification/tla/.tlc-states/tmp" -cp "/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tla2tools.jar" tlc2.TLC -metadir "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission/verification/tla/.tlc-states/states" -config "verification/tla/AtomicAcceptedRunAdmission.cfg" "verification/tla/AtomicAcceptedRunAdmission.tla"`
Exit: 0

```text
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Computed 2 initial states...
Finished computing initial states: 4 distinct states generated at 2026-05-15 17:41:01.
Progress(12) at 2026-05-15 17:41:01: 7,964 states generated, 1,100 distinct states found, 0 states left on queue.
Checking 3 branches of temporal properties for the complete state space with 3300 total distinct states at (2026-05-15 17:41:01)
Finished checking temporal properties in 00s at 2026-05-15 17:41:01
Model checking completed. No error has been found.
7964 states generated, 1100 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 12.
Finished in 00s at (2026-05-15 17:41:01)
```

Checked `AtomicAcceptedRunAdmission.cfg` entries after repair:

- `AllRecordsOrNoAcceptedRun`
- `NoPartialAfterFailure`
- `IndexesOnlyCommitted`
- `ReadbackOnlyCommitted`
- `RestartReadbackDeterministic`
- `NoAckBeforeCommit`
- `NoRuntimeAllocationBeforeCommit`
- `EventuallyAckOrFail`
- `EventuallyReadableAfterCommit`
- `EventuallyRestartReadbackAfterCommit`

Deadlock stance: no `CHECK_DEADLOCK FALSE` appears in `AtomicAcceptedRunAdmission.cfg`; TLC default deadlock checking remained enabled.

### Verus Regression Evidence

Command: `verus verification/verus/accepted_run_atomic_admission.rs`
Exit: 0

```text
verification results:: 6 verified, 0 errors
```

### Marker Scan Evidence

Command: `rg "CHECK_DEADLOCK|Restart ==|RestartReadbackDeterministic|EventuallyRestartReadbackAfterCommit|WF_vars\\(Restart\\)|PROPERTY" verification/tla/AtomicAcceptedRunAdmission.*`
Exit: 0

```text
verification/tla/AtomicAcceptedRunAdmission.tla:78:Restart ==
verification/tla/AtomicAcceptedRunAdmission.tla:121:  /\ WF_vars(Restart)
verification/tla/AtomicAcceptedRunAdmission.tla:137:RestartReadbackDeterministic ==
verification/tla/AtomicAcceptedRunAdmission.tla:153:EventuallyRestartReadbackAfterCommit ==
verification/tla/AtomicAcceptedRunAdmission.cfg:9:INVARIANT RestartReadbackDeterministic
verification/tla/AtomicAcceptedRunAdmission.cfg:12:PROPERTY EventuallyAckOrFail
verification/tla/AtomicAcceptedRunAdmission.cfg:13:PROPERTY EventuallyReadableAfterCommit
verification/tla/AtomicAcceptedRunAdmission.cfg:14:PROPERTY EventuallyRestartReadbackAfterCommit
```

No `CHECK_DEADLOCK` match appeared in the marker scan output.

### TLC Cleanup Evidence

Command: `rm -rf "verification/tla/.tlc-states" && test ! -e "verification/tla/.tlc-states"`
Exit: 0

Command: `rm -f "accepted_run_atomic_admission" && test ! -e "accepted_run_atomic_admission"`
Exit: 0

### Record-Family Refinement Map

- `source`: source record family in planned storage variables.
- `artifact`: accepted artifact record family.
- `header`: admission header record family.
- `run_accepted`: `RunAccepted` event family.
- `status_index`: accepted-run status index family.
- `workflow_index`: workflow-to-run index family.
- `action_index`: action-to-run index family.

The abstract `RecordKinds` set is a finite one-to-one projection of the planned per-family storage variables. `staged` means present in the pre-commit batch, and `durable` means visible after the modeled atomic commit boundary. The abstraction is sufficient for `TLA-ATOM-001` because the checked invariants reason only about whole-family presence, all-or-nothing visibility, committed-only indexes, fail-closed absence, and deterministic readback after restart; it intentionally does not claim byte codec, concrete key derivation, or production I/O behavior.
