# Round 5: Evidence-Pack + Remaining-Work — Transcript Index (12 agents)

**Run date:** 2026-06-07 · **Workspace:** /home/lewis/src/velvet-ballistics
**Subagent type:** general (12 agents in parallel)

The R5 agents wrote their plans to specific files. Index:

## Plan Files

| Agent | Plan Output | Total Hours | Beads |
|---|---|---|---|
| R5-A1 | `states/r4-p0-remediation-plan.md` (597 lines) | 40h | vb-r4fix-001..006 |
| R5-A2 | `to-fix/11-section50-arrayqueue-migration-plan.md` | 10.5h | vb-section50-1..4 |
| R5-A3 | `to-fix/11-section-65-taxonomy-migration.md` (MAJOR-6 filed as `vb-yfveq`) | 15.25h | vb-yfveq + 7 children |
| R5-A4 | `.bead-progress/source-length-r2-repair/plan.md` | 37.5h | vb-source-length-r2 |
| R5-A5 | `to-fix/13-section17-deadletter-recovery-plan.md` (1176 lines, 12 beads created) | 52h | vb-13d2a..k |
| R5-A6 | `to-fix/12-resource-contract-admission-gap.md` (30 KB) | 30h | vb-o5zb.3.1..6 |
| R5-A7 | `to-fix/13-dead-ir-deduplication-plan.md` (611 lines, 10 beads) | 4.5-7.5h | vb-br993..vb-eq7lv |
| R5-A8 | `docs/attempt-scope-fix-plan.md` (7 steps, 22.75h) | 22.75h | vb-scope-attempt.1..7 |
| R5-A9 | `/tmp/section38-property-test-gap-plan.md` (466 lines) | 41h | vb-cs38.1..11 |
| R5-A10 | `to-fix/11-round4-pipeline-integrity-plan.md` (453 lines) | 118h | vb-r4mi/rcov/rmut/rfuz/rbnc/rdet/rpp1/rpp2/rax1..13 |
| R5-A11 | (plan in agent output) | 7h | vb-ref-roots.1..6 |
| R5-A12 | `.evidence/SHIP-MASTER-ROADMAP-2026-06-07.md` (444 lines) | 124h total | **MASTER SYNTHESIS** |

## Per-Plan Highlights (condensed)

### R5-A1: P0 workstream (wait/recovery/Kani/Flux/perf/P0 closures) — 40h
- WI-1: Fix `await_timer` to read `deadline_slot` (transitions.rs:171) — 6h
- WI-2: Add `Runtime::recover()` from Fjall — 12h
- WI-3: Add `#[kani::unwind(N)]` to 3 timing-out Kani harnesses — 4h
- WI-4: Replace 12 `#[flux_rs::trusted]` with real refinements — 8h
- WI-5: Profile and reduce `compile_ir_1000_steps` to fit 200ms budget — 6h
- WI-6: Close 4 stale P0 beads (vb-1ev82, vb-8o7p5, vb-o5zb, vb-yesh4) — 4h

### R5-A2: Section 50 ArrayQueue migration — 10.5h
- Scanner extension: catch `crossbeam_channel::bounded(`, `sync_channel(`, `Mutex<VecDeque>` patterns
- BoundedActionCompletionQueue → ArrayQueue
- MemoryIngress → ArrayQueue (preserve public API)
- Backend-identity contract tests

### R5-A3: Section 65 SideEffect/RetrySafety migration — 15.25h
- 8 work items: rename enums, cascade 28 call sites, rewrite 3 gates, fix dead tests
- Migration in single commit (workspace never half-typed)

### R5-A4: source-length gate repair — 37.5h
- 7 files to split (cli_postcard/tests.rs 751 lines, cli_postcard/types.rs 530, output.rs 303, compiled_slug.rs 583, vb_ajc40_public_decode_regression.rs 600, dispatch_tests.rs 302, vb_a7t6_3_instruction_count_tests.rs 317)
- 2 stale rows to clean
- Quarterly ledger self-test
- Fix `infer_legacy_json_error_code` Holzman §3 violation in output.rs

