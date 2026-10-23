# Source-reachable bounded composite execution

Parent F1 remains open. Input candidate is exact allocator v2 + composite v3 + frozen19case source fixture v5. Runtime/validator repairs must preserve the compiler-generated IR as the executed artifact, all source slots and taints, bounded body execution, state isolation, and terminal continuation.

Repeat must progress through at most its declared attempts and retain a separate result; choose semantically correct canonical RepeatCheck/Attempt wiring consistent with production primitive contracts. Only an owning-loop-validated backedge may be admitted, never generic backward next edges. Sequential completed loop spans must be removed before comparing a new loop's nesting. Crossing loops and invalid state/targets remain errors.

Together results and continuation must use consistent initialized runtime state and bounded branch completion. Collect executes the first nonempty page body even when source fits one page, then each subsequent bounded page exactly once. Empty sources execute no body. No validation/lint waiver, fabricated IR, integer/taint laundering or test weakening. Negative malformed/crossing-loop and boundary tests plus existing tests are required.

Local5090 authors production repairs,3090 performs independent source/test review; root coordinates serialized execution and integration. No main acceptance before native source execution and gates. This child does not close arbitrary nested source language, general lexical names or metadata controls in parent F1.
