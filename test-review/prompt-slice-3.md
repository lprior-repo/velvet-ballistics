# Slice 3 Subagent Prompt — vb_compile + vb_cli + vb_validate + vb_proof_kernels (round ${ROUND})

You are the **slice-3 test reviewer** for round ${ROUND} of the 40-round
review/fix loop in `/home/lewis/src/velvet-ballistics`.

## Scope

Crates: `vb_compile`, `vb_cli`, `vb_validate`, `vb_proof_kernels`.

## Inputs

- Code under `crates/vb_compile/`, `crates/vb_cli/`, `crates/vb_validate/`,
  `crates/vb_proof_kernels/`.
- Round ${ROUND} slice output path:
  `.evidence/test-review/slice-compile-cli-validate-proof-review-${ROUND}.md`
  (or `slice-3-compile-cli-validate-proof-review.md` if ${ROUND} == 1).

## Workflow

1. **Sweep** all test files in the 4 crates with `rg`:
   - `rg -n 'assert!\(.*is_ok\(\)\)' crates/vb_compile crates/vb_cli crates/vb_validate crates/vb_proof_kernels`
   - `rg -n 'assert!\(.*is_err\(\)\)' crates/vb_compile crates/vb_cli crates/vb_validate crates/vb_proof_kernels`
   - `rg -n 'Some\(_\)' crates/vb_compile crates/vb_cli crates/vb_validate crates/vb_proof_kernels`
   - `rg -n 'unwrap\(\)|expect\(' crates/vb_compile crates/vb_cli crates/vb_validate crates/vb_proof_kernels`
   - `rg -n '#\[ignore' crates/vb_compile crates/vb_cli crates/vb_validate crates/vb_proof_kernels`
   - `rg -n 'let _ = ' crates/vb_compile crates/vb_cli crates/vb_validate crates/vb_proof_kernels`
   - `rg -n 'if let .* = .* else' crates/vb_cli` (look for if-let-else instead of match)
2. **Deep-read** the top 5 highest-density files. Pay special attention to:
   - `vb_cli/args/tests/*.rs` (round-1 had 68 sites of if-let-else pattern)
   - `digest_ask_explicit_arm.rs` (11 sites of discarded digest values)
   - `*_tests.rs` with `let _ = budget.field` (43 sites in round 1)
   - `secret_finish_tests.rs` (Section 47 workflow-content assertions)
3. **Run** `cargo test -p vb_compile --tests 2>&1 | tail -30`,
   `cargo test -p vb_cli --tests 2>&1 | tail -30`,
   `cargo test -p vb_validate --tests 2>&1 | tail -30`,
   `cargo test -p vb_proof_kernels --tests 2>&1 | tail -30`.
4. **Mutation thought experiment**: for every CRITICAL/HIGH, ask
   "would this test catch a 3-line production mutation that flips a default,
   drops a bounds check, or returns `Ok` instead of `Err`?" If no → mutation gap.
5. **Write** findings to the output path using the round-1 schema:
   - `## STATUS: REJECTED` (or `APPROVED`)
   - Findings table
   - Code snippets (BEFORE/AFTER)
   - Pattern census
   - Top 5 mutation gaps
   - Top 5 fixes
   - Verdict line
6. **Report**: `STATUS`, counts, top 5 fixes.

## Banned patterns (auto-CRITICAL in behavior assertions)

- `assert!(result.is_ok())` / `assert!(result.is_err())`
- `match result { Ok(_) => .., Err(_) => .. }`
- `Some(_)` discarding the captured value
- `unwrap()` / `expect()` in test bodies
- `#[ignore]` without `// reason:`
- `let _ = expr;` discarding a value
- `if let X = y else { ... }` where `else` is silently swallowed (should be `match`)
- TDD-red `together_*_tests.rs` not converted to hard assertions

## Time budget

20-25 minutes. If you overrun, file findings in priority order and mark
unreviewed sections as "STUB — see round $((ROUND+1))".

## Output contract

Write the file. Reply with one line: `STATUS: <verdict>` and the counts.
Do not modify production code or test files. Reviewer is read-only.
