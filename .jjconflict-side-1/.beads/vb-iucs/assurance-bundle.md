# Assurance Bundle: vb-iucs

## Claims

| Claim | Evidence | Verdict |
|-------|----------|---------|
| Target recovered | `bd show vb-iucs` plus `.beads/vb-qi37.8` search hits | PASS |
| Gate 8 proof integration exists | `crates/vb_validate/src/kani_gate_08_accessor.rs` | PASS |
| Gate 8 verifier evidence exists | `.beads/vb-qi37.8/formal-verification-report.md` | PASS |
| StepState runtime binds to proof kernel | `crates/vb_core/src/frame.rs` | PASS |
| StepState Kani parity exists | `crates/vb_core/src/kani_step_state_transition.rs` and raw report | PASS |
| StepState Verus mirror exists | `verification/verus/step_state_machine.rs` and raw report | PASS |
| BudgetArithmetic TLA evidence exists | `specs/tla/BudgetArithmetic.tla` and raw TLC report | PASS |
| Deferred non-claims preserved | `final-evidence-decision.md` and this bundle | PASS |

## Raw Evidence Paths

- `.beads/vb-qi37.8/proof-evidence.md`
- `.beads/vb-qi37.8/formal-verification-report.md`
- `.beads/vb-qi37.8/black-hat-review.md`
- `.beads/vb-qi37.8/truth-serum-report.md`
- `.beads/vb-qi37.8/final-evidence-decision.md`
- `/home/lewis/.local/share/opencode/tool-output/tool_e3551a45e001e3Jgip0j7IWB1F`
