# Boundary Map: Verifier Tooling Contract

## Pure Core

- Parse lane labels into `LaneId`.
- Validate command specs against lane contracts.
- Classify raw command summaries into `EvidenceClassification` plus `ApplicableCount`.
- Enforce illegal-state combinations in `VerifierExit`.

## Imperative Shell

- Resolve executables/scripts/jars.
- Read Cargo metadata/features and fuzz target inventory.
- Execute verifier commands.
- Capture raw logs and hashes.

## Tool Boundaries

- **Kani**: `cargo kani`, `scripts/kani-list.sh`, package feature graph.
- **Flux**: `cargo flux`, `scripts/flux-check-package.sh`.
- **TLA/TLC**: `tlc`, `java -jar`, `TLA2TOOLS_JAR`, `.moon/tasks/tlc.yml`, `scripts/run-tlc-checks.sh`.
- **Proptest**: `cargo test`/nextest output parser.
- **Fuzz**: `cargo fuzz list/build/run`, `fuzz/Cargo.toml`, sanitizer target triple.
- **Loom**: Cargo cfg/dependency graph, `RUSTFLAGS=--cfg loom`, model tests.

## Data Boundaries

- Inputs: delivery scope JSONL, codebase map, baseline report, proof command specs from later states.
- Outputs: contract artifacts and proof seeds only.
- No production runtime state, storage schema, IPC, or workflow IR is modified by this contract.

## Trust Boundaries

- Tool version checks are trusted only as setup health.
- Wrapper scripts are untrusted until availability and behavior are checked.
- Prior capped evidence is contextual and cannot be reused as fresh pass evidence.
- Agent-generated classifications require raw command output references downstream.
