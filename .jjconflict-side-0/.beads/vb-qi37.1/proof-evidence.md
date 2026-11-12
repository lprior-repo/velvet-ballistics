# Proof Evidence: vb-qi37.1

State 5 attempt 3 proof repair evidence from isolated workspace `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1`.

## Isolation Evidence

Command:

```bash
pwd -P && test "$PWD" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1" && case "$PWD" in /home/lewis/src/velvet-ballistics|/home/lewis/src/velvet-ballistics/*) exit 1;; esac
```

Exit status: `0`.

Output:

```text
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1
```

## Verus Evidence

Command:

```bash
mkdir -p target/tmp && TMPDIR=target/tmp verus verification/verus/recovery_verification.rs
```

Exit status: `0`.

Relevant output:

```text
verification results:: 16 verified, 0 errors
```

Checked artifact:

- `verification/verus/recovery_verification.rs`

Covered obligations:

- `PO-011` / `VERUS-PRE-005`: unsupported flags imply rejection in the pure recovery-boundary model.
- `PO-012` / `VERUS-POST-003`: unsupported durable frame seed cannot hydrate successfully in the pure boundary model.
- `PO-013` / `VERUS-POST-005`: summary-only recovery never hydrates a frame in the pure boundary model.
- `PO-014` / `VERUS-INV-002`: recovered slot/taint presence cannot coexist with unsupported fabricated state in the pure model.
- `PO-015` / `VERUS-INV-003`: pending actions and unsupported taint are rejection gates.
- `PO-016` / `VERUS-INV-005`: non-vacuous typed-error propagation/refinement via `SpecRecoveryError`, `SpecRuntimeError`, `SpecRecoveryDecision`, `SpecRuntimeDecision`, `spec_recover_frame_decision`, `spec_refine_recovery_error`, `spec_runtime_decision`, `proof_typed_recovery_errors_refine_to_runtime_errors`, `proof_typed_recovery_errors_cannot_succeed`, `proof_recovery_decision_preserves_workflow_digest_error`, `proof_recovery_decision_preserves_compiled_ir_error`, and `proof_recovery_decision_preserves_dimension_error`.
- `PO-017` / `VERUS-DIGEST-001`: scoped required digest proof through `spec_verify_required_digests` for workflow-source and compiled-IR mismatch detection only.
- `PO-019` / `ERR-002`: `proof_workflow_source_mismatch_detected` proves workflow-source mismatch rejects required digest verification in full mode.
- `PO-020` / `ERR-003`: `proof_compiled_ir_mismatch_detected` proves compiled-IR mismatch rejects required digest verification in full mode.
- `PO-027` / `ERR-010`: dimension overflow condition is detected in the pure model.
- `PO-028` / `ERR-011`: unsupported frame seed cannot hydrate successfully in the pure model.
- `PO-029` / `ERR-012`: summary-only boundary cannot hydrate successfully in the pure model.

Non-vacuity notes for `PO-016`:

- The repaired proof no longer has an ensures clause whose consequent repeats its antecedent.
- `Err(SpecRecoveryError)` inputs are refined to explicitly named `SpecRuntimeError` variants.
- `proof_typed_recovery_errors_cannot_succeed` proves an existing typed error decision cannot refine to `SpecRuntimeDecision::Ok`.
- Workflow-source, compiled-IR, and dimension overflow error paths are proven as concrete recovery-decision inputs, not only abstract enum mapping.

Digest-scope notes for `PO-017`, `PO-019`, and `PO-020`:

- `spec_verify_required_digests` contains only workflow-source and compiled-IR checks.
- `spec_verify_optional_downstream_digests` contains action ABI and policy checks and is not used as required `PO-017` evidence.
- `PO-021` and `PO-022` remain explicit waived optional downstream rows from State 4 attempt 4.

Verus notes:

- Verus emitted automatically chosen quantifier trigger notes.
- Trigger notes were informational; final verifier result was `16 verified, 0 errors`.

## TLA+ Evidence