### R5-A5: Section 17 dead-letter codes — 52h
- 12 beads: SECRET_UNAVAILABLE, REPLAY_DIVERGED, WAIT_TIMEOUT+ASK_TIMEOUT, STEP_SKIPPED_REFERENCE, FOR_EACH_ITEM_FAILED, TOGETHER_BRANCH_FAILED, COLLECT_PAGE_FAILED, REDUCE_ITEM_FAILED, RESULT_REFERENCE_MISSING, INPUT_MAPPING_FAILED, RETRY_EXHAUSTED
- 4 SHIP-BLOCKER minimum (21h): SECRET_UNAVAILABLE, REPLAY_DIVERGED, WAIT/ASK_TIMEOUT, STEP_SKIPPED_REFERENCE
- Delete self-laundering test bucket

### R5-A6: ResourceContract admission gap — 30h
- 6 work items: wire budget gate (12h), recalibrate policy (2h), lower hard limits (3h), re-open vb-o5zb.3 (2h), remove dead fields (4h), delete compiled_workflow.rs (2h)
- Re-open `vb-o5zb.3` or file follow-up bead

### R5-A7: Duplicate IR types cleanup — 4.5-7.5h
- 10 beads: delete nodes.rs, expressions.rs, accessors.rs, validation/ dir, compiled_workflow.rs, compiled_workflow.rs.removed, kani_resource_contract_validation_18_fields.rs
- Update master contract lines 572, 578, 3443
- Add `scripts/check-no-dead-ir-duplicates.sh` CI gate
- 4.5h base / 7.5h ceiling
- 2,097 dead LOC to remove

### R5-A8: $attempt.number restriction fix — 22.75h
- 7 leaf beads: AST foundation (add `body: Vec<StepAst>` to StepKindAst::Repeat), re-plumb 4 match sites, declare mod restrictions, add InvalidVariableScope error variant, file tracking bead, Kani harness
- Reopen `vb-xi2f.25` and `vb-xi2f.31`
- 7 leaf beads + 2 reopened parents

### R5-A9: Section 38 property tests + coverage — 41h
- 4 SHIP-BLOCKER proptests to create: concurrency_safety, bytecode_ast_parity, taint_propagation, error_recovery
- 5 alias strengthenings: digest_stability, for_each_ordering (proptest), resource_budget, bound_enforcement, layout_stability
- Real coverage report (replaces 3-byte stub)
- Test-density gate (5x enforcement)
- 11 beads

### R5-A10: moon ci integrity + bead hygiene — 118h
- 4 P0 bead closures/reclassifications
- 5 smoke-only lane expansions (miri, coverage, mutants, fuzz, bench)
- test-determinism re-inclusion with T0-T5 phased triage
- 2 phantom task removals
- 15 excluded task re-admissions or bead filings
- 27 total beads

### R5-A11: Section 8 reference root fix — 7h
- 4 easy-win work items: step_id, loop_name, error, total
- 1 placeholder for $attempt (depends on other bead)
- 1 $time diagnostic (FORBIDDEN_RUNTIME_REFERENCE 0x0205)
- 1 $inputs plural
- 6 beads

### R5-A12: MASTER SYNTHESIS (44-page roadmap) — 124h total
- **SHIP score: 41/100 → HOLD**
- Component breakdown: Build 70, Master 64, Runtime 25, CI 30, Coverage 5, Bead 30
- 9 MVS work items (130h) to bring SHIP to ≥ 80
- Full backlog (382h) for full Backend / IR Interpreter Complete DoD
- Estimated release-candidate landing: 2026-06-19 (MVS), 2026-06-26 (Full DoD)

## Master Synthesis (R5-A12)

The full synthesis is at `.evidence/SHIP-MASTER-ROADMAP-2026-06-07.md`. Key conclusions:

1. **SHIP SCORE: 41/100** — weighted across 6 dimensions
2. **RELEASE DECISION: HOLD** — do not declare "Backend / IR Interpreter Complete"
3. **9 MVS work items** (130h) bring SHIP to ≥ 80 by 2026-06-19
4. **Full DoD** requires 382h of work
5. **100+ new beads** needed (all filed in this round 5 dispatch)
6. **3-week wall-clock with femdation parallel dispatch**
