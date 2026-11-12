# Verification Layers: vb-l2d7

## Boundary
- Verified kernel: documentation consistency and abstract taint semantics for resolved DRIFT-1 wording.
- Lean contract projection: finite taint lattice and abstract data-flow propagation only.
- Runtime shell: actual Rust implementation, tests, generated mode, journal behavior, and CI evidence.
- External systems excluded from formal proof: bd status, git pushes, source-control metadata, human review decisions.

## Layer Assignment
- PRE-001 -> manual-qa + static-scan
- PRE-002 -> manual-qa + static-scan
- PRE-003 -> manual-qa
- PRE-004 -> static-scan over master doc before edit
- PRE-005 -> manual-qa evidence review
- POST-001 -> lean + proptest/Kani companion obligation + static-scan + manual-qa + gauntlet-proof + gauntlet-standard
- POST-002 -> static-scan + mutation-style seeded contradiction check
- POST-003 -> static-scan + manual-qa
- POST-004 -> manual-qa + release-provenance-style evidence audit waiver; no release artifact claimed
- POST-005 -> static-scan + manual-qa
- INV-001 -> manual-qa + static-scan + traceability review
- INV-002 -> lean + executable Rust lattice-law companion evidence (`cargo test -p vb_core taint_join_laws`) + optional/required-if-present Kani harness + gauntlet-proof
- INV-003 -> lean + executable doc consistency scan + gauntlet-standard + manual-qa
- INV-004 -> manual-qa + static-scan
- INV-005 -> exact lint/static gates for later code changes: `moon run :lint-src`, `moon run :check`, `moon run :verify-standard`, and audit of `Cargo.toml` lints forbidding unsafe, unwrap/expect/panic/todo/unimplemented/dbg, indexing/slicing, unchecked arithmetic, unchecked casts, and ignored must-use results
- ERR-001 WrongWorkspace -> manual-qa path review
- ERR-002 OutOfScopeChange -> manual-qa diff review
- ERR-003 StaleCleanOnlyTaintText -> static-scan seeded with stale phrases
- ERR-004 UnsupportedEvidenceClaim -> manual-qa evidence audit
- ERR-005 TaintVocabularyConflict -> static-scan vocabulary audit
- ERR-006 ControlFlowTaintConflation -> static-scan + manual-qa
- ERR-007 MissingTraceability -> JSONL validation + manual-qa

## Exact Evidence Expectations
- Static scan evidence: downstream report showing no contradictory `Always Clean`, `no taint join`, or `write_slot`-only wording remains for `EvalExpr`, `BuildObject`, or `BuildList` in normative taint sections.
- Evidence-bounded wording evidence: downstream report listing every DRIFT-1 implementation-evidence sentence and the concrete cited artifact, or marking it as pending/unverified.
- JSONL evidence: `proof-obligations.jsonl` and `traceability-matrix.jsonl` parse as one JSON object per line.
- Moon lane evidence: `moon run :verify-standard` after doc/test work if downstream states add executable checks; `moon run :verify-proof` only if Lean proof files are later added.
- Lean companion evidence: each Lean-owned clause must also have executable realization evidence. INV-002 requires `cargo test -p vb_core taint_join_laws` and Kani when a harness exists or is added. POST-001 requires `cargo test -p vb_runtime joined_taint_propagation` or equivalent named workspace tests. INV-003 requires a checked-in doc consistency command, preferred `python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md`.
- INV-005 evidence: `moon run :lint-src` includes clippy denies for unwrap/expect/panic/todo/unimplemented/dbg; `Cargo.toml` workspace lints deny `unsafe_code`, `panic_in_result_fn`, `indexing_slicing`, `string_slice`, `get_unwrap`, `arithmetic_side_effects`, `as_conversions`, and `let_underscore_must_use`; `moon run :check` and `moon run :verify-standard` must pass after any later code change.

## Lean Scope
- Theorem module: `Velvet.TaintLattice`, `Velvet.TaintPropagation`.
- Spec target: master-doc taint semantics and abstract node propagation table.
- Abstraction relation: documentation statements map to finite lattice/join model and node-specific propagation model.
- Shell exclusions: code conformance, CI status, generated Rust parity, source-to-IR lowering, runtime journal behavior.
- Non-goals: release proof, performance proof, API compatibility, SBOM, vectorization, runtime concurrency.

