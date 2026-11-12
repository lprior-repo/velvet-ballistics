# Proof Writer Report: vb-engine-yaml

## Scope

- Bead: `vb-engine-yaml`.
- State: 5 proof-writer repair, attempt 2 after repaired States 3 and 4 plus prior State 6 rejection.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`.
- Source checkout write boundary: `/home/lewis/src/velvet-ballistics` was not written.
- Edit boundary honored: verification artifacts under `verification/tla/` and evidence/state artifacts under `.beads/vb-engine-yaml/` only.
- No production source, tests, dependencies, CI config, or source-checkout files were edited.

## Inputs Read

- `.beads/vb-engine-yaml/proof-obligations.planned.jsonl`.
- `.beads/vb-engine-yaml/proof-strategy.md`.
- `.beads/vb-engine-yaml/proof-plan-review-input.md`.
- `.beads/vb-engine-yaml/contract.md`.
- `.beads/vb-engine-yaml/traceability-matrix.jsonl`.
- Prior rejection artifacts: `.beads/vb-engine-yaml/proof-review.md`, `.beads/vb-engine-yaml/proof-findings.jsonl`, `.beads/vb-engine-yaml/proof-repair-guide.md`, `.beads/vb-engine-yaml/contract-verification-review.md`.

## Verification Artifacts Repaired

- `verification/tla/EngineYamlAdmission.tla` for `PO-002`: replaced vacuous `[]AckOrFailState` with `<>(ack_state \in {"acked", "failed"})` and added `WF_vars(AdmissionProgress)`.
- `verification/tla/EngineYamlRunLifecycle.tla` for `PO-003`: added terminal snapshot variables and `NoTerminalMutationAfterTerminal` over `run_state`, `seq`, and `journal`; replaced vacuous lifecycle property with `<>(run_state \in Terminal \/ run_state = "suspended")`; added `WF_vars(LifecycleProgress)`.
- `verification/tla/EngineYamlRecovery.tla` for `PO-004`: replaced vacuous recovery property with `<>(run_state \in {"hydrated", "failed_closed"})` and added `WF_vars(RecoveryProgress)`.
- `verification/tla/EngineYamlIngress.tla` and `.cfg` for `PO-005`: added observable `full_submit_observed` and `full_submit_rejected` state, strengthened `BackpressureRejects`, added `FullQueueRejectsWithoutGrowth`, added `EventuallyAcceptsOrRejects`, and added `WF_vars(IngressProgress)`.

## Commands And Results

| Obligation | Command | Exit | Status | Evidence |
| --- | --- | --- | --- | --- |
| workspace | `pwd -P` | 0 | PASS | Returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`. |
| `PO-002` | `tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-admission -config verification/tla/EngineYamlAdmission.cfg verification/tla/EngineYamlAdmission.tla` | 0 | PASS | TLC: no error; 32 states generated, 13 distinct, depth 7; temporal properties checked. |
| `PO-003` | `tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-lifecycle -config verification/tla/EngineYamlRunLifecycle.cfg verification/tla/EngineYamlRunLifecycle.tla` | 0 | PASS | TLC: no error; 100 states generated, 31 distinct, depth 8; temporal properties checked. |
| `PO-004` | `tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-recovery -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla` | 0 | PASS | TLC: no error; 838 states generated, 387 distinct, depth 6; temporal properties checked. |
| `PO-005` | `tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-ingress -config verification/tla/EngineYamlIngress.cfg verification/tla/EngineYamlIngress.tla` | 0 | PASS | TLC: no error; 256 states generated, 87 distinct, depth 9; temporal properties checked. |
| `PO-006` | `tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-capability -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla` | 0 | PASS | TLC: no error; 478 states generated, 220 distinct, depth 3. |
| `PO-007` | `verus verification/verus/resource_budget.rs` | 0 | PASS | Verus: `verification results:: 10 verified, 0 errors`. |
| `PO-008` | `verus verification/verus/step_state_machine.rs` | 0 | PASS | Verus: `verification results:: 9 verified, 0 errors`. |
| `PO-009` | `verus verification/verus/recovery_verification.rs` | 0 | PASS_WITH_NOTES | Verus printed auto-trigger notes and `verification results:: 7 verified, 0 errors`. |
| `PO-010` | `verus verification/verus/capability_artifact_model.rs` | 0 | PASS | Verus: `verification results:: 8 verified, 0 errors`. |
| `PO-013` | `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue` | nonzero | FAIL_LOCAL | Compile fails with undeclared `Arc` in `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs:18:21` and `crates/vb_runtime/src/models/loom/shutdown_drain.rs:16:23`; proof-writer did not edit runtime model source. |
| `PO-011` | `cargo kani -p vb_compile --harness lower_accessor_reference_numeric` | nonzero | BLOCKED_PLAN_MISMATCH | Kani is installed, but no harness matched `lower_accessor_reference_numeric`. |
| `PO-012` | `cargo kani --harness engine_yaml_admission_rejects_raw_ir` | nonzero | BLOCKED_PLAN_MISMATCH | Kani is installed, but no harness matched `engine_yaml_admission_rejects_raw_ir`. |
| tooling | `cargo kani --version` | 0 | PASS | `cargo-kani 0.67.0`. |

## Assumptions And Bounds

