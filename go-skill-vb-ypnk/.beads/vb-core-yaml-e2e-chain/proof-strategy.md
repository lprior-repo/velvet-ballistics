# Proof Strategy: vb-core-yaml-e2e-chain State 4 Attempt 3

## Scope

- Planning only. No production code, tests, proof/model/harness/spec files, dependency files, or source checkout files were edited.
- Workspace verified by `pwd -P`: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`.
- Source checkout use: none for writes; no source checkout reads were needed beyond existing isolated artifacts.

## Inputs Read

- Repaired State 3: `contract.md`, `domain-model-review.md`, `tla-spec.md`, `lean-contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`.
- Scope/context: `STATE.md`, `baseline-report.md`, `codebase-map.md`, `delivery-scope.jsonl`.
- State 6 rejection/repair context: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`.
- Prior proof evidence as context only: `proof-evidence.md`, `proof-writer-report.md`.

## Discovery Commands

- `pwd -P` -> `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`.
- `test -s ".beads/vb-core-yaml-e2e-chain/contract.md" && test -s ".beads/vb-core-yaml-e2e-chain/traceability-matrix.jsonl" && test -s ".beads/vb-core-yaml-e2e-chain/delivery-scope.jsonl"` -> exit 0.
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" <scoped delivery paths>` -> exit 0; 1473 matches in 77 files.
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" <scoped delivery paths plus verification/tla verification/verus verification/kani>` -> exit 0; 426 matches in 88 files.
- Blocked discovery commands: none.

## Risk Classification

- Temporal/persistence: required. TLA+ owns strict admission ordering, persist-before-ack, journal prefix durability, recovery input boundaries, and no YAML reparse after admission.
- Rust-local digest role invariants: required. Verus owns pure source/artifact digest role separation and mismatch classification, with explicit shell waivers and required executable compensation.
- Bounded admission matrix: required. Kani remains blocked until the harness is discoverable by `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix`.
- Parser/codec malformed input: required for strict YAML tests, recovery corruption tests, and Miri over codec/recovery paths. Fuzz is waived because no bead-specific target was discovered.
- Executable behavior: required. CLI, storage, runtime, and recovery integration tests must prove durable events/inspect/recovery evidence and exact fail-closed taxonomy.
- Concurrency/Loom: not applicable unless downstream implementation introduces scoped concurrency primitives; TLA+ covers persistence ordering.
- Flux: not applicable because Verus is the selected refinement lane and prior tool discovery recorded `cargo flux` unavailable.

## Planning Decision

- Refresh planned obligations from repaired State 3, not from the rejected State 4/5/6 plan.
- Keep all release-critical contract clauses mandatory unless explicitly `blocked_tooling`, `waived`, or `not_applicable` with owner, reason, expiry, and compensating evidence.
- Do not claim any verifier has passed. Prior PASS/BLOCKED evidence is context only.

## Required Lanes

- `tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla`
- `verus verification/verus/yaml_e2e_digest_roles.rs`
- `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix` as a known blocked Kani lane until harness discovery is repaired.
- `cargo test -p vb_compile -- --nocapture`
- `cargo test -p vb_storage -- --nocapture`
- `cargo test -p vb_runtime -- --nocapture`
- `cargo test -p velvet_ballastics --test cli_integration -- --nocapture`
- `cargo test -p velvet-ballastics-workspace --test vb_qi37_1_1_red_recovery_contract_test -- --nocapture`
- `cargo +nightly miri test -p vb_storage`
- `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings`
- `moon ci`

## Outputs

- Machine-readable obligations: `.beads/vb-core-yaml-e2e-chain/proof-obligations.planned.jsonl`.
- Reviewer packet: `.beads/vb-core-yaml-e2e-chain/proof-plan-review-input.md`.
