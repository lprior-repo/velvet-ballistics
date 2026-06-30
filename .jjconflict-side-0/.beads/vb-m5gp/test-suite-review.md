# Test Suite Review: vb-m5gp — State 9 Retry Attempt 5

STATUS: APPROVED

## Doctrine Cited

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`: lines 113-187 require suite static scans for banned assertions, swallowed errors, ignored tests, sleeps, global mutable state, mocks, private integration imports, error variant completeness, density/API evidence, and insta detection.
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`: same content; per startup rule this copy wins on conflict.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`: lines 13-49 require traceable exact evidence and bounded generated cases; lines 178-191 require failure locality.

## Inputs Reviewed

- `.beads/vb-m5gp/test-plan.md`
- `.beads/vb-m5gp/contract.md`
- `.beads/vb-m5gp/implementation.md`
- `.beads/vb-m5gp/source-length-report.md`
- `.beads/vb-m5gp/static-scan-report.md`
- `crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs`
- `scripts/check-source-length.sh`
- `crates/vb_compile/src/mod_compile_errors/{kind.rs,collection.rs,source_mark.rs}`
- `crates/vb_compile/src/mod_compile_validation/part_*.rs`

## Tier 0 — Static / Source Review

- PASS: focused banned-pattern scan over `crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs` returned `0 matches` for banned result-only assertions, swallowed errors, ignored tests, sleeps, shared mutable state, mocks, `.expect_`, and private `use crate::` integration imports.
- PASS: integration API purity. The split contract test imports public `vb_compile::...` crate-root API only.
- PASS: public internal module leak scan returned `0 matches` for public `compile`, `lower`, `validation`, or `mod_compile_*` module paths.
- PASS: fake split regression checks remain present: the suite rejects doc-only split modules, `include!` bodies, and a returned `compile_core_impl.rs`.
- PASS: recursive source-length coverage now descends into bead-local `mod_compile_*` directories. Independent count found no bead-local split source at or above 300 physical lines.
- PASS: dependency-edge coverage now rejects forbidden `mod_compile_errors -> mod_compile_validation` and `mod_compile_validation -> mod_compile_core` imports. Independent scan found `dependency_edge_violations: 0`.
- PASS: bead-local split source counts are all below 300 lines; highest observed file was `crates/vb_compile/src/mod_compile_errors/collection.rs: 286`.

## Tier 1 — Execution Evidence

- PASS: `cargo +nightly fmt --all --check` exited 0.
- PASS: `cargo +nightly check -p vb_compile --all-targets --all-features` exited 0.
- PASS: strict source clippy command exited 0: `cargo +nightly clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings ...`.
- PASS: focused edge test passed: `cargo +nightly test -p velvet-ballistics-workspace-tests --test vb_m5gp_compile_split_contract mod_compile_dependency_edges_remain_acyclic_and_diagnostic_leaf` — 1 passed.
- PASS: focused source-length test passed: `cargo +nightly test -p velvet-ballistics-workspace-tests --test vb_m5gp_compile_split_contract vb_compile_production_sources_remain_under_agreed_line_limit` — 1 passed.
- PASS: `cargo +nightly test -p velvet-ballistics-workspace-tests --test vb_m5gp_compile_split_contract` passed: 8 passed, 0 failed, 0 ignored.
- PASS: `bash scripts/check-source-length.sh` exited 0, with only `DEFERRED_GLOBAL` notices for pre-existing unrelated top-level files: `expression_bytecode.rs`, `expression.rs`, `references.rs`, `schema.rs`, and `type_taint.rs`.

## Tier 2 — Coverage

- PASS by scoped structural evidence. This retry's blocker was module dependency direction after source-length repair, not new runtime behavior. The targeted tests enumerate every bead-local split source under `mod_compile_*` directories and reject forbidden dependency edges.

## Tier 3 — Mutation Thought Experiment

- PASS: change recursive directory traversal to top-level-only → `vb_compile_production_sources_remain_under_agreed_line_limit` would miss a seeded oversized nested file; the shell gate mirrors the traversal.
- PASS: increase `kind.rs` above 300 lines → Rust split test and `scripts/check-source-length.sh` fail.
- PASS: reintroduce `crate::mod_compile_validation` in `mod_compile_errors` or `crate::mod_compile_core` in `mod_compile_validation` → `mod_compile_dependency_edges_remain_acyclic_and_diagnostic_leaf` fails.
- PASS: reintroduce `compile_core_impl.rs` or `include!` → split ownership test and shell gate fail.
- PASS: expose `mod_compile_*` publicly → privacy scan/test fails.

## LETHAL FINDINGS

- None.

## MAJOR FINDINGS (0)

- None.

## MINOR FINDINGS (0/5 threshold)

- None.

## Verdict

Approved. The dependency-edge repair is now test-enforced, recursive source-length enforcement still passes, and the scoped split suite passes with exact evidence.
