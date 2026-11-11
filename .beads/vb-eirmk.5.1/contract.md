# Numeric execution qualification contract

Bead: vb-eirmk.5.1. Source parent: nrqyptrx3c7fbe4e. Scope: live vb_core expression operators; public validated EvalExpr-to-Finish execution. No public API, persisted format, dependency, or gate change.

All Add/Sub/Mul/Div and Gt/Gte/Lt/Lte cases exercise the deterministic drive with I64/I64, I64/F64, F64/I64 and F64/F64 operands. Compare greater, lesser and equal inputs with exact true/false results, including strict versus inclusive distinction. Successful execution must finish at the Finish node with both nodes Succeeded.

Pure integer add/subtract/multiply overflow returns InvalidCompiledWorkflow with reason integer arithmetic overflow. MIN divided by -1 preserves integer division overflow. Integer division by zero returns DivisionByZero. Floating or mixed zero division and nonfinite arithmetic return NonFiniteNumber. A boolean on either numeric operand returns exact TypeMismatch expected number, found boolean.

I64-to-F64 promotion uses IEEE nearest-even results: 9007199254740993 becomes 9007199254740992, 9007199254740995 becomes 9007199254740996, MAX becomes 9223372036854775808 and MIN becomes -9223372036854775808. Promotion must affect comparisons consistently; promoted 2^53+1 is not greater than floating 2^53 and is greater-or-equal. Results are exact finite values, not merely finite or approximately equal.

Each numeric operator and type pairing joins Secret from either operand and returns the exact expected value with Secret taint. Clean operands remain Clean. Preserve variant-sensitive equality behavior and all prior tests.

Behavioral evidence: pinned nightly-2026-04-28 nextest for vb_core with all features and warnings denied, strict source clippy, native formatting, then canonical Moon gates. Four independent symbolic properties remain meaningful and error-propagating. Sol CODE and independent TEST review must pass the final source. Focused green tests do not establish whole backend or full CI acceptance. No speed claim.
