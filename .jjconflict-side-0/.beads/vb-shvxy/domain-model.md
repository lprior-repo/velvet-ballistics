# Domain Model: vb-shvxy Global Formal Verifier Tooling Blocker

## Scope

This contract models the tooling domain for restoring verifier lanes. It does not define product runtime behavior and does not authorize production/tooling edits.

## Ubiquitous Language

- **Verifier lane**: one formal or semi-formal evidence path: Kani, Flux, TLA/TLC, proptest, cargo-fuzz, or Loom.
- **Verifier command**: a fully specified executable invocation with tool availability, package/target, feature/cfg wiring, and evidence expectations.
- **Applicable test count**: number of tests/harnesses/models/targets actually selected and executed by a verifier command.
- **Vacuous success**: exit status 0 with zero applicable tests, zero harnesses, missing target, skipped model, or only tooling/version evidence.
- **Evidence classification**: typed label separating setup health, inventory, behavior proof, behavior test, fuzz smoke, model check, or blocker evidence.
- **Lane readiness**: verifier command is executable in this workspace with required tool, target triple, feature/cfg wiring, registered targets, and non-vacuous evidence checks.
- **Fail-closed verifier result**: any missing tool, absent target, unsupported selector, zero-test success, incompatible target/sanitizer, or unresolved cfg/dependency returns a typed blocker rather than pass evidence.

## Aggregates

### VerifierLaneContract

Owns one lane's readiness and evidence semantics.

- Identity: `LaneId` in `{kani, flux, tla_tlc, proptest, fuzz, loom}`.
- State: `Unavailable`, `InventoryOnly`, `Runnable`, `ExecutedNonVacuous`, `Blocked`.
- Invariants:
  - A lane cannot enter `ExecutedNonVacuous` without `applicable_count > 0`.
  - Tool version/setup output cannot be classified as behavior proof.
  - Missing wrapper/script/tool/target is `Blocked`, not `Unavailable pass`.

### VerifierCommandSpec

Represents one command before execution.

- Required fields: lane, executable, working directory, package/target selector, feature/cfg set, expected artifact kind, nonzero execution expectation, evidence classification.
- Forbidden states:
  - Kani command with feature name not declared by the selected package.
  - Flux package wrapper with unsupported `--lib`, `--test`, `--tests`, `--benches`, or `--all-targets` selectors.
  - TLC command hardcoding missing `tools/tla2tools.jar` while a PATH/TLA2TOOLS_JAR contract is intended.
  - Proptest/cargo test command that accepts `running 0 tests` as success.
  - cargo-fuzz sanitizer run without an explicit compatible target triple.
  - Loom cfg exposing library imports for an unavailable dependency.

### EvidenceRecord

Captures command output semantics.

- Required fields: command, exit code, lane, classification, applicable count, raw output path, blocker code if any.
- Invariants:
  - `classification=behavior_proof|behavior_test|model_check|fuzz_smoke` requires `applicable_count > 0`.
  - `classification=setup_health|inventory` cannot close behavior obligations.
  - Raw output must not be truncated in the contract layer; truncation is a hazard to be blocked downstream.

## Domain Invariants

- **INV-001 Non-vacuity**: Successful verifier evidence must prove at least one applicable harness/test/model/target executed.
- **INV-002 Availability is explicit**: Tool or wrapper absence is represented as a blocker with lane-specific code.
- **INV-003 Feature/cfg declaration parity**: Every requested Cargo feature or cfg-dependent dependency must be declared for the selected package build graph.
- **INV-004 Target/sanitizer compatibility**: Sanitizer fuzz lanes must use `x86_64-unknown-linux-gnu` unless a proof-planned alternative is explicitly justified.
- **INV-005 Evidence class separation**: inventory/list/version/setup logs are not behavior evidence.
- **INV-006 Portable TLA runner**: TLC commands resolve through canonical runner policy (`tlc` on PATH or `TLA2TOOLS_JAR`) and do not rely on missing repo-local jars or user-local hardcoded paths.
- **INV-007 Raw evidence preservation**: command output used as evidence must retain enough lines to show selected target count, execution count, status, and errors.

## Open Domain Decisions

1. Kani wrapper ownership: whether `scripts/kani-list.sh` remains inventory-only or passes `--harness` through to execution.
2. Kani feature contract: whether `vb_runtime/kani-artifact-version-barrier` is restored or obligations are migrated to declared feature names.
3. TLA runner policy: whether repo vendors `tools/tla2tools.jar` or standardizes on `tlc`/`TLA2TOOLS_JAR` only.
4. Zero-test guard location: command wrapper, formal-verifier evidence parser, or shared cargo-test helper.
5. Loom wiring: real Cargo feature plus optional dependency versus cfg-only package-test lane.
