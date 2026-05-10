# Verification Layers: vb-qi37.7.3

## Boundary
- Verified kernel: pure reference and resource predicates for compiled numeric IR.
- Lean contract projection: `lean-contract.md` owns critical deterministic invariants INV-001 through INV-008.
- Runtime shell: Rust traversal, error construction, diagnostic rendering, API behavior, and integration of core/verifier gates.
- Excluded from formal proof: external I/O, runtime dispatch execution, YAML/string reference validation, storage, async runtime behavior, and performance claims.

## Layer Assignment
- PRE-001: unit + integration + proptest; untrusted malformed `WorkflowParts` must be rejected without panic.
- PRE-002: miri + static-scan; borrowed inputs are not mutated and no global state is touched.
- PRE-003: unit + static-scan; fallible admission uses `Result` and does not panic.
- PRE-004: integration + mutation; `validate`/`validate_with_contracts` boundary remains observable.
- PRE-005: proptest + fuzz + kani; numeric IDs and resource fields are generated across bounds.
- PRE-006: static-scan; no JSON/YAML/HTTP/filesystem/network lookup enters runtime core.
- POST-001: lean + unit + proptest + kani.
- POST-002: lean + unit + proptest.
- POST-003: lean + unit + integration + mutation.
- POST-004: integration + mutation.
- POST-005: lean + unit + proptest + kani.
- POST-006: lean + unit + proptest + mutation.
- POST-007: lean + integration + proptest.
- POST-008: lean + unit + proptest + kani.
- POST-009: unit + integration + mutation + static-scan.
- INV-001: lean + unit + proptest + kani.
- INV-002: lean + unit + proptest.
- INV-003: lean + unit + integration + mutation.
- INV-004: lean + unit + proptest + kani.
- INV-005: lean + unit + proptest + kani.
- INV-006: lean + unit + proptest + kani.
- INV-007: lean + unit + proptest + kani.
- INV-008: lean + unit + proptest + kani.
- INV-009: static-scan + kani + cargo-llvm-cov.
- INV-010: static-scan + miri + cargo-careful + gauntlet-standard.
- ERR-001: unit + integration + mutation + diagnostic tests.
- ERR-002: unit + proptest + mutation.
- ERR-003: unit + proptest + mutation.
- ERR-004: unit + integration + mutation.
- ERR-005: unit + integration + mutation.
- ERR-006: unit + proptest + kani + mutation.
- ERR-007: unit + proptest + kani + mutation.
- ERR-008: unit + proptest + kani + mutation.
- ERR-009: unit + integration + mutation + static-scan.
- AC-001: lean + unit + proptest + kani.
- AC-002: lean + unit + proptest.
- AC-003: lean + unit + integration + proptest + kani.
- AC-004: unit + integration + mutation.
- AC-005: unit + integration + mutation.
- AC-006: integration + mutation.
- AC-007: lean + unit + proptest + kani + mutation.
- AC-008: static-scan + miri + gauntlet-standard.
- AC-009: unit + integration + mutation + cargo-llvm-cov.
- AC-010: gauntlet-fast + gauntlet-standard + gauntlet-proof + gauntlet-all.

## Tool Commands / Evidence Targets
- Lean/proof lane: `moon run :verify-proof`, evidence `formal-verification-report.md` and proof build logs.
- Fast Rust gate: `moon run :verify-fast`, evidence fast lane log.
- Standard Rust gate: `moon run :verify-standard`, evidence standard lane log including static scans.
- Deep gate: `moon run :verify-deep`, evidence fuzz/proptest/miri/mutation summaries where configured.
- Full release-critical gate: `moon run :verify-all`, evidence aggregate report.
- Kani: bounded harnesses for indexing, checked arithmetic, slot/constant/handler/resource bounds, evidence `formal-verification-report.md`.
- Miri/cargo-careful: interpreter/runtime safety for validation tests, evidence `formal-verification-report.md`.
- Proptest: generated `WorkflowParts` and contract sets exploring boundary IDs and resources.
- Fuzz/Bolero: malformed/adversarial serialized or constructed IR only if an IR decode/parser boundary is exercised; otherwise waived below.
- Mutation: `cargo-mutants` must kill mutants that remove any reference carrier, error branch, Gate 12 call, or resource comparison.
- Coverage: `cargo-llvm-cov` must show branch coverage for each error variant and success path.
- Static scans: banned constructs, unchecked indexing/arithmetic/casts, `unsafe`, panic macros, and runtime JSON/YAML/HTTP/filesystem/network use in runtime core.

## Waivers
- W-FUZZ-001: Clause: PRE-005, POST-001, POST-005, POST-006, POST-007, INV-001..INV-008, AC-001..AC-003, AC-007. Waived layer: cargo-fuzz/Bolero for pure in-memory validator only. Reason: this bead validates already-constructed `WorkflowParts` and does not add or modify a parser/codec boundary. Compensating evidence: proptest over constructed IR plus Kani bounds checks. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review; void if a serialized IR boundary is touched.
- W-LOOM-001: Clause: non-contract scope waiver. Waived layer: loom/shuttle/lockbud. Reason: no concurrency, async runtime, shared mutable state, or task scheduling is in scope for any clause. Compensating evidence: static scan confirms no new concurrency primitives. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- W-PERF-001: Clause: non-contract scope waiver. Waived layer: performance and assembly-ir. Reason: no speed, zero-cost, vectorization, or allocation-performance claim is made in this bead. Compensating evidence: deterministic bounded validation remains covered by INV-009, static scan, Kani, and coverage. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review; create a separate performance contract before making speed claims.
- W-API-001: Clause: PRE-003, PRE-004, POST-004, AC-006 if public signatures/enums remain workspace-internal; otherwise clause-specific api-compat is required. Waived layer: api-compat. Reason: public API compatibility is not a primary claim unless signatures/enums are exposed outside workspace crates. Compensating evidence: compile checks and downstream workspace tests; use `cargo semver-checks` if public crate API changes. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- W-REL-001: Clause: non-contract scope waiver. Waived layer: release-provenance. Reason: release provenance/SBOM is outside this bead and no release artifact is produced. Compensating evidence: workspace release process. Owner: Rust Contract Agent. Expiration/follow-up: before release.

## Independent Review Requirement
Downstream test planning, proof work, and implementation must not consume these artifacts until an independent reviewer writes `contract-verification-review.md` with `STATUS: APPROVED`.
