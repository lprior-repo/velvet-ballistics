# vb-qi37.6 Proof Strategy

## Boundary

- Workspace: `/home/lewis/src/vb-qi37-6` only.
- Planner-owned outputs: `.beads/vb-qi37.6/proof-strategy.md`, `.beads/vb-qi37.6/proof-plan-review-input.md`, and `.beads/vb-qi37.6/proof-obligations.planned.jsonl`.
- Source checkout `/home/lewis/src/Velvet-ballistics` is forbidden and was not used.
- This State 4 rerun only plans proof obligations. It does not edit production code, tests, proof models, harnesses, fuzz manifests, or CI config.

## Inputs Read

- `.beads/vb-qi37.6/contract.md`
- `.beads/vb-qi37.6/delivery-scope.jsonl`
- `.beads/vb-qi37.6/traceability-matrix.jsonl`
- `.beads/vb-qi37.6/proof-obligations.jsonl`
- `.beads/vb-qi37.6/proof-obligations.planned.jsonl`

## Discovery Evidence

- Required bead artifacts exist: `contract.md`, `traceability-matrix.jsonl`, `delivery-scope.jsonl`, `proof-obligations.jsonl`, and `proof-obligations.planned.jsonl`.
- Scoped risk scan covered the files named by `delivery-scope.jsonl`, plus `verification/tla` and `verification/verus`.
- Risk triggers found: auth/security capability checks, serialization/deserialization of accepted artifacts and UI views, temporal admission/run state transitions, bounded shard queues, cancellation paths, Kani harnesses, TLA+ capability lifecycle specs, and Verus capability model proofs.
- Focused proof artifacts exist: `verification/tla/CapabilityLifecycle.tla`, its five focused configs plus all-config, and `verification/verus/capability_artifact_model.rs`.
- JSONL validation before repair showed 24 traceability rows, 24 contract obligation rows, 24 planned rows, no duplicate primary keys, and no `PASS` statuses.

## Strategy

- TLA+ owns the temporal and fail-closed lifecycle surface: accepted-artifact envelope lookup, gate-count mismatch, exact profile admission, denial atomicity, no-contract Do denial, public empty-grant behavior, and legacy bypass prevention.
- Verus owns pure capability algebra and preservation models: exact name/action matching, prefix denial, cardinality, non-empty contract preservation, and Gate 12 schema assumptions.
- Kani remains required for bounded Rust implementation harnesses, but it is routed through State 8 setup and State 11 execution. State 4 does not claim Kani evidence.
- Fuzz remains required for capability name/contract schema inputs, but target registration is State 8 setup and execution is State 11. State 4 does not claim fuzz evidence.
- Unit, integration, BDD, UI serde, static scan, Miri, clippy, and Moon-equivalent lanes remain later execution evidence and do not replace formal rows.

## State Routing

- State 4: owns this proof plan, exact row coverage, traceability, executable commands, and no-pass/no-placeholder ledger hygiene.
- State 8: owns Kani module wiring and fuzz bin registration setup where rows explicitly say setup is required.
- State 10: owns unit, integration, BDD, UI parity, and static scan execution rows.
- State 11: owns TLA+/Verus/Kani/fuzz/formal gauntlet execution evidence after setup is complete.

## Planned Obligation Set

- `PRE-001-TLA-ENVELOPE`: TLA+ persisted accepted-artifact envelope required before non-Relaxed admission.
- `PRE-002-TLA-GATE15`: TLA+ Strict/Journaled gate count must equal 15.
- `PRE-003-FUZZ-SCHEMA`: State 8 fuzz target registration check, then State 11 capability schema fuzz execution.
- `PRE-004-API-GRANTS`: State 10 public Runtime grant path tests for non-empty requirements.
- `PRE-005-TLA-CONTRACT-SLICE`: TLA+ contract slice presence/absence drives Do execution or denial.
- `PRE-006-UI-SOURCE`: State 10 UI source parity and serde tests.
- `POST-001-VERUS-EXACT`: Verus exact-only grant predicate model.
- `POST-002-TLA-GATE-DENIAL`: TLA+ invalid gate count denies without allocation.
- `POST-003-TLA-CARDINALITY-DENIAL`: TLA+ grant count mismatch denies without allocation.
- `POST-004-TLA-MISSING-EXACT`: TLA+ missing/non-exact grant denies without allocation.
- `POST-005-TLA-SUCCESS-JOURNAL`: TLA+ RunAdmission is journaled only after successful admission.
- `POST-006-TLA-DO-CHECKS`: TLA+ contracted Do checks capabilities before AwaitingAction.
- `POST-007-TLA-NO-CONTRACT-DENY`: TLA+ no-contract Do fails closed without AwaitingAction.
- `POST-008-TLA-LEGACY-BYPASS`: TLA+ and static evidence prevent legacy protected-submit bypass.
- `POST-009-UI-PARITY`: State 10 UI projection parity evidence.
- `INV-001-KANI-EXACT-SETUP`: State 8 Kani module setup check, then State 11 exact grant Kani harness.
- `INV-002-KANI-CARDINALITY-SETUP`: State 8 Kani dependency setup check, then State 11 runtime cardinality Kani harness.
- `INV-003-TLA-GATE-CONTRACT`: TLA+ single gate-count contract value fails closed on mismatch.
- `INV-004-VERUS-PERSISTENCE`: Verus required-capability preservation model.
- `INV-005-TLA-DENIAL-ATOMIC`: TLA+ denial causes allocate no run and journal no RunAdmission.
- `INV-006-TLA-SHARD-CONTRACTS`: TLA+ shard drive cannot bypass missing contracts.
- `INV-007-STATIC-LEGACY`: State 10 static scan and integration route legacy APIs out of protected admission evidence.
- `INV-008-TLA-PUBLIC-GRANTS`: TLA+ public submit grant profile rejects empty grants for non-empty requirements.
- `GAUNTLET-010`: State 11 proof/deep/static/Miri/fuzz/Moon-equivalent release gauntlet consumes all prior evidence or explicit waivers.

## Ledger Constraints

- `proof-obligations.planned.jsonl` preserves all 24 primary IDs exactly once.
- Every planned row maps to a `requirement_id` and `contract_clause`, including the release-gate row.
- TLA+ rows include executable `tlc` commands, module/config/model metadata, finite state constraints, invariants, and refinement notes.
- Verus rows include executable `verus` commands, proof/spec metadata, trusted boundary notes, and shell exclusions.
- Kani/fuzz rows include executable setup-check commands and separate State 11 execution commands in `after_setup_commands`.
- No row records `PASS`; all rows remain future evidence obligations.