## Five-Lane Gauntlet Mapping
- `moon run :verify-fast`: acceptable quick evidence for JSONL parsing and doc scan scripts if such scripts exist downstream.
- `moon run :verify-standard`: required downstream acceptance lane for documentation reconciliation if executable checks are added.
- `moon run :verify-deep`: optional for mutation/coverage strength if doc consistency checks become tests.
- `moon run :verify-proof`: required only if Lean artifacts are implemented later.
- `moon run :verify-all`: release-level gate; not required by State 1 and not a claim of this bead.

## Waivers
- Clause ID: PRE-001, PRE-002, PRE-003, POST-001, POST-002, POST-003, POST-004, POST-005, INV-001, INV-002, INV-003, INV-004, INV-005, ERR-001, ERR-002, ERR-003, ERR-004, ERR-005, ERR-006, ERR-007. Waived layer: fuzzing/Bolero. Reason: no parser, codec, protocol, or hostile byte input surface is modified by this documentation-only bead. Compensating evidence: static doc scan, manual QA, JSONL parse, and Moon lanes named above. Owner: State 2 documentation agent for bead `vb-l2d7`. Expiry/follow-up: expires immediately if downstream scope adds a parser, codec, protocol, binary format, or hostile-input surface.
- Clause ID: PRE-001, PRE-002, PRE-003, POST-001, POST-002, POST-003, POST-004, POST-005, INV-001, INV-002, INV-003, INV-004, INV-005, ERR-001, ERR-002, ERR-003, ERR-004, ERR-005, ERR-006, ERR-007. Waived layer: Loom/Shuttle/Lockbud. Reason: no async task, thread, channel, shared mutable state, cancellation, or concurrent transition is modified by this documentation-only bead. Compensating evidence: diff scope review and `moon run :verify-standard`; if code with concurrency markers is touched, this waiver is invalid and deep/proof concurrency gates must be selected. Owner: State 2 documentation agent for bead `vb-l2d7`. Expiry/follow-up: expires immediately if downstream scope touches runtime concurrency, async scheduling, cancellation, channels, locks, or shared mutable state.
- Clause ID: PRE-001, PRE-002, PRE-003, POST-001, POST-002, POST-003, POST-004, POST-005, INV-001, INV-002, INV-003, INV-004, ERR-001, ERR-002, ERR-003, ERR-004, ERR-005, ERR-006, ERR-007. Waived layer: Miri/cargo-careful. Reason: no Rust code, unsafe-sensitive path, aliasing-sensitive logic, FFI, or layout-sensitive behavior is modified by State 1. Compensating evidence: manual diff review; if later code is touched, `moon run :verify-deep` supplies Miri/cargo-careful where applicable. Owner: State 1 contract agent for State 1 artifacts; downstream code owner if scope expands. Expiry/follow-up: expires immediately if any Rust source, unsafe-sensitive code, FFI, pointer/aliasing, or layout-sensitive behavior is touched.
- Clause ID: INV-005. Waived layer: Miri/cargo-careful for State 1 only. Reason: State 1 touches no code, but later code work must not rely on this waiver. Compensating evidence: later code changes require `moon run :lint-src`, `moon run :check`, `moon run :verify-standard`, and if unsafe/FFI markers exist `moon run :verify-deep` or `CAREFUL_REQUIRED=1` as appropriate. Owner: any downstream implementation agent touching code for bead `vb-l2d7`. Expiry/follow-up: expires immediately when code is touched.
- Clause ID: PRE-001, PRE-002, PRE-003, POST-001, POST-002, POST-003, POST-004, POST-005, INV-001, INV-002, INV-003, INV-004, INV-005, ERR-001, ERR-002, ERR-003, ERR-004, ERR-005, ERR-006, ERR-007. Waived layer: performance/assembly-ir/api-compat/release-provenance. Reason: this bead makes no performance, zero-cost, vectorization, public API, ABI, SBOM, or release artifact claims. Compensating evidence: explicit non-goals in `contract.md` and evidence-bounded wording audit. Owner: State 2 documentation agent for bead `vb-l2d7`. Expiry/follow-up: expires immediately if downstream wording adds performance, assembly, API compatibility, ABI, SBOM, supply-chain, or release-provenance claims.
