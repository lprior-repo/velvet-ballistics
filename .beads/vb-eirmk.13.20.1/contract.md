# Existing Verus assertion literal typing

Preserve the exact repeat-owner and expression-stack propositions and all production behavior. The source100e43c verifier rejects RHS literal inference in two repeat-owner proof assertions and one expression-stack proof assertion. spec_repeat_owner returns Option<int>; spec_expr_effect returns (int,int). Annotate only their RHS literals with int, keeping values0,2,1 and every assertion.

All executable bodies, function signatures, specification definitions, requires/ensures/recommends, invariants/decreases, imports and trust markers remain byte-identical. No new assumptions, exclusions or solver limits. Proof-assertion regions are mutable under the installed Verus skill; no immutable-region delta is authorized by this repair.

Verify the original failing command verus --crate-type=lib verification/verus/try_from_parts.rs, then the registry-driven Moon Verus task. Retain raw command, source, exit, zero-error summaries and existing warnings; independent CODE and TEST reviews are required. A target pass is scoped evidence, not whole repository proof or CI qualification.
