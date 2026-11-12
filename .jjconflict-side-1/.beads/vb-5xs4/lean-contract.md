# Lean Contract Projection: vb-5xs4

## Boundary
- Lean-owned kernel: pure classification and disposition algebra over already-discovered findings.
- Rust/runtime shell: filesystem discovery, Rust source parsing, macro/source mapping, bead/report writing, CLI output, process execution, and Moon/CI orchestration.
- External systems excluded from Lean proof: filesystem, bd/beads database, Dolt, git, Moon, mutation tools, terminal output, wall-clock time, and OS errors.

## Abstract Model
- `PatternKind`: loop/table/helper/macro-derived test repetition categories.
- `CaseLabel`: absent, ambiguous, or sufficient label with behavior and case identity.
- `Risk`: non-risky or risky with a reason.
- `Evidence`: repair reference, accepted exception metadata, or labeling proof.
- `Disposition`: `RepairRequired`, `AcceptedException`, or `SafeLabelingProven`.
- `Finding`: stable location, pattern kind, risk, evidence, and disposition.
- `Inventory`: ordered list of findings.

## Lean-Owned Clauses
- INV-001 -> `VelvetBallastics.TestLoopInventory::risky_findings_are_assigned`
- INV-002 -> `VelvetBallastics.TestLoopInventory::risky_finding_has_exactly_one_disposition`
- INV-003 -> `VelvetBallastics.TestLoopInventory::retained_loop_has_sufficient_case_identity`
- POST-005 -> `VelvetBallastics.TestLoopInventory::safe_labeling_proof_requires_behavior_and_case`
- ERR-006 -> `VelvetBallastics.TestLoopInventory::ambiguous_label_is_not_safe`
- INV-004 -> `VelvetBallastics.TestLoopInventory::evidence_refinement_is_monotonic`
- POST-006 -> `VelvetBallastics.TestLoopInventory::classification_is_deterministic`
- POST-007 -> `VelvetBallastics.TestLoopInventory::quality_gate_fails_closed`

## Theorem Obligations

### THM-INV-001
- Contract clauses: INV-001, POST-007
- Rust/spec target: `crate::quality::test_loop_inventory::validate_inventory`
- Lean module: `VelvetBallastics.TestLoopInventory`
- Theorem shape: `risky_findings_are_assigned`
- Model: finite list of abstract findings with risk and disposition states.
- Refinement: Rust `ValidatedInventory` may be constructed only from an abstract inventory satisfying `all_risky_assigned`.
- Shell exclusions: filesystem discovery, parser failures, bead writes, CLI rendering.
- Evidence command: `moon run :verify-proof`

### THM-INV-002
- Contract clauses: INV-002, ERR-008
- Rust/spec target: `crate::quality::test_loop_inventory::assign_disposition`
- Lean module: `VelvetBallastics.TestLoopInventory`
- Theorem shape: `risky_finding_has_exactly_one_disposition`
- Model: sum type with three legal dispositions and invalid none/multiple states.
- Refinement: Rust disposition construction maps to exactly one Lean disposition constructor.
- Shell exclusions: user input, report formatting, bead command behavior.
- Evidence command: `moon run :verify-proof`

### THM-INV-003
- Contract clauses: INV-003, POST-003
- Rust/spec target: `crate::quality::test_loop_inventory::classify_loop_pattern`
- Lean module: `VelvetBallastics.TestLoopInventory`
- Theorem shape: `retained_loop_has_sufficient_case_identity`
- Model: label sufficiency predicate requiring behavior identity and case identity.
- Refinement: Rust `SafeLabelingProven` can reify only from labels satisfying the Lean predicate.
- Shell exclusions: exact compiler diagnostic text, macro expansion, terminal display width.
- Evidence command: `moon run :verify-proof`

### THM-POST-005
- Contract clauses: POST-005
- Rust/spec target: `crate::quality::test_loop_inventory::assign_disposition`
- Lean module: `VelvetBallastics.TestLoopInventory`
- Theorem shape: `safe_labeling_proof_requires_behavior_and_case`
- Model: safe-labeling proof record with mandatory behavior identity and case identity evidence fields.
- Refinement: Rust `SafeLabelingProven` disposition reifies to Lean only when both evidence fields satisfy the sufficiency predicate.
- Shell exclusions: report formatting, terminal diagnostic rendering, filesystem discovery, bead writes.
- Evidence command: `moon run :verify-proof`

### THM-ERR-006
- Contract clauses: ERR-006
- Rust/spec target: `crate::quality::test_loop_inventory::classify_loop_pattern`
- Lean module: `VelvetBallastics.TestLoopInventory`
- Theorem shape: `ambiguous_label_is_not_safe`
- Model: label classification predicate distinguishing absent, ambiguous, and sufficient labels.
- Refinement: Rust `AmbiguousCaseLabel` or `RepairRequired` corresponds to Lean ambiguous-label state; Rust `SafeLabelingProven` is unreachable for that state.
- Shell exclusions: parser recovery, macro expansion, exact compiler or terminal diagnostic wording.
- Evidence command: `moon run :verify-proof`

### THM-INV-004
- Contract clauses: INV-004
- Rust/spec target: `crate::quality::test_loop_inventory::classify_loop_pattern`
- Lean module: `VelvetBallastics.TestLoopInventory`
- Theorem shape: `evidence_refinement_is_monotonic`
- Model: evidence partial order from absent to ambiguous to sufficient/assigned evidence.
- Refinement: Rust classifier risk downgrades only when evidence relation increases to sufficient evidence.
- Shell exclusions: source scanning and parser error recovery.
- Evidence command: `moon run :verify-proof`

### THM-POST-006
- Contract clauses: POST-006
- Rust/spec target: `crate::quality::test_loop_inventory::classify_loop_pattern`
- Lean module: `VelvetBallastics.TestLoopInventory`
- Theorem shape: `classification_is_deterministic`
- Model: pure total function from ordered abstract pattern plus policy to risk/disposition requirement.
- Refinement: Rust classification result equals Lean model for normalized input and policy.
- Shell exclusions: file traversal order before normalization, I/O errors, nondeterministic external commands.
- Evidence command: `moon run :verify-proof`

## Waivers
- No waiver for pure critical classification/disposition behavior.
- WAIVER-001:
  - Clauses: PRE-001, PRE-003, PRE-004, ERR-001, ERR-003, ERR-004, ERR-005, ERR-010.
  - Waived layer: Lean.
  - Reason: filesystem traversal, file decoding, parser recovery, macro/source mapping, and OS errors are runtime-shell behavior, not pure deterministic kernel behavior.
  - Compensating evidence: manual-qa, fuzz/bolero, Miri/cargo-careful, coverage, and static-scan obligations in `verification-layers.md` and `proof-obligations.jsonl`.
  - Owner: downstream vb-5xs4 implementation owner.
  - Expiration/follow-up: expires if a pure parser/normalizer model is introduced.
- WAIVER-002:
  - Clauses: POST-001, POST-004, INV-006.
  - Waived layer: Lean.
  - Reason: report rendering and human-readable exception/report metadata are shell presentation concerns.
  - Compensating evidence: Fowler scenarios, cargo-llvm-cov branch evidence, mutation checks, and manual QA against real inventory output.
  - Owner: downstream vb-5xs4 implementation owner.
  - Expiration/follow-up: expires if report validation becomes a pure typed renderer with semantic formatting claims.