Initial command:

```bash
TMPDIR=target/tmp tlc -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla
```

Initial exit status: non-zero.

Relevant failure output:

```text
java.io.IOException: Disk quota exceeded
Fatal errors while parsing TLA+ spec in file RecoveryHydration
```

Final command:

```bash
TMPDIR=target/tmp tlc -metadir target/tmp/tlc-metadir -config verification/tla/RecoveryHydration.cfg verification/tla/RecoveryHydration.tla
```

Final exit status: `0`.

Relevant output:

```text
Model checking completed. No error has been found.
10740192 states generated, 8405208 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 7.
```

Checked artifacts:

- `verification/tla/RecoveryHydration.tla`
- `verification/tla/RecoveryHydration.cfg`

Covered obligations:

- `PO-001` / `TLA-PRE-001`: durable input recovery or fail-closed path modeled.
- `PO-002` / `TLA-PRE-002`: mixed-run recovery rejected by `NoMixedRunRecovery` and fail-closed replay divergence.
- `PO-003` / `TLA-PRE-003`: snapshot plus strictly later tail ordering checked by `SnapshotThenTailOnly`.
- `PO-004` / `TLA-POST-001`: successful recovery requires accepted ordered durable input.
- `PO-005` / `TLA-POST-002`: frame seed requires all bounded durable facts or fail-closed unsupported state.
- `PO-006` / `TLA-POST-004`: snapshot/tail ordering invariant checked.
- `PO-007` / `TLA-POST-007`: before-ack and after-ack crash cuts remain model checked.
- `PO-008` / `TLA-INV-001`: corrupt ordering cannot become accepted frame recovery.
- `PO-009` / `TLA-INV-004`: YAML-marked journal input cannot become accepted recovery.
- `PO-010` / `TLA-INV-006`: terminal output must match durable terminal facts for accepted recovery.
- `PO-023` / `ERR-006`: replay divergence cases fail closed.
- `PO-024` / `ERR-007`: no durable data cannot produce frame success.
- `PO-026` / `ERR-009`: terminal contradiction cannot produce frame success.

Assumptions and bounds:

- `MaxSeq = 2`.
- Runs are bounded to `run_a` and `run_b`.
- Journal streams are finite explicit cases: empty, complete base facts, base plus terminal, out-of-order, mixed-run, unsupported, YAML-marked, missing frame facts, and contradictory terminal facts.
- Durable fact families are represented as bounded booleans for header, pc, slot, taint, step, action, wait, ask, retry, collect, and action ticket.
- Terminal self-loop remains the checked terminal design; `CHECK_DEADLOCK FALSE` is not present.
- Trusted boundaries: Fjall I/O ordering, snapshot byte decoding, concrete OS crash mechanics, and hash/artifact loading.

## Revised Workflow/IR Digest Scope Evidence

Production source read:

- `crates/vb_storage/src/recovery/recover.rs` lines 53-73.

Relevant source behavior:

- `DigestCheck::WorkflowSourceOnly | DigestCheck::WorkflowAndIr | DigestCheck::Full` calls `check_workflow_source_digest`.
- `DigestCheck::WorkflowAndIr | DigestCheck::Full` calls `check_compiled_ir_digest`.
- The function returns `Ok(())` after those checks.
- No action ABI or policy digest input, lookup, or comparison path exists in `verify_digests` for this bead.

Artifact digests:

```text
5fba71c1f1cdf5b00dfb699e9b1fc8cca62af3b60349fe302442423d121a221f  crates/vb_storage/src/recovery/recover.rs
036220cf4d3576b2bd178dc031981613f399b0400cf67275ed5cf4176385599d  verification/verus/recovery_verification.rs
253cbc5f3933ecec24dfc6c6cfc7226cde8411d33d8dd518fa79e68fc7f3235d  .beads/vb-qi37.1/proof-obligations.planned.jsonl
eae84a561c306d513ca067745f78fe139838792a6935f05be71023bd487112f5  .beads/vb-qi37.1/contract.md
71a18c537d6bc72b797e83876d2552fc465bc80a43f5d546ff32419d38ee497f  .beads/vb-qi37.1/verification-layers.md
```

