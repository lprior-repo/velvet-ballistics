# Minimal F64 regression recovery contract R2

R2 supersedes initial test-file recovery proposal after discovering existing workspace signed-zero integration coverage. Preserve R1 candidate c1d847f6 and all evidence/old contract.

Preserve current production and all existing assertions. Reuse current workspace signed-zero helpers to add only missing literal -0.0/--0.0/-(-0.0) bit checks and successful double integer-negation boundary cases through both production compilers/evaluators. Strengthen exact nested-negation constant-pool order and stack 3 plus helper nested-negation constant order. Existing reference float/overflow/type/binary subtraction controls remain and must execute.

Only three existing test files may change. No cloud test-file/registration duplication, dependencies, gates, GPU, Python, benchmarks or main push. Pinned affected two-crate library suites and existing signed-zero integration target must pass; format/source policy and exact hash-bound patch for independent review. Author cannot approve own tests.

Retain cloud +/-1 witnesses by extending the existing finite float fixture to ten points, add missing direct AST sign-bit controls and integer-boundary checks in existing loops; do not duplicate the existing reference/runtime/error matrix.
