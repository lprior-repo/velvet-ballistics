# vb-qi37.6 Proof Evidence

## Scope

- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6`.
- Timestamp: `2026-05-15T21:31:18Z`.
- Evidence role: go-skill State 5 proof-writer attempt 2.
- Edited artifacts: verification comments plus `.beads` proof evidence/report/state only.
- No production source, tests, dependencies, CI config, or source checkout edits.

## Isolation Evidence

Command:

```bash
pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac
```

Result: PASS, exit 0.

Output:

```text
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6
```

## Tool Discovery

```text
command -v java: /home/lewis/.local/share/mise/installs/java/26.0.1/bin/java
command -v tlc: /home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc
command -v verus: /home/lewis/.local/bin/verus
cargo kani --version: cargo-kani 0.67.0
cargo fuzz --version: cargo-fuzz 0.13.1
```

## Ledger Check

Commands:

```bash
jq -c . ".beads/vb-qi37.6/proof-obligations.planned.jsonl" >/dev/null
jq -r '.id' ".beads/vb-qi37.6/proof-obligations.jsonl" > ".tmp/vb-qi37-6-state5-a2-primary.ids" && jq -r '.id' ".beads/vb-qi37.6/proof-obligations.planned.jsonl" > ".tmp/vb-qi37-6-state5-a2-planned.ids" && rtk diff -u ".tmp/vb-qi37-6-state5-a2-primary.ids" ".tmp/vb-qi37-6-state5-a2-planned.ids"
```

Result: PASS, exit 0 for both commands.

Evidence: planned JSONL parses and primary/planned IDs match after State 4 attempt 3 repair.

## Verus Evidence

Command:

```bash
TMPDIR=.tmp RUSTC_WRAPPER= verus "verification/verus/capability_artifact_model.rs"
TMPDIR=.tmp RUSTC_WRAPPER= verus "verification/verus/capability_artifact_model.rs"
```

Result: PASS, exit 0 for original attempt and final post-edit rerun.

Output summary: `verification results:: 8 verified, 0 errors`.

Obligations covered: `VERUS-CAP-001`, `VERUS-CARD-003`, `VERUS-CERT-007`.

## TLA+ Evidence

Commands:

```bash
TMPDIR=.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=.tmp tlc -metadir ".tmp/vb-qi37-6-state5-a2-tlc-all" -config "verification/tla/CapabilityLifecycleAll.cfg" "verification/tla/CapabilityLifecycle.tla"
TMPDIR=.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=.tmp tlc -metadir ".tmp/vb-qi37-6-state5-a2-tlc-nocontract" -config "verification/tla/CapabilityLifecycleNoContract.cfg" "verification/tla/CapabilityLifecycle.tla"
TMPDIR=.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=.tmp tlc -metadir ".tmp/vb-qi37-6-state5-a2-final-tlc-all" -config "verification/tla/CapabilityLifecycleAll.cfg" "verification/tla/CapabilityLifecycle.tla"
TMPDIR=.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=.tmp tlc -metadir ".tmp/vb-qi37-6-state5-a2-final-tlc-nocontract" -config "verification/tla/CapabilityLifecycleNoContract.cfg" "verification/tla/CapabilityLifecycle.tla"
```

Result: PASS for all commands, exit 0.

Output summary for each config:

- `TLC2 Version 2.19 of 08 August 2024`.
- `Model checking completed. No error has been found.`
- `478 states generated, 220 distinct states found, 0 states left on queue.`
- `The depth of the complete state graph search is 3.`

Obligations covered: `TLA-LIFE-004`, `TLA-DENY-005`, `TLA-DRIVE-006`.

## Kani Evidence

Commands:

```bash
timeout 120s cargo kani -p vb_core --harness capability_name_grants_harness
timeout 120s cargo kani -p vb_runtime --harness check_capability_harness
```

Result: BLOCKED_TOOLING/TIMED_OUT. No PASS claimed.

Evidence summary:

- `capability_name_grants_harness` output was truncated and saved at `/home/lewis/.local/share/opencode/tool-output/tool_e2d897557001mpdJk37VmVpj6u`; visible output repeatedly showed `aborting path on assume(false)` and unwinding through `core::str::lossy::Utf8Chunks::next` through iteration `182`.
- `check_capability_harness` output was truncated and saved at `/home/lewis/.local/share/opencode/tool-output/tool_e2d8975fa001X57Q9q9h9DPeoL`; visible output repeatedly showed `aborting path on assume(false)` and unwinding through `core::str::lossy::Utf8Chunks::next` through iteration `180`.

Obligations affected: `KANI-CAP-002`, `RUNTIME-KANI-010`.

## Fuzz Evidence

Commands:

```bash
cargo fuzz run capability_name_schema -- -runs=1000
cargo fuzz run capability_contract_schema -- -runs=1000
```

Result: BLOCKED_TOOLING. No PASS claimed.

Evidence summary: both builds failed before fuzz execution with `sanitizer is incompatible with statically linked libc, disable it using -C target-feature=-crt-static` while cargo-fuzz built for `x86_64-unknown-linux-musl`.

Obligations affected: `SCHEMA-FUZZ-008`, `SCHEMA-FUZZ-009`.

## Integration Evidence

Commands:

```bash
TMPDIR=.tmp RUSTC_WRAPPER= rtk cargo test -p vb_storage submit_artifact_persists_non_empty_required_capabilities_when_contract_requires_capability --lib
TMPDIR=.tmp RUSTC_WRAPPER= sh -c 'cargo test -p vb_runtime admit_artifact_run --lib && rg -n REQUIRED_GATE_COUNT crates/vb_runtime/src/admission.rs && rg -n ArtifactInvalidGateCount crates/vb_runtime/src/admission.rs && rg -n ADMISSION_GATE_COUNT crates/vb_storage/src/admission.rs'
TMPDIR=.tmp RUSTC_WRAPPER= sh -c 'cargo test -p vb_runtime admit_artifact_run_rejects_missing_grants_without_allocation --lib && cargo test -p vb_runtime admit_artifact_run_rejects_excess_grants --lib && cargo test -p vb_runtime admit_artifact_run_preserves_non_empty_required_capabilities --lib && rg -n submit_direct_with_grants crates/vb_runtime/src/runtime.rs && rg -n submit_compiled_with_grants crates/vb_runtime/src/runtime.rs && rg -n submit_direct_with_inputs_grants_and_contracts crates/vb_runtime/src/runtime.rs && rg -n SubmitWithContracts crates/vb_runtime/src/shard/types.rs'
TMPDIR=.tmp RUSTC_WRAPPER= sh -c 'cargo test -p vb_runtime execute_do_succeeds_when_required_capability_is_granted --lib && cargo test -p vb_runtime execute_do_returns_capability_denied_when_required_capability_not_granted --lib && cargo test -p vb_runtime cat10_do_awaiting_action --lib && cargo test -p vb_runtime cat10_do_without_contract_rejects --lib && rg -n state.action_contracts crates/vb_runtime/src/shard/lifecycle/chunk_002.rs && rg -n action_contracts: crates/vb_runtime/src/shard/types.rs'
```

Results:

- `INTEG-011`: FAIL_LOCAL. Output: `journal open failed: artifact structure validation failed`. Raw rtk log path: `~/.local/share/rtk/tee/1778880597_cargo_test.log`.
- `INTEG-012`: COMMAND_EXIT_0_BUT_CONTRACT_FAIL. Runtime tests passed, but output showed `crates/vb_runtime/src/admission.rs:16:pub const REQUIRED_GATE_COUNT: u8 = 15;` and `crates/vb_storage/src/admission.rs:118:const ADMISSION_GATE_COUNT: u8 = 2;`.
- `INTEG-013`: PASS, exit 0. All three named tests passed and required API/type names were found by `rg`.
- `INTEG-014`: PASS, exit 0. All four named tests passed and required action-contract threading names were found by `rg`.

## Release Evidence

Command:

```bash
timeout 600s moon ci
```

Result: FAIL_LOCAL/BLOCKED_ENVIRONMENT. No release PASS claimed.

Output summary:

- `Tasks: 12 completed, 3 failed, 5 skipped`.
- `velvet-ballastics:source-length`: `fatal: not a git repository (or any parent up to mount point /)` and `cargo-mutants residue check failed`.
- `velvet-ballastics:test`: `error writing dependencies to /tmp/sccacheq7OckB/deps.d: Disk quota exceeded (os error 122)`.
- `velvet-ballastics:mutants-smoke`: `Disk quota exceeded (os error 122)` while writing `/tmp/cargo-mutants-vb-qi37-6-IyC23O.tmp/crates/vb_core/src/diagnostic.rs`.

Obligation affected: `GATE-016`.

## Assumptions And Boundaries

- TLA+ evidence is bounded finite-state safety evidence only; no liveness proof is claimed.
- TLA+ bounds: `CanonicalGate = 15`, `GateCounts == {0, 2, CanonicalGate}`, `CapabilityCounts == 0..2`, boolean contract and legacy-path abstraction.
- Verus evidence is pure-model evidence only. Storage I/O, runtime scheduling, UI rendering, public API behavior, Fjall persistence, postcard serialization, and filesystem durability are excluded shell boundaries.
- Kani/fuzz commands were attempted but blocked; no results are fabricated or treated as passing.
- `INTEG-011` and `INTEG-012` require production/storage repair outside proof-writer scope.
- `GATE-016` requires environment/source-control context and disk quota repair or formal-verifier classification outside proof-writer scope.

---

# State 5 Repair Evidence After State 6 Rejection

Timestamp: `2026-05-15T18:04:00Z`.

## Isolation

Command: `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac`

Result: PASS, exit 0.

Output: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6`.

