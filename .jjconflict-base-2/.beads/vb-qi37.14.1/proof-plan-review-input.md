# Proof Plan Review Input: vb-qi37.14.1

## Bead
`vb-qi37.14.1` — `run --step` single-step CLI command

## Scope
- **Package**: `vb_cli` (CLI layer) + `vb_core` (engine layer)
- **Entry point**: `cmd_run_step` in `app_impl.rs`
- **Core API**: `step_once(plan, run, store) -> Result<EngineSignal, EngineError>`

## Contract Summary
- **PRE-001**: durability must be `None`
- **PRE-002**: step_id must be in-bounds
- **PRE-003**: workflow must compile
- **PRE-004**: step-input must decode as `postcard::deserialize<Box<[SlotValue]>>`
- **POST-001**: `step_once` called exactly once
- **POST-002/003/004**: structured output (JSON/JSONL) with pc/slot/taint/state deltas
- **POST-005**: output slot value + taint in result
- **POST-006**: error reporting with diagnostic codes
- **POST-007**: validation-failed exit on durability != None
- **POST-008**: exit codes per outcome class
- **INV-001**: `RunFrame::new` bounds
- **INV-002**: `EngineSignal → StepState` mapping exhaustiveness
- **INV-003**: no panic on uninitialized slot read
- **INV-004**: PC in bounds after `step_once`
- **INV-005**: exactly one `step_once` per invocation
- **INV-006**: taint always {Clean, DerivedFromSecret, Secret}
- **ERR-001**: all `EngineError` variants returned correctly

## Risk Assessment
| Risk | Severity | Mitigation |
|---|---|---|
| `structured-output-gap` | Medium | Integration tests for JSON/JSONL format |
| `delta-reporting` | High | Integration tests + Kani state-snapshot harness |
| `durability-gates` | Low | Already correct; unit + integration coverage |
| `typed-errors` | Medium | Unit + Kani ERR-001 coverage |

## Verifier Lanes (29 obligations)
| Lane | Count | Obligations |
|---|---|---|
| Verus proof | 4 | INV-001, INV-002, INV-004, INV-006 |
| Kani bounded MC | 6 | PRE-002, INV-002, INV-003, INV-004, INV-006, ERR-001 |
| Unit test | 4 | PRE-001, POST-007, INV-005, ERR-001 |
| Integration test | 13 | PRE-002/003/004/005, POST-001/002/003/004/005/006/008 |
| Clippy | 1 | SRC-LINT |
| Waiver | 2 | TLA+, Lean |

## Waivers Requested
1. **TLA+**: Single-shot pure function — no temporal model needed. Rationale in
   `contract.md §TLA+-Owned` and `verification-layers.md §Waiver: TLA+`.
2. **Lean/Aeneas/Hax**: 9×9 boolean matrix exhaustively verified by Kani + unit
   tests. Rationale in `contract.md §Theorem-Owned`.

## Open Questions (UNRESOLVED — block test writing)
- **Q2**: Full `SlotValue` serialization or summary in JSON output?
- **Q3**: Diff-only deltas or full frame snapshot?

## Discovery Findings
- `step.rs`: `#![forbid(unsafe_code)]` — confirmed no UB risk
- `frame.rs`: `#![forbid(unsafe_code)]` — confirmed no UB risk
- Existing Verus artifacts: `run_frame_invariant.rs`, `signals_invariant.rs`,
  `step_state_machine.rs` — some overlap with INV-001/002/004/006
- Kani proofs for step-state matrix already exist in `frame.rs` (line 1308+)
- Workspace builds clean (`cargo build` succeeds)
- Kani v0.67.0 and Verus v0.2026.05.05 available

## Verification Artifact Inventory
| Obligation | Artifact | Status |
|---|---|---|
| VB-INV001-VERUS | `verification/verus/run_frame_invariant.rs` | EXISTS — may need extension |
| VB-INV002-VERUS | `verification/verus/step_state_machine.rs` | EXISTS — may need extension |
| VB-INV004-VERUS | `verification/verus/signals_invariant.rs` | EXISTS — may need extension |
| VB-INV006-VERUS | `verification/verus/run_frame_invariant.rs` | EXISTS — may need extension |
| VB-INV003-KANI | Harness `step_once_slot_init_harness` | MISSING — proof-writer must create |
| VB-PRE002-KANI | Harness `step_once_bounds_harness` | MISSING — proof-writer must create |
| VB-INV002-KANI | Harness `step_once_state_mapping_harness` | MISSING — proof-writer must create |
| VB-INV004-KANI | Harness `step_once_pc_bounds_harness` | MISSING — proof-writer must create |
| VB-INV006-KANI | Harness `taint_validity_harness` | MISSING — proof-writer must create |
| VB-ERR001-KANI | Harness `step_once_error_harness` | MISSING — proof-writer must create |
| CLI integration tests | `crates/vb_cli/tests/cli_integration.rs` | EXISTS — add run_step tests |

## Reviewer Action Required
1. **contract-verification-reviewer**: Approve/reject TLA+ and Lean waivers
2. **proof-reviewer**: Approve Verus spec-function assignments per invariant;
   flag any missing `requires`/`ensures` in existing Verus files
3. **black-hat-reviewer**: Verify INV-005 (exactly one `step_once`) is enforced
   structurally, not just by code inspection

## Status: PENDING REVIEW
All obligations marked `planned`. No proof artifact has been executed.
