# Proof Strategy: vb-ahfl State 4 Repair After State 3 Scope Repair

## Status

Planning repair only. No production code, tests, proof/model/harness/spec files, dependencies, or CI config were edited.

State 4 now consumes the repaired State 3 contract as a UI artifact schema parity contract. `SCOPE-001` resolves the earlier `BLOCKER-SCOPE-001` for this artifact stack by binding all downstream proof planning to `.beads/vb-ahfl/delivery-scope.jsonl`. Engine YAML-to-IR compile/admit/run/journal/replay semantics remain excluded and require regenerated State 2/3/4/5 artifacts if selected by an owner/orchestrator.

## Inputs Read

- `.beads/vb-ahfl/STATE.md`
- `.beads/vb-ahfl/codebase-map.md`
- `.beads/vb-ahfl/delivery-scope.jsonl`
- `.beads/vb-ahfl/contract.md`
- `.beads/vb-ahfl/domain-model-review.md`
- `.beads/vb-ahfl/tla-spec.md`
- `.beads/vb-ahfl/lean-contract.md`
- `.beads/vb-ahfl/verification-layers.md`
- `.beads/vb-ahfl/proof-obligations.jsonl`
- `.beads/vb-ahfl/traceability-matrix.jsonl`
- Prior State 6 rejection context: `.beads/vb-ahfl/proof-review.md`, `.beads/vb-ahfl/proof-findings.jsonl`, `.beads/vb-ahfl/proof-repair-guide.md`, `.beads/vb-ahfl/contract-verification-review.md`
- Prior proof evidence as context only: `.beads/vb-ahfl/proof-evidence.md`, `.beads/vb-ahfl/proof-writer-report.md`
- Kani command drift evidence: `cargo kani --version` reports `cargo-kani 0.67.0`; `cargo kani --help` lists `--tests`, `--harness`, `--default-unwind`, and negative safety toggles such as `--no-overflow-checks`, but does not list `--bounds-checks` or a positive `--overflow-checks` flag.

## Discovery Commands

```bash
pwd -P
test -s ".beads/vb-ahfl/contract.md" && test -s ".beads/vb-ahfl/traceability-matrix.jsonl" && test -s ".beads/vb-ahfl/delivery-scope.jsonl" && test -s ".beads/vb-ahfl/proof-obligations.jsonl" && test -s ".beads/vb-ahfl/verification-layers.md"
rg -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates/vb_ui_model crates/vb_ui_makepad crates/velvet_ballistics velvet-ballistics-MASTER.md
rg -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates/vb_ui_model crates/vb_ui_makepad crates/velvet_ballistics verification velvet-ballistics-MASTER.md
bash -lc 'cargo metadata --format-version 1 --no-deps >/tmp/vb-ahfl-cargo-metadata.json && ! rg -n "^(\\s*(makepad|tokio|async-std|reqwest|hyper|serde_yaml|yaml-rust)\\s*=|\\s*use\\s+(makepad|tokio|async_std|reqwest|hyper|serde_yaml|yaml_rust)\\b|\\s*extern\\s+crate\\s+(makepad|tokio|async_std|reqwest|hyper|serde_yaml|yaml_rust)\\b)" crates/vb_ui_model/Cargo.toml crates/vb_ui_model/src'
```

Discovery was not blocked. The scoped source paths exist in the isolated workspace. The repaired static boundary command is intentionally dependency/import scoped so documentation comments mentioning async runtimes are not false positives.

## Risk Classification

- Scope alignment: resolved for current artifacts by `SCOPE-001`; reroute only if owner/orchestrator selects engine YAML-to-IR scope.
- Rust-local pure data invariants: Verus-owned for metadata, bounds, redaction, graph/event structure, and ordering once production-bound harnesses exist.
- Bounded canonicalization: Kani-owned after canonicalization APIs and harness bounds exist.
- Kani command drift: State 4-owned command syntax repair only. The accepted cargo-kani 0.67.0 command is `cargo kani -p vb_ui_model --tests --harness vb_ahfl_canonicalization_no_false_parity --default-unwind 8`. Production canonicalization APIs, harness discoverability, and the pre-existing missing include blocker remain State 10-owned before State 5 can claim Kani evidence.
- CLI/UI schema parity: proptest/integration-owned after authoritative emitters and canonicalization tests exist.
- Cold-path boundary: static dependency/import scan plus CI-owned; planned command now matches the State 3 repaired boundary and avoids comment-text false positives.
- Redaction malformed input: fuzz-owned later unless the owning state records a reviewed substitution.
- Public schema compatibility and typed error coverage: API compatibility and mutation-owned later.
- Temporal, theorem, Loom, Miri, Flux, and dependency-audit lanes: explicit not-applicable rows for current static UI schema parity scope, with expiry triggers.

## Strategy

1. Replace prior `MANUAL-SCOPE-001` planning language with `SCOPE-001`; treat scope as resolved for this UI artifact stack and as a regeneration trigger if changed.
2. Keep production-bound Verus, Kani, proptest, API, mutation, fuzz, static boundary, and CI obligations as required planned rows with exact downstream commands.
3. Do not treat prior abstract Verus evidence or prior State 6 not-run evidence as proof closure.
4. Use the repaired `STATIC-BOUNDARY-001` command that scans only Cargo dependency declarations and Rust imports/extern crates in `crates/vb_ui_model`.
5. Keep non-applicable lanes explicit: TLA+, Lean/Aeneas/Hax, Loom, Miri, Flux, and dependency audit remain not applicable unless scope or dependencies change.
6. Repair `KANI-CANON-001` by removing unsupported `--bounds-checks --overflow-checks`, selecting the supported integration-test route with `--tests`, and recording finite unwind bound `8`; this is not Kani pass evidence and does not move production API or harness wiring work out of State 10.

## Outputs

- `.beads/vb-ahfl/proof-obligations.planned.jsonl` is the machine-readable obligation matrix for proof-writer/reviewer.
- `.beads/vb-ahfl/proof-plan-review-input.md` summarizes reviewer checks, resolved State 6 findings, and remaining downstream proof/test/release obligations.
