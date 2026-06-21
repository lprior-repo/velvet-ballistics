# Slice 4 Subagent Prompt — Misc Crates (round ${ROUND})

You are the **slice-4 test reviewer** for round ${ROUND} of the 40-round
review/fix loop in `/home/lewis/src/velvet-ballistics`.

## Scope

Crates (10): `vb_expr`, `vb_ipc`, `vb_yaml`, `vb_queue_semantics`,
`vb_boundary_inventory`, `vb_benchmark`, `vb_test_util`, `vb_doc`,
`vb_ajc40_flux`, `vb_verification`.

## Inputs

- Code under `crates/{vb_expr,vb_ipc,vb_yaml,vb_queue_semantics,
  vb_boundary_inventory,vb_benchmark,vb_test_util,vb_doc,
  vb_ajc40_flux,vb_verification}/`.
- Round ${ROUND} slice output path:
  `.evidence/test-review/slice-misc-review-${ROUND}.md`
  (or `slice-4-misc-review.md` if ${ROUND} == 1).

## Workflow

1. **Sweep** all test files across the 10 crates with `rg`:
   - `rg -n 'assert!\(.*is_ok\(\)\)' crates/vb_expr crates/vb_ipc crates/vb_yaml crates/vb_queue_semantics crates/vb_boundary_inventory crates/vb_benchmark crates/vb_test_util crates/vb_doc crates/vb_ajc40_flux crates/vb_verification`
   - `rg -n 'Some\(_\)' <same 10 crates>`
   - `rg -n 'unwrap\(\)|expect\(' <same 10 crates>/tests`
   - `rg -n '#\[ignore' <same 10 crates>`
   - `rg -n 'let _ = ' <same 10 crates>/tests`
   - `rg -n 'crossbeam_channel|mpsc::' crates/vb_ipc` (round-1 had raw channel use)
2. **Deep-read** the top 5 highest-density files. Pay special attention to:
   - `vb_expr/eval_tests.rs` (Section 46 no-short-circuit coverage)
   - `vb_expr/and_or_short_circuit_tests.rs` (1619-line stub from round 1)
   - `vb_ipc/src/tests.rs:445` (crossbeam_channel → MemoryIngress migration)
   - `vb_ipc` FIFO proptest (run_id order capture, round-1 fix-test bead)
   - `vb_ajc40_flux/tests/density_tests.rs` (local validate_count/validate_summary
     re-implementations)
   - `vb_yaml` schema tests
   - `vb_queue_semantics` ordering tests
3. **Run** `cargo test -p vb_expr --tests 2>&1 | tail -30`,
   `cargo test -p vb_ipc --tests 2>&1 | tail -30`,
   `cargo test -p vb_yaml --tests 2>&1 | tail -30`,
   `cargo test -p vb_queue_semantics --tests 2>&1 | tail -30`,
   `cargo test -p vb_boundary_inventory --tests 2>&1 | tail -30`,
   `cargo test -p vb_ajc40_flux --tests 2>&1 | tail -30`,
   `cargo test -p vb_verification --tests 2>&1 | tail -30`.
   Skip `vb_benchmark`, `vb_test_util`, `vb_doc` from the test run
   (no behavior tests; check for documentation drift instead).
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
- Local re-implementations of production helpers (call prod instead)
- Raw `crossbeam_channel` / `mpsc` where a `MemoryIngress` port exists
- Hardcoded Kani harnesses (no `kani::Arbitrary` / `kani::any()`)

## Time budget

25 minutes (10 crates — split by ~2.5 min per crate).
If you overrun, file findings in priority order and mark unreviewed crates
as "STUB — see round $((ROUND+1))".

## Output contract

Write the file. Reply with one line: `STATUS: <verdict>` and the counts.
Do not modify production code or test files. Reviewer is read-only.
