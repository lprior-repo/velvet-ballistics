bead_id: vb-tw3b
phase: 3

# Domain model review

Existing domain boundaries are adequate for closure: `ExprProgram`, `ExprOp`, `SlotValue`, and `Taint` carry the state needed to compare bytecode/interpreter and generated Rust expression behavior. No illegal-state refactor required for this dependency-close bead.

STATUS: APPROVED
