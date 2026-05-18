# Proof Evidence: vb-engine-yaml

## Attempt

- State: 5 proof-writer repair.
- Attempt: 2.
- Workspace command: `pwd -P`.
- Workspace command exit: 0.
- Workspace command output: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`.

## Artifact Map

- `PO-002`: `verification/tla/EngineYamlAdmission.tla`, `verification/tla/EngineYamlAdmission.cfg`; PASS.
- `PO-003`: `verification/tla/EngineYamlRunLifecycle.tla`, `verification/tla/EngineYamlRunLifecycle.cfg`; PASS.
- `PO-004`: `verification/tla/EngineYamlRecovery.tla`, `verification/tla/EngineYamlRecovery.cfg`; PASS.
- `PO-005`: `verification/tla/EngineYamlIngress.tla`, `verification/tla/EngineYamlIngress.cfg`; PASS.
- `PO-006`: `verification/tla/CapabilityLifecycle.tla`, `verification/tla/CapabilityLifecycleAll.cfg`; PASS, unchanged artifact with fresh command evidence.
- `PO-007`: `verification/verus/resource_budget.rs`; PASS, unchanged artifact with fresh command evidence.
- `PO-008`: `verification/verus/step_state_machine.rs`; PASS, unchanged artifact with fresh command evidence.
- `PO-009`: `verification/verus/recovery_verification.rs`; PASS_WITH_NOTES, unchanged artifact with fresh command evidence.
- `PO-010`: `verification/verus/capability_artifact_model.rs`; PASS, unchanged artifact with fresh command evidence.
- `PO-011`: planned focused Kani harness absent; BLOCKED_PLAN_MISMATCH.
- `PO-012`: planned focused Kani admission harness absent; BLOCKED_PLAN_MISMATCH.
- `PO-013`: Loom command compile failure remains; FAIL_LOCAL.

## Raw Command Evidence

```text
$ pwd -P
exit=0
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml

$ tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-admission -config verification/tla/EngineYamlAdmission.cfg verification/tla/EngineYamlAdmission.tla
exit=0
Checking temporal properties for the complete state space with 13 total distinct states.
Model checking completed. No error has been found.
32 states generated, 13 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 7.

$ tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-lifecycle -config verification/tla/EngineYamlRunLifecycle.cfg verification/tla/EngineYamlRunLifecycle.tla
exit=0
Checking temporal properties for the complete state space with 31 total distinct states.
Model checking completed. No error has been found.
100 states generated, 31 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 8.

$ tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-recovery -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla
exit=0
Checking temporal properties for the complete state space with 387 total distinct states.
Model checking completed. No error has been found.
838 states generated, 387 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 6.

$ tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-ingress -config verification/tla/EngineYamlIngress.cfg verification/tla/EngineYamlIngress.tla
exit=0
Checking temporal properties for the complete state space with 87 total distinct states.
Model checking completed. No error has been found.
256 states generated, 87 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 9.

$ tlc -metadir .beads/vb-engine-yaml/attempt2-tlc-capability -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla
exit=0
Model checking completed. No error has been found.
478 states generated, 220 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 3.

$ verus verification/verus/resource_budget.rs
exit=0
verification results:: 10 verified, 0 errors

$ verus verification/verus/step_state_machine.rs
exit=0
verification results:: 9 verified, 0 errors

$ verus verification/verus/recovery_verification.rs
exit=0
note: Verus printed automatically chosen quantifier triggers.
verification results:: 7 verified, 0 errors

$ verus verification/verus/capability_artifact_model.rs
exit=0
verification results:: 8 verified, 0 errors

$ RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue
exit=nonzero
error[E0433]: cannot find type `Arc` in this scope
  --> crates/vb_runtime/src/models/loom/timer_fired_cancel.rs:18:21
error[E0433]: cannot find type `Arc` in this scope
  --> crates/vb_runtime/src/models/loom/shutdown_drain.rs:16:23
full output: ~/.local/share/rtk/tee/1778880068_cargo_test.log

$ cargo kani --version
exit=0
cargo-kani 0.67.0

$ cargo kani -p vb_compile --harness lower_accessor_reference_numeric
exit=nonzero
Manual Harness Summary:
error: no harnesses matched the harness filter: `lower_accessor_reference_numeric`

$ cargo kani --harness engine_yaml_admission_rejects_raw_ir
exit=nonzero
Manual Harness Summary:
error: no harnesses matched the harness filter: `engine_yaml_admission_rejects_raw_ir`
```

## Status Ledger

- PASS: `PO-002`, `PO-003`, `PO-004`, `PO-005`, `PO-006`, `PO-007`, `PO-008`, `PO-009`, `PO-010`.
- FAIL_LOCAL: `PO-013`, because the required Loom command still fails to compile existing runtime model files.
- BLOCKED_PLAN_MISMATCH: `PO-011`, `PO-012`, because planned focused Kani harness names are absent.
- NOT_RUN owner-state-11 lanes: `PO-001`, `PO-014`, `PO-015`, `PO-016`, `PO-017`, `PO-018`, `PO-019`, `PO-020`, `PO-021`.

## No Hidden Claims

- No PASS is claimed for Loom, Kani, Moon CI, Miri, fuzz, mutation, coverage, supply-chain, or performance lanes.
- Prior raw evidence is cited only where unchanged artifacts were rerun in this attempt and the exact path is named above.

## Attempt 3 Raw Command Evidence

```text
$ pwd && rtk git status --short
exit=nonzero for git status
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml
fatal: not a git repository (or any parent up to mount point /)
Stopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).

$ rtk ls "target" && mkdir -p "target/tmp"
exit=0
target/ existed; target/tmp available.