## Verus/TLC Evidence

- `TMPDIR=target/tmp RUSTC_WRAPPER= verus verification/verus/capability_artifact_model.rs`: PASS, `verification results:: 8 verified, 0 errors`.
- TLC all config with `TMPDIR=target/tmp`: PASS, no invariant violation, `478 states generated`, `220 distinct states found`, depth `3`.
- TLC no-contract config with `TMPDIR=target/tmp`: PASS, no invariant violation, `478 states generated`, `220 distinct states found`, depth `3`.

## Kani Evidence

- Broad `KANI-CAP-002` command timed out/path-exploded: `/home/lewis/.local/share/opencode/tool-output/tool_e2dd8a2b1002h161AFr7maFtOA`.
- Broad `RUNTIME-KANI-010` command timed out/path-exploded: `/home/lewis/.local/share/opencode/tool-output/tool_e2dd8a2ef003ILP1U1aooY9rnD`.
- Core split Kani batch passed all six concrete harnesses: `/home/lewis/.local/share/opencode/tool-output/tool_e2ddba25f001kmNk1sMzx84GBj`.
- Runtime `check_capability_action_match_name_denies` passed with `VERIFICATION:- SUCCESSFUL`: `/home/lewis/.local/share/opencode/tool-output/tool_e2dded816001sXIA6ioQTzkaKZ`.
- Runtime remaining split Kani batch passed all five concrete harnesses: `/home/lewis/.local/share/opencode/tool-output/tool_e2ddf010b001y36xklGk5g417Z`.

