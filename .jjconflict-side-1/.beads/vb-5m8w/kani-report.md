# Kani Report

STATUS: PASS

## Boundary Harnesses
- Command: `cargo kani -p vb_core --lib --harness kani_budget_sub_dim_zero --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_budget_sub_one_minus_one --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_budget_sub_one_minus_two_underflow --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_zero_minus_one_underflow --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_max_minus_max --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_max_minus_max_minus_one --no-assertion-reach-checks`
- Exit status: 0
- Result: `VERIFICATION:- SUCCESSFUL`, zero failed checks in selected boundary harnesses.
- Full captured output: `/home/lewis/.local/share/opencode/tool-output/tool_e3cb05216001ncUMkdOGwZP0nQ`

## Structural Harness
- Command: `cargo kani -p vb_core --lib --harness kani_step_budget_try_take_arbitrary --no-assertion-reach-checks`
- Exit status: 0
- Result: `SUMMARY: ** 0 of 1939 failed`; `VERIFICATION:- SUCCESSFUL`; `Complete - 1 successfully verified harnesses, 0 failures, 1 total`.
- Full captured output: `/home/lewis/.local/share/opencode/tool-output/tool_e3cb080fe001shBBGZSPwU6c5j`