- TLA+ models remain finite smoke models over the configured `.cfg` constants.
- Admission abstracts Fjall persistence as an atomic durable batch success or failure.
- Lifecycle fairness is scoped to model progress actions, not external action completion fairness.
- Recovery has no YAML parser action in the transition relation.
- Ingress uses `Capacity = 2` and `MaxEvents = 4`; the repaired property proves observed full-submit rejection without queue growth inside this finite model.
- Verus trusted boundaries remain those already present in the scoped Verus files; no trusted boundary was added or hidden.

## Blockers

- `PO-013` remains `FAIL_LOCAL`: the required Loom command cannot compile until runtime model files import/use the correct `Arc`. This is outside the proof-writer edit boundary because the files are under `crates/vb_runtime/src/models/loom/`.
- `PO-011` and `PO-012` remain `BLOCKED_PLAN_MISMATCH`: the planned focused Kani harness names are absent. State 4/implementation must either identify existing harness names or create the required verification harnesses in an allowed source-edit state.
- Later owner-state-11 lanes remain `NOT_RUN`: `PO-001`, `PO-014`, `PO-015`, `PO-016`, `PO-017`, `PO-018`, `PO-019`, `PO-020`, and `PO-021` through canonical CI/release evidence.

## Reviewer Guidance

- Re-review the four repaired TLA models for non-vacuity and contract parity.
- Do not approve `PO-013`, `PO-011`, or `PO-012` from this report; they need source/harness repair outside this proof-only pass or an approved replanning waiver.

## Attempt 3 Repair After State 6 Rejection

### Additional Verification Artifacts Repaired

- `PO-005`: extended `verification/tla/EngineYamlIngress.tla` and `.cfg` with `protocol_kind`, `diagnostic_class`, unsupported YAML/JSON/HTTP/text-command rejection, artifact-not-accepted rejection, `TypedOperatorOutcome`, `UnsupportedProtocolRejects`, and `UnsupportedProtocolsNeverAccepted`.
- `PO-013`: repaired missing `Arc` imports in `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs` and `crates/vb_runtime/src/models/loom/shutdown_drain.rs`.
- `PO-011`: exposed existing `crates/vb_compile/src/kani/` harness modules through `#[cfg(kani)] pub mod kani;` and repaired stale `vb_compile::` self-imports to `crate::` in harness modules.
- `PO-012`: added `crates/vb_runtime/src/kani_engine_yaml_admission.rs` plus `#[cfg(kani)]` module exposure for admission rejection harnesses.

### Attempt 3 Commands And Results

| Obligation | Command | Exit | Status | Evidence |
| --- | --- | --- | --- | --- |
| isolation | `pwd && rtk git status --short` | nonzero git status | PASS_ISOLATED | Command ran in `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`; `rtk git status` reported `fatal: not a git repository`, so this is not the source checkout. |
| tempdir | `rtk ls "target" && mkdir -p "target/tmp"` | 0 | PASS | `target/tmp` available for focused commands. |
| `PO-013` | `TMPDIR=target/tmp RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue` | 0 | PASS | `cargo test: 2 passed, 1467 filtered out`. |
| `PO-012` | `TMPDIR=target/tmp cargo kani -p vb_runtime --harness engine_yaml_admission_rejects_raw_ir` | 0 | PASS | `Manual Harness Summary: Complete - 1 successfully verified harnesses, 0 failures, 1 total.` |
| `PO-011` | `TMPDIR=target/tmp cargo kani -p vb_compile --harness lower_accessor_reference_numeric` | timeout | FOUND_BUT_TIMEOUT | Harness is now discovered; command exceeded 180s after Kani explored parser/token drop paths. No PASS claimed. |
| `PO-005` | `TMPDIR=target/tmp tlc -metadir target/tmp/tlc-ingress -config verification/tla/EngineYamlIngress.cfg verification/tla/EngineYamlIngress.tla` | nonzero | BLOCKED_ENV_QUOTA | TLC failed before model checking with `java.io.IOException: Disk quota exceeded` while resolving `/tmp/Naturals.tla`. No PASS claimed for the newly extended model. |

### Attempt 3 Status

- `PO-013`: PASS for the focused Loom command that State 6 rejected on missing `Arc`.
- `PO-012`: PARTIAL_PASS; raw-IR admission harness is present and passes. Other admission harnesses were created but not run in this focused pass.
- `PO-011`: PARTIAL_REPAIR; missing-harness rejection is fixed, but execution remains blocked by Kani runtime/timeout on the parser-heavy harness.
- `PO-005`: PARTIAL_REPAIR; model coverage gap is repaired on disk, but TLC rerun is blocked by host disk quota.

### Attempt 3 Compile Sanity Checks

- `TMPDIR=target/tmp rtk cargo check -p vb_compile` -> `BLOCKED_ENV_QUOTA`; `blake3` build script failed through `sccache` with `failed to write temporary file`.
- `TMPDIR=target/tmp rtk cargo check -p vb_runtime` -> `BLOCKED_ENV_QUOTA`; compiler failed writing `/tmp/sccache*/deps.d` with `Disk quota exceeded`.
- `TMPDIR=target/tmp rtk cargo check -p vb_runtime --config 'build.rustflags=["--cfg","kani"]'` -> `BLOCKED_ENV_QUOTA`; compiler failed writing `/tmp/sccache*/deps.d` with `Disk quota exceeded`.
- These checks do not provide PASS evidence; rerun after `/tmp` quota is repaired or `sccache` is redirected/disabled by the owning environment.
