# vb-njju Contract Specification

## Startup authority

- Read `/home/lewis/.claude/skills/rust-contract/SKILL.md`: contract-first, no implementation, map every clause to verification, emit valid JSONL obligations.
- Read `/home/lewis/.agents/skills/rust-contract/SKILL.md`: same version/content; if conflict existed, `.agents` would win. No conflict observed.

## Context

- Bead: `vb-njju` - BDD mutation/fuzz/property closure scenarios.
- Scope source: `.beads/vb-njju/codebase-map.md` and `.beads/vb-njju/delivery-scope.jsonl` in isolated workspace `/home/lewis/src/femdation-vb-njju`.
- Acceptance criteria from source bead DB, captured in `codebase-map.md` line 9:
  - `test_mutation_gate_fails_when_admission_branch_removed`
  - `test_fuzz_smoke_runs_yaml_ipc_journal_compiled_ir_targets`
  - `test_property_gate_fails_when_generated_ir_comparison_ignores_taint`
  - `test_unsafe_boundary_fuzz_missing_causes_release_gate_failure`

## Domain terms

- Acceptance catalog: public scenario rows with Given/When/Then, public surface, isolated fixture, expected outcome/error, related bead, evidence target, and deferred follow-up.
- Mutation gate: evidence lane that must prove critical mutation survivors cannot pass release closure silently.
- Admission branch: strict runtime admission/control branch whose removal must be detected by the mutation gate.
- Fuzz smoke: release-relevant fuzz evidence that runs the named targets or otherwise proves executable hostile-seed invocation; build-only target discovery is insufficient.
- Generated IR comparison: property oracle comparing generated Rust behavior to IR behavior.
- Taint: safety/flow metadata that must be part of generated-vs-IR parity, not ignored while result slots/signals still match.
- Unsafe boundary fuzz: hostile input coverage for unsafe/decoder/binary boundary surfaces required before release.

## Assumptions

- State 4 may edit tests/catalog/docs/config in the isolated workspace only; State 3 does not.
- Existing command names listed in `delivery-scope.jsonl` are valid enough for downstream execution planning; unknown exact future harness names are recorded as planned targets from the acceptance criteria.
- This bead is release-critical per every delivery-scope row, so workspace/release gate obligations are blocking unless explicitly waived by independent review.

## Open questions for State 4

- Whether `moon run :fuzz-smoke` will be upgraded to actually run hostile seeds or whether the BDD test will verify a separate exact seed-run script.
- Whether generated-vs-IR taint parity will live in `vb_codegen` proptests or workspace acceptance tests only.
- Whether unsafe boundary inventory already exposes a public machine-readable list of required boundary fuzz targets; if not, State 4 must add public test-visible evidence without private helper coupling.

## Preconditions

- PRE-001: vb-njju scenarios are added only through public acceptance-catalog surfaces and keep non-empty Given/When/Then fields.
- PRE-002: Every scenario fixture is isolated and names `vb-njju` as related bead evidence.
- PRE-003: Mutation evidence names the admission-branch scope; unrelated `diagnostic.rs` smoke mutation is not accepted as admission closure.
- PRE-004: Fuzz evidence names all required targets: `yaml_events`, `ipc_frame`, `journal_event`, and `compiled_ir`.
- PRE-005: Generated-vs-IR property evidence includes taint parity in addition to success/result/slot/signal parity.
- PRE-006: Unsafe/decoder/binary boundary release evidence includes fuzz isolation or explicit manual QA/follow-up blocker evidence per boundary.

## Postconditions