## Fuzz Evidence

- `fuzz/src/lib.rs` oracles were strengthened to assert `CapabilityNameEmpty`, `CapabilityNameInvalid`, `CapabilityActionMismatch`, and `CapabilityDuplicate` as applicable.
- `cargo fuzz run capability_name_schema -- -runs=1000` with `RUSTFLAGS="-C target-feature=-crt-static"`: BLOCKED_ENVIRONMENT by disk quota before fuzz execution.
- `cargo fuzz run capability_contract_schema -- -runs=1000` with `RUSTFLAGS="-C target-feature=-crt-static"`: BLOCKED_ENVIRONMENT by disk quota before fuzz execution.
- `cargo check --manifest-path fuzz/Cargo.toml --lib`: BLOCKED_ENVIRONMENT by disk quota in `blake3`/`libfuzzer-sys`; raw log `~/.local/share/rtk/tee/1778886192_cargo_check.log`.

## Integration/Release Evidence

- `INTEG-011`: FAIL_LOCAL, raw log `~/.local/share/rtk/tee/1778886081_cargo_test.log`, failure `journal open failed: artifact structure validation failed`.
- `INTEG-012`: COMMAND_EXIT_0_BUT_CONTRACT_FAIL; runtime `REQUIRED_GATE_COUNT: u8 = 15`, storage `ADMISSION_GATE_COUNT: u8 = 2`.
- `INTEG-013`: PASS via exact runtime submit command.
- `INTEG-014`: PASS via exact Do dispatch command.
- `GATE-016`: FAIL_LOCAL/BLOCKED_ENVIRONMENT; `moon ci` reports `Tasks: 13 completed, 2 failed, 5 skipped`, with non-git source-length failure and `/tmp` disk-quota failure during `test`.

## Boundaries

- Split Kani evidence is bounded concrete implementation evidence, not full arbitrary string coverage.
- `std::mem::forget` appears only inside runtime Kani harnesses after by-reference error-class assertions to avoid proof-harness destructor path explosion for boxed diagnostic strings; it is not production behavior.
- Fuzz 1000-run obligations remain unexecuted; stronger oracles are repaired but not executed to PASS.
- Storage and release blockers remain outside proof-writer scope.

---

# State 5 Repair Evidence After State 6 Rejection Retry 4

Timestamp: `2026-05-16T04:50:36Z`.

## Isolation

Command: `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac && test -s ".beads/vb-qi37.6/proof-review.md" && test -s ".beads/vb-qi37.6/proof-findings.jsonl" && test -s ".beads/vb-qi37.6/proof-repair-guide.md"`

Result: PASS, exit 0.