$ TMPDIR=target/tmp RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue
exit=0
cargo test: 2 passed, 1467 filtered out (9 suites, 0.00s)

$ TMPDIR=target/tmp cargo kani -p vb_runtime --harness engine_yaml_admission_rejects_raw_ir
exit=0
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

$ TMPDIR=target/tmp cargo kani -p vb_compile --harness lower_accessor_reference_numeric
exit=timeout after 180000 ms
Harness is discovered as kani::vb_compile_accessor::lower_accessor_reference_numeric.
Observed Kani exploration of parser/token drop paths and recursion; no PASS claimed.
Full output: /home/lewis/.local/share/opencode/tool-output/tool_e2dd190e30019k1v3L22j7ADiU

$ TMPDIR=target/tmp tlc -metadir target/tmp/tlc-ingress -config verification/tla/EngineYamlIngress.cfg verification/tla/EngineYamlIngress.tla
exit=nonzero
java.io.IOException: Disk quota exceeded
Parsing file /tmp/Naturals.tla
Error: Parsing or semantic analysis failed.
```

## Attempt 3 Status Ledger

- PASS: `PO-013` focused Loom bounded queue command.
- PASS: `PO-012` focused `engine_yaml_admission_rejects_raw_ir` Kani harness.
- FOUND_BUT_TIMEOUT: `PO-011` `lower_accessor_reference_numeric`; missing harness rejection is repaired, but proof did not complete.
- BLOCKED_ENV_QUOTA: `PO-005` extended ingress TLA rerun; no TLC PASS claimed for the new model until disk quota is resolved.

## Attempt 4 New Evidence (2026-05-16)

### PO-005 TLC Ingress Rerun
```
$ TMPDIR=target/tmp tlc -metadir target/tmp/tlc-ingress-attempt3 -config verification/tla/EngineYamlIngress.cfg verification/tla/EngineYamlIngress.tla
exit=0
Model checking completed. No error has been found.
2234 states generated, 447 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 9.
The average outdegree of the complete state graph is 1 (minimum is 0, the maximum 10 and the 95th percentile is 5).
Finished in 01s
```
RESULT: PASS - BLOCKED_ENV_QUOTA resolved with TMPDIR workaround.

### PO-011 Kani Sub-Harness Status
Corrected harness name mapping (plan vs actual):

| Planned Name | Actual Name | Package | Result | Time |
|---|---|---|---|---|
| accessor_index_assignment | accessor_index_assignment | vb_compile | PASS | 17s |
| rejects_non_numeric_accessor_path | rejects_non_numeric_accessor_path | vb_compile | PASS | 8s |
| compile_expr_to_bytecode_overflow | compile_expr_to_bytecode_overflow | vb_compile | PASS | 234s |
| lower_slot_reference_with_path_creates_accessor | lower_slot_reference_with_path_creates_accessor | vb_compile | PASS | 4s |
| idempotency_gate_parity | idempotency_gate_parity | vb_compile | PASS | 0.3s |
| kani_div_by_zero_returns_error | kani_div_by_zero_returns_error | vb_core | PASS | 39s |
| harness_new_valid_capacity | harness_new_valid_capacity | vb_core | PASS | 3.5s |
| harness_push_with_room | harness_push_with_room | vb_core | PASS | 16s |
| lower_accessor_reference_numeric | lower_accessor_reference_numeric | vb_compile | TIMEOUT | - |
| push_constant_overflow | push_constant_overflow | vb_compile | TIMEOUT | - |
| push_constant_isolation | push_constant_isolation | vb_compile | TIMEOUT | - |
| slot_count_overflow_at_max | slot_count_overflow_at_max | vb_compile | FAIL_ALLOC | - |
| lower_slot_reference_valid | lower_slot_reference_valid | vb_compile | FAIL_ALLOC | - |
| node_id_uniqueness | node_id_uniqueness | vb_compile | FAIL_ALLOC | - |
| expression_stack_capacity_respects_limit | (none - actual names differ) | vb_core | PLAN_MISMATCH | - |

### Attempt 4 Status Ledger

- PASS: `PO-005` TLC ingress (447 distinct states, extended model)
- PASS: `PO-012` Kani admission
- PASS: `PO-013` Loom
- PARTIAL: `PO-011` - 8 sub-harnesses pass, 3 timeout, 3 fail alloc, 1 plan mismatch
- BLOCKED: `PO-011` `lower_accessor_reference_numeric`, `push_constant_overflow`, `push_constant_isolation` - Kani timeout
- PLAN_MISMATCH: `PO-011` vb_core harness names don't match planned names

## Attempt 3 Compile Sanity Evidence

```text
$ TMPDIR=target/tmp rtk cargo check -p vb_compile
exit=nonzero
error: failed to run custom build command for `blake3 v1.8.5`
sccache: caused by: Compiler not supported: "failed to write temporary file"
full output: ~/.local/share/rtk/tee/1778885426_cargo_check.log

$ TMPDIR=target/tmp rtk cargo check -p vb_runtime
exit=nonzero
error: error writing dependencies to `/tmp/sccachenxSr9n/deps.d`: Disk quota exceeded (os error 122)
full output: ~/.local/share/rtk/tee/1778885425_cargo_check.log

$ TMPDIR=target/tmp rtk cargo check -p vb_runtime --config 'build.rustflags=["--cfg","kani"]'
exit=nonzero
error: error writing dependencies to `/tmp/sccacheM5nWoX/deps.d`: Disk quota exceeded (os error 122)
full output: ~/.local/share/rtk/tee/1778885422_cargo_check.log
```