- POST-001: `test_mutation_gate_fails_when_admission_branch_removed` fails if admission-branch mutation evidence is absent, unrelated, or treated as non-blocking.
- POST-002: `test_fuzz_smoke_runs_yaml_ipc_journal_compiled_ir_targets` fails if the fuzz-smoke lane only builds targets and lacks run/seed invocation evidence for any required target.
- POST-003: `test_property_gate_fails_when_generated_ir_comparison_ignores_taint` fails if generated-vs-IR comparison can ignore taint while still passing result/slot/signal checks.
- POST-004: `test_unsafe_boundary_fuzz_missing_causes_release_gate_failure` fails release closure when any required unsafe/decoder/binary boundary lacks fuzz evidence or approved blocker/follow-up evidence.
- POST-005: Existing acceptance catalog validation still passes for all rows and rejects weak evidence dispositions.
- POST-006: Required release closure evidence is traceable from contract clause to executable test/proof obligation.

## Invariants

- INV-001: No vb-njju BDD scenario may rely on private crate internals when public quality/catalog APIs exist.
- INV-002: Build-only fuzz evidence is never equivalent to fuzz-run evidence for release-critical closure.
- INV-003: Mutation evidence for unrelated files cannot satisfy admission-branch mutation closure.
- INV-004: Taint is a first-class parity field for generated-vs-IR comparison.
- INV-005: Missing unsafe-boundary fuzz evidence is release-blocking unless converted into explicit blocker/follow-up evidence accepted by independent review.
- INV-006: Every clause has at least one planned executable verification layer or an explicit non-applicability statement.

## Error taxonomy

- `EvidenceError::MissingScenario` - required vb-njju catalog row/test is absent.
- `EvidenceError::WeakDisposition` - evidence target is empty, non-executable, or not exact enough to support the claim.
- `EvidenceError::UnrelatedMutationScope` - mutation evidence points to unrelated files or omits admission branch scope.
- `EvidenceError::BuildOnlyFuzzSmoke` - fuzz lane builds targets without run/seed evidence.
- `EvidenceError::MissingFuzzTarget` - one or more required fuzz targets are absent from manifest or smoke evidence.
- `EvidenceError::TaintParityIgnored` - generated-vs-IR comparison omits taint from equality/failure oracle.
- `EvidenceError::UnsafeBoundaryFuzzMissing` - a required boundary lacks fuzz evidence or approved blocker/follow-up.
- `EvidenceError::ReleaseGateWouldPassUnsafely` - release closure succeeds despite a required local closure failure.

## Contract signatures for State 4 design

- `fn validate_vb_njju_catalog(catalog: &[Scenario]) -> Result<(), EvidenceError>`
- `fn validate_admission_mutation_gate(evidence: MutationEvidence) -> Result<(), EvidenceError>`
- `fn validate_required_fuzz_smoke(evidence: FuzzSmokeEvidence) -> Result<(), EvidenceError>`
- `fn validate_generated_ir_taint_parity(evidence: PropertyEvidence) -> Result<(), EvidenceError>`
- `fn validate_unsafe_boundary_release_gate(evidence: BoundaryFuzzEvidence) -> Result<(), EvidenceError>`

## Verus-owned clauses

- Verus is not required for this bead at State 3 because the immediate artifact is a BDD/quality-gate closure over evidence manifests and test scenarios, not new Rust-local pure production logic.
- If State 4 introduces pure evidence classifiers beyond simple string/catalog checks, State 4 should either keep them trivial and covered by proptest/mutation or add a follow-up Verus obligation for classifier soundness.

## TLA+-owned clauses

- No TLA+ model is required for this bead. The bead defines static/release gate evidence closure, not a temporal workflow, scheduler, protocol, lease, retry, or concurrent lifecycle.
- Release pass/fail ordering is modeled as a finite evidence lattice in `tla-spec.md` with an explicit non-applicability rationale for TLC execution.

## Theorem-owned clauses

- No Lean/Aeneas/Hax theorem kernel is required. The relevant invariants are evidence classification and acceptance-test fail-closed behavior, not an algebraic theorem beyond Rust tests/property/mutation.

## Non-goals

- No production runtime behavior changes in State 3.
- No performance, assembly, API compatibility, or SBOM claims for this bead.
- No claim that fuzz targets provide exhaustive safety proof; this bead only requires release gate closure and smoke/seed evidence.