Output: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6`.

## JSONL Validation

- `jq -c . .beads/vb-qi37.6/proof-obligations.jsonl >/dev/null`: PASS.
- `jq -c . .beads/vb-qi37.6/proof-obligations.planned.jsonl >/dev/null`: PASS.
- `jq -c . .beads/vb-qi37.6/traceability-matrix.jsonl >/dev/null`: PASS.
- `jq -c . .beads/vb-qi37.6/proof-findings.jsonl >/dev/null`: PASS.

## Fuzz Evidence

- `timeout 300s env TMPDIR=target/tmp RUSTC_WRAPPER= SCCACHE_DIR=target/tmp/sccache RUSTFLAGS="-C target-feature=-crt-static" cargo fuzz run capability_name_schema --target x86_64-unknown-linux-gnu -- -runs=1000`: PASS, exit 0. Output showed release build completion and execution of `target/x86_64-unknown-linux-gnu/release/capability_name_schema ... -runs=1000`.
- `timeout 300s env TMPDIR=target/tmp RUSTC_WRAPPER= SCCACHE_DIR=target/tmp/sccache RUSTFLAGS="-C target-feature=-crt-static" cargo fuzz run capability_contract_schema --target x86_64-unknown-linux-gnu -- -runs=1000`: PASS, exit 0. Output showed release build completion and execution of `target/x86_64-unknown-linux-gnu/release/capability_contract_schema ... -runs=1000`.
- Default target retry classified as environment-only: `libfuzzer-sys` failed because `x86_64-linux-musl-g++` is missing for the default `x86_64-unknown-linux-musl` fuzz target.

Obligations covered: `SCHEMA-FUZZ-008`, `SCHEMA-FUZZ-009`.

## Proof Artifact Lint Evidence

- `env TMPDIR=target/tmp RUSTC_WRAPPER= SCCACHE_DIR=target/tmp/sccache rtk cargo clippy --manifest-path fuzz/Cargo.toml --lib -- -D warnings`: PASS, exit 0, output `cargo clippy: No issues found`.
- `fuzz/src/lib.rs` repair is proof-harness-only and preserves behavior while avoiding both clippy `manual_unwrap_or_default` and an `unwrap` call.

## Integration Evidence

- `env TMPDIR=target/tmp RUSTC_WRAPPER= rtk cargo test -p vb_storage submit_artifact_persists_non_empty_required_capabilities_when_contract_requires_capability --lib`: FAIL_LOCAL. Raw log `~/.local/share/rtk/tee/1778906838_cargo_test.log`; output reports `journal open failed: artifact structure validation failed`.
- `env TMPDIR=target/tmp RUSTC_WRAPPER= sh -c 'cargo test -p vb_runtime admit_artifact_run --lib && rg -n REQUIRED_GATE_COUNT crates/vb_runtime/src/admission.rs && rg -n ArtifactInvalidGateCount crates/vb_runtime/src/admission.rs && rg -n ADMISSION_GATE_COUNT crates/vb_storage/src/admission.rs'`: COMMAND_EXIT_0_BUT_CONTRACT_FAIL. Runtime tests passed; runtime `REQUIRED_GATE_COUNT` is `15`; storage `ADMISSION_GATE_COUNT` is still `2`.

## Release Gate Evidence

- Final command: `timeout 600s env TMPDIR=target/tmp RUSTC_WRAPPER= SCCACHE_DIR=target/tmp/sccache moon ci`.
- Result: FAIL_LOCAL, `Tasks: 13 completed, 2 failed, 5 skipped`.
- Repaired during retry: fuzz proof-artifact lint, workspace-local `target/tmp`, and crate-local temp directories for `vb_codegen`, `vb_ipc`, and `vb_runtime` needed by relative `TMPDIR=target/tmp`.
- Remaining failure 1: `velvet-ballastics:source-length` reports `fatal: not a git repository (or any parent up to mount point /)` and `cargo-mutants residue check failed`; classified as environment/source-control context for this jj workspace.
- Remaining failure 2: `velvet-ballastics:test` reaches `vb_storage` admission tests and fails with `journal open failed: artifact structure validation failed`; classified as the same bead-local storage implementation blocker as `INTEG-011`.

## Disposition

- `SCHEMA-FUZZ-008`: PASS.
- `SCHEMA-FUZZ-009`: PASS.
- `INTEG-011`: FAIL_LOCAL; production/storage behavior repair required outside proof-writer state.
- `INTEG-012`: FAIL_LOCAL; production/storage gate-count alignment required outside proof-writer state.
- `GATE-016`: FAIL_LOCAL; narrowed to non-git source-length environment plus storage admission failures.
