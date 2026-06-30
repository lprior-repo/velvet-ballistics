# TLA Spec Note: vb-iucs

The relevant TLA+ model is `specs/tla/BudgetArithmetic.tla` with config `specs/tla/BudgetArithmetic.cfg`.

Evidence recovered from `.beads/vb-qi37.8/formal-verification-report.md`:

- Command: `tlc -config specs/tla/BudgetArithmetic.cfg specs/tla/BudgetArithmetic.tla`
- Result: PASS, no errors.
- State count: 166 states generated, 84 distinct states found, depth 2.
- Raw output: `.beads/vb-qi37.8/evidence/tlc-budget-arithmetic.out`.

The model represents Rust integers as exact 16-bit limbs and includes overflow/underflow error outcomes. It is accepted as scoped BudgetArithmetic evidence, not as proof of full validation pipeline composition.
