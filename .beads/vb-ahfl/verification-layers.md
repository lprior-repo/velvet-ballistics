# Verification Layers: vb-ahfl

## Boundary

- Verus-owned kernel: pure UI artifact metadata/envelope mapping, bounds/truncation, redaction, graph/event reference, event ordering, and canonicalization relations over named production modules/types. These are required production-bound obligations; abstract-only local models and missing-target waivers do not close them.
- TLA+ temporal model: not applicable for the explicitly accepted static UI schema parity scope. Engine YAML-to-IR compile/admit/run/journal/replay lifecycle semantics are outside this contract and require regenerated State 2/3/4/5 artifacts if selected.
- Theorem projection: waived; no Lean/Aeneas/Hax kernel required.
- Runtime shell: CLI process emission, Makepad rendering, filesystem, clocks, and runtime state acquisition.
- External systems excluded from formal proof: wall-clock source, terminal formatting, UI renderer, and bd/jj metadata.

## Layer Assignment

- PRE-001 -> manual-qa + contract-verification-review because `BLOCKER-SCOPE-001` is resolved here by explicit State 2 delivery-scope acceptance for UI artifact schema parity; scope changes require regeneration rather than hidden proof debt.
- PRE-002, POST-001, INV-001 -> Verus + proptest + API compatibility.
- PRE-003, POST-005, INV-003 -> Verus + Kani + proptest.
- PRE-004, POST-007, INV-002 -> Verus + proptest + mutation + CLI integration tests.
- PRE-005, POST-006, INV-004 -> Verus + Kani + fuzz/proptest + static scan.
- PRE-006, POST-008, INV-007 -> static scan + clippy + dependency tree audit + moon ci.
- POST-002, POST-003, POST-004, INV-005, INV-006 -> Verus + Kani/proptest + CLI/UI parity tests.
- ERR-* -> typed error scenario tests + mutation.
- API-001 -> API compatibility for public `vb_ui_model` types and CLI JSON/JSONL contract.
- REL-001 -> release-provenance/supply-chain gates through `moon ci` or repository release lane.

## Verus Scope

- Rust target: `crates/vb_ui_model/src/envelope/types.rs::MetadataEnvelope`, `EnvelopeKind`, exported UI model structs in `workflow.rs`, `run.rs`, `verify.rs`, and `incident.rs`, and CLI envelope kind mapping in `crates/velvet_ballastics/src/cli_envelope.rs`.
- Spec/proof functions: metadata completeness, bounded collection length, redaction no-raw-secret projection, graph reference validity, event ordering, and deterministic canonicalization.
- Invariants: INV-001 through INV-006.
- Trusted boundary: validated constructors/converters from CLI JSON/JSONL and UI model data into canonical artifact types.
- Shell exclusions: CLI process I/O, Makepad rendering, wall-clock time, storage, runtime execution, YAML parsing, HTTP, async scheduling.

## TLA+ Scope

- Module/model path: none for accepted static UI schema parity scope.
- Variables/actions/properties: not applicable under WAIVED-TLA-001 because there is no lifecycle/protocol in the accepted scope.
- Fairness/deadlock stance: not applicable because no temporal protocol is scoped.
- Refinement boundary: if scope changes, Rust runtime journal events must refine YAML compile/admit/run/replay model actions.
- Evidence command: none for current scope; regenerate if owner/orchestrator changes scope to engine YAML-to-IR or async/lifecycle behavior.

## Theorem Scope

- Theorem module: none.
- Rust target: none.
- Abstraction relation: Verus-owned abstractions suffice.
- Shell exclusions: all I/O and rendering.
- Non-goals: Lean proof for UI data shape parity.

## Exact Evidence Commands Known At Contract Time

- `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-ahfl --json` for bead reality.
- `moon ci` for repository canonical CI gate after implementation.
- `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings` for production/source lint if State 4 confirms this command is compatible with current workspace tooling.
- `bash -lc 'cargo metadata --format-version 1 --no-deps >/tmp/vb-ahfl-cargo-metadata.json && ! rg -n "^(\\s*(makepad|tokio|async-std|reqwest|hyper|serde_yaml|yaml-rust)\\s*=|\\s*use\\s+(makepad|tokio|async_std|reqwest|hyper|serde_yaml|yaml_rust)\\b|\\s*extern\\s+crate\\s+(makepad|tokio|async_std|reqwest|hyper|serde_yaml|yaml_rust)\\b)" crates/vb_ui_model/Cargo.toml crates/vb_ui_model/src'` for cold-path dependency/import boundary scanning after implementation; this intentionally ignores documentation/comment text.

## Production-Bound Obligation Policy

Required production-bound obligations are not waived in this repaired State 3 stack. State 5/7/8/10/12 must either execute the exact command named in `proof-obligations.jsonl` with raw evidence, create the named verification/test harness without changing production semantics, or report a blocking target-discovery failure. Abstract-only local proof evidence is context only.

- Verus production harnesses: `verification/verus/vb_ahfl_metadata_envelope_production.rs`, `verification/verus/vb_ahfl_bounds_production.rs`, `verification/verus/vb_ahfl_redaction_production.rs`, and `verification/verus/vb_ahfl_graph_events_production.rs`.
- Kani production harness: `vb_ahfl_canonicalization_no_false_parity`.
- Property test target: `cargo test -p velvet_ballastics --test vb_ahfl_cli_ui_parity -- --nocapture`.
- Static boundary target: dependency/import scan only, comment text ignored.
- API, fuzz, mutation, and CI commands are exact planned obligations and remain blocking until their owner states execute or explicitly re-review them.

## Waivers

- WAIVED-TLA-001: no temporal model for accepted static schema parity scope; expires if engine YAML-to-IR or async/lifecycle scope is selected.
- WAIVED-LEAN-001: no theorem kernel required; Verus owns pure data proofs.
- WAIVED-PERF-001: no performance or zero-cost claims in this bead scope.
- WAIVED-TARGETS-001: removed. Missing production-bound targets are blockers for the owning state, not waivers.

## State 3 Repair Evidence

- Isolation command: `pwd -P` -> `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-ahfl`.
- Repaired static boundary command was executed in the isolated workspace and returned no matches, avoiding the State 6 false positive from comment text.