## Artifact Validation

Command:

```bash
TMPDIR=target/tmp jq -c . .beads/vb-qi37.1/proof-obligations.planned.jsonl >/dev/null && TMPDIR=target/tmp jq -c . .beads/vb-qi37.1/proof-obligations.jsonl >/dev/null && TMPDIR=target/tmp jq -c . .beads/vb-qi37.1/traceability-matrix.jsonl >/dev/null && TMPDIR=target/tmp test -s .beads/vb-qi37.1/proof-strategy.md && TMPDIR=target/tmp test -s .beads/vb-qi37.1/proof-plan-review-input.md
```

Exit status: `0`.

## NOT_RUN

- Cargo tests, integration/fault-injection, proptest, `moon ci`, Kani, Flux, Loom, Miri, fuzz, and dependency audit were not run in State 5 attempt 3 because the planned owner rows assign those lanes to later states or waive them.
- No production code was edited.

---

## State 5 Attempt 4 Direct PRE-004 Repair Evidence

Repair target: direct `VERUS-PRE-004` / `PO-003A` evidence after State 3/4 added explicit PRE-004 rows.

### Isolation Recheck

Command:

```bash
pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac
```

Exit status: `0`.

Output:

```text
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1
```

### Direct PRE-004 Verus Evidence

Changed verification artifact only:

- `verification/verus/recovery_verification.rs`

Direct proof added:

- `proof_required_digest_preconditions_by_level`

Covered direct obligation:

- `PO-003A` / `VERUS-PRE-004`: `spec_verify_required_digests` requires workflow-source digest match for `WorkflowSourceOnly`, and requires both workflow-source and compiled-IR digest matches for `WorkflowAndIr` and `Full`.

Command:

```bash
mkdir -p target/tmp && TMPDIR=target/tmp verus verification/verus/recovery_verification.rs
```

Exit status: `0`.

Relevant output:

```text
verification results:: 17 verified, 0 errors
```

Scope notes:

- `SpecDigestCheck::Full` remains scoped to production-visible workflow-source and compiled-IR inputs for vb-qi37.1.
- Action ABI and policy digest checks remain optional downstream algebra only through `spec_verify_optional_downstream_digests`; they are not used as direct `PO-003A` evidence.
- Production source readback still shows `verify_digests` checks workflow source for `WorkflowSourceOnly | WorkflowAndIr | Full` and compiled IR for `WorkflowAndIr | Full`, then returns `Ok(())`.
- No production code, tests, dependencies, CI config, or source checkout files were edited.

### Validation And Hash Evidence

JSONL validation command:

```bash
jq -c . .beads/vb-qi37.1/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/traceability-matrix.jsonl >/dev/null
```

Exit status: `0`.

Required artifact gate:

```bash
test -s .beads/vb-qi37.1/proof-writer-report.md && test -s .beads/vb-qi37.1/proof-evidence.md && test -s .beads/vb-qi37.1/contract-verification-review.md && test -s .beads/vb-qi37.1/proof-strategy.md && test -s .beads/vb-qi37.1/proof-plan-review-input.md
```

Exit status: `0`.

Artifact digests after repair:

```text
0ae28996b1c26e8fabf93be16514ec8ad71d4d4d407552208a6fa7ab8900c7f1  verification/verus/recovery_verification.rs
5fba71c1f1cdf5b00dfb699e9b1fc8cca62af3b60349fe302442423d121a221f  crates/vb_storage/src/recovery/recover.rs
07d6b7df4e6780d3c5669c5686ebea427be62c6043e1611c59d0225345ea06b4  .beads/vb-qi37.1/proof-obligations.jsonl
0d69e84619518016a6fee909e37dda1bda451125c1e47994b231b239657473e8  .beads/vb-qi37.1/proof-obligations.planned.jsonl
```
