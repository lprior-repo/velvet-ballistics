# Master Roadmap & SHIP Decision

**Date:** 2026-06-07
**Repo:** `/home/lewis/src/velvet-ballistics`
**Checkout:** 106b5b621 (clean main)

---

## SHIP SCORE: 41 / 100 → **HOLD**

| Component | Weight | Score | Evidence |
|---|---:|---:|---|
| Build + Test + Lint | 0.20 | 70 | `cargo check` green; 14,305 tests pass; 7 source-length violations RED |
| Master contract | 0.20 | 64 | Avg of §14 99, §15 68, §18 94, §50 35, §65 18, §8 71, §13 64, §17 65, §36-39 62 = 64 |
| Verified runtime correctness | 0.25 | 25 | Wait primitive ignores deadline_slot; no recovery of pending_timers; 3 Kani harnesses timeout; Flux annotations vacuous |
| CI integrity | 0.15 | 30 | 5 of 21 pipeline tasks are smoke-only (Miri 0.14%, Mutants 0.026%, Fuzz 5s, Coverage 1 test); 15 `runInCI:false` tasks; test-determinism hides 1,088 findings |
| Coverage evidence | 0.10 | 5 | `tarpaulin-report.json` is 3 bytes; `coverage.log` is a 1-line stub; no real lcov summary |
| Bead hygiene | 0.10 | 30 | 4 P0 beads stale; 1,088 determinism findings hidden; 1 P0 OPEN (vb-yfveq) |

**Weighted Total: 41 / 100**

---

## Release Decision: **HOLD**

Do not declare "Backend / IR Interpreter Complete." Do not merge past `moon ci` RED. Do not waive the canonical gate.

The product identity (`MASTER.md:31`) is "an AI-safe, local-first, single-server durable execution engine that verifies AI-authored workflows before admission, persists an inspectable journal, protects side effects with idempotency evidence." The current state is "a runtime that loses suspended runs on restart and ignores the deadline it claims to honor, with 11 of 30 runtime error codes silently missing, with the IPC ingress violating the ArrayQueue mandate, with the SideEffect taxonomy broken, with smoke-only verification lanes masquerading as full gates, and with 7 source files over the 300-line limit." The two are not the same product.

---

## Minimum Viable Ship Subset (MVS-1..9)

9 work items that, if completed, would bring the SHIP score from 41 to ≥ 80.

| # | WI | Bead | Hours | Score Δ |
|---|---|---|---|---|
| 1 | Fix `await_timer` to read `deadline_slot` (wait primitive is functionally broken) | vb-r4fix-001 | 6 | +8 |
| 2 | Add `Runtime::recover()` to re-insert `pending_timers` from Fjall | vb-r4fix-002 | 12 | +10 |
| 3 | Wire `admit_run_with_budget_policy` into production admission + lower policy to 1000 max steps | vb-r4fix-003, vb-o5zb.3.1 | 8 | +6 |
| 4 | Fix 7 source-length violations (split compiled_slug.rs canonical; add ledger rows for the rest) | vb-source-length-r2 | 37 | +5 |
| 5 | Section 65 SideEffect/RetrySafety migration to master taxonomy (rename enums + rewrite gates + rewire tests) | vb-yfveq (MAJOR-6) | 15 | +5 |
| 6 | Section 50 ArrayQueue migration (replace `crossbeam_channel` in IPC ingress + `Mutex<VecDeque>` in action queue) | vb-section50-1..4 | 11 | +4 |
| 7 | Section 17 dead-letter codes: SECRET_UNAVAILABLE fix + REPLAY_DIVERGED exit code fix + WAIT/ASK_TIMEOUT variants | vb-13d2a..d | 21 | +4 |
| 8 | Close 4 stale P0 beads; expand miri + coverage + mutants + fuzz + bench lanes; add test-density-gate | vb-r4mi/rcov/rmut/rfuz/rbnc/rdet/rpp1/rpp2 | 12 | +4 |
| 9 | Re-expand orchestrator pipeline: add `verify-kani-vb-validate`, wire flux/loom, re-admit `verify-proof` | vb-r4mi (flux/loom part) | 8 | +4 |

**Total: ~130h / 55h wall-clock with femdation dispatch.**

**Estimated release-candidate landing: 2026-06-19 (MVS), 2026-06-26 (Full DoD).**

---

## Full Backend / IR Interpreter Complete Backlog (124h)

After MVS, the full Definition of Done requires:

| Backlog | Beads | Hours |
|---|---|---|
| Fix the 4 remaining P0 critical runtime defects (wait/recovery/Kani/Flux) | vb-r4fix-001..006 | 40 |
| Section 50 ArrayQueue migration | vb-section50-1..4 | 11 |
| Section 65 SideEffect/RetrySafety migration | vb-yfveq (MAJOR-6) + 7 children | 15 |
| source-length gate repair | vb-source-length-r2 | 37 |
| Section 17 dead-letter codes | vb-13d2a..k (12 beads) | 52 |
| ResourceContract admission gap | vb-o5zb.3.1..6 | 30 |
| Duplicate IR types cleanup | vb-br993..vb-eq7lv (10 beads) | 7 |
| `$attempt.number` scope restriction fix | vb-scope-attempt.1..7 (7 beads) | 24 |
| Section 38 property tests + coverage | vb-cs38.1..11 (11 beads) | 41 |
| moon ci integrity + bead hygiene | vb-r4mi/rcov/rmut/rfuz/rbnc/rdet/rpp1/rpp2/rax1..13 (27 beads) | 118 |
| Section 8 reference root fix | vb-ref-roots.1..6 (6 beads) | 7 |
| **Total** | **~100+ new beads** | **~382h** |

---

## Definition of Done for Backend / IR Interpreter Complete (Master §44)

All of the following must be true:

1. **Build pipeline green.** `moon ci` exits 0 from a clean main checkout. All 21 pipeline tasks (plus the additional `runInCI: true` tasks) are NOT red.
2. **Master contract parity.** All 67 sections of master are matched:
   - §13 Resource contracts: 16 fields (not 18); `BoundednessPolicy::DEFAULT` enforces 1000-step ceiling
   - §14 Core types: ✓
   - §15 IR contract: 34 variants emitted; `LoadAccessor` present; `ExprOp` = 30
   - §16-17 Error codes: all 36 §16 codes raised; all 30 §17 codes raised
   - §18 Fjall: 9 keyspaces, 7 magics, 20 record kinds (no extras without master amendment)
   - §19 Action ABI: 7 SideEffect + 4 RetrySafety variants
   - §20 Shard: `BoundedActionCompletionQueue` uses `ArrayQueue`; `pending_timers` persistent
   - §21 IPC: ingress uses `ArrayQueue`; 11 commands at IDs 1..=11
   - §30 `tick_shard`: all 4 directives implemented
   - §36-39: 5x test density enforced; 11/11 property tests; 22/22 bench groups with real measurement
   - §50: no `crossbeam_channel` or `Mutex<VecDeque>` in any hot path
   - §65: master taxonomy enforced
3. **Runtime correctness.** Wait/Ask timer reads `deadline_slot` and persists; recovery from process restart re-inserts `pending_timers`, `runtime_states`, `journal_sequences`, `terminal_runs` from Fjall.
4. **Verifier coverage.** Kani harnesses cover ≥ 4 crates (not just vb_core); Miri covers ≥ 100% of master-required UB-relevant tests; Mutants covers ≥ 10% of vb_core functions; Fuzz covers ≥ 50% of targets for ≥ 60s each; Coverage lcov.info has real data with branch coverage.
5. **Bead hygiene.** All P0 beads closed or moved to non-blocked status. `test-determinism` returns 0 findings and is `runInCI: true`.
6. **Duplicate IR types eliminated.** Master contract cites canonical paths only. Dead files deleted. CI gate prevents re-introduction.
7. **`$attempt.number` restriction active.** `mod restrictions;` declared. `StepKindAst::Repeat` carries body. 19 tests run. `InvalidVariableScope` error variant exists.
8. **E2E test on the live binary** demonstrates: `wait` of 1 hour survives process restart and resumes at the original deadline; `kill` returns `Cancelled` from `inspect_run` (not `NotFound`); `SECRET_UNAVAILABLE` errors surface as `SECRET_UNAVAILABLE` in logs; `REPLAY_DIVERGED` returns exit code 8.
9. **Evidence pack at `.evidence/<bead>/`** for every closed bead, with raw command output.
10. **`git push` succeeds.** `bd dolt push` succeeds. `git status` is clean.

---

## Top 20 Prioritized Beads (by impact × likelihood)

| # | Bead | Severity | Title | User-visible impact |
|---|------|----------|-------|---------------------|
| 1 | vb-r4fix-001 | CRITICAL | Fix `await_timer` deadline_slot read | Every `wait: { until: 2099 }` resumes on the next tick |
| 2 | vb-r4fix-002 | CRITICAL | Add `Runtime::recover()` | Suspended runs lost on process restart |
| 3 | vb-section50-1..4 | CRITICAL | ArrayQueue migration | IPC ingress lock contention; action queue mutex thrash |
| 4 | vb-yfveq | CRITICAL | MAJOR-6 SideEffect/RetrySafety migration | Process/UnsafeShell actions silently retry |
| 5 | vb-r4fix-003, vb-o5zb.3.1 | CRITICAL | Wire admission budget gate | 50,000-step workflow admitted without complaint |
| 6 | vb-source-length-r2 | HIGH | Split 7 over-limit files | `moon ci` RED |
| 7 | vb-13d2a..d | HIGH | Section 17 dead-letter codes | SECRET_UNAVAILABLE miscategorized; REPLAY_DIVERGED exit 5 |
| 8 | vb-r4mi/rcov/rmut/rfuz/rbnc/rdet | HIGH | Expand 5 smoke-only lanes | False confidence in 21-task pipeline |
| 9 | vb-scope-attempt.1..7 | HIGH | Fix `$attempt.number` scope | Body of `repeat` steps silently dropped at parse time |
| 10 | vb-cs38.1..4 | HIGH | Section 38 ship-blocker proptests | `concurrency_safety` race condition; `bytecode_ast_parity` divergence |
| 11 | vb-br993..vb-eq7lv (10 beads) | HIGH | Delete duplicate IR types | Master contract cites dead files; future agents edit dead code |
| 12 | vb-rpp1 | HIGH | Remove phantom `nightly-feature-cargo-probe` | No-op task in pipeline |
| 13 | vb-rpp2 | HIGH | Add real `banned-token-gates` script | Phantom task in pipeline |
| 14 | vb-13d2e..k | MEDIUM | Section 17 remaining codes (FOR_EACH_ITEM, TOGETHER_BRANCH, etc.) | Body failure attribution lost |
| 15 | vb-cs38.5..9 | MEDIUM | Section 38 alias strengthenings | Registry coverage gaps |
| 16 | vb-ref-roots.1..6 | MEDIUM | Section 8 reference roots | 5 of 8 roots silently rejected |
| 17 | vb-o5zb.3.1..6 | MEDIUM | ResourceContract admission gap (full) | 1000x policy ceiling; 65x/8x hard limits |
| 18 | vb-1ev82, vb-8o7p5, vb-o5zb, vb-yesh4 | MEDIUM | Close 4 stale P0 beads | bd list does not reflect reality |
| 19 | vb-rax1..13 | MEDIUM | Re-admit 15 `runInCI: false` tasks or file beads | Half the verification surface is invisible |
| 20 | vb-cs38.10..11 | MEDIUM | Real coverage report + test-density gate | Stub 3-byte tarpaulin-report.json; 3.99x density unenforced |

---

## Recommended Sequencing (3-week wall-clock with femdation)

### Week 1: Quick wins + 4 P0 fixes
- Day 1-2: P0 stale bead closures (`vb-1ev82`, `vb-8o7p5`, `vb-o5zb`, `vb-yesh4`), 2 phantom task removals
- Day 3-4: P0 runtime defects (`await_timer` deadline, `Runtime::recover()`)
- Day 5: Kani harness unwinding fix; Flux annotations replacement

### Week 2: Section 50 + Section 65 + Section 17 dead-letters
- Day 1-2: Section 50 ArrayQueue migration (IPC ingress + action queue + scanner)
- Day 3: Section 65 SideEffect/RetrySafety migration
- Day 4-5: Section 17 SECRET_UNAVAILABLE + REPLAY_DIVERGED + WAIT/ASK_TIMEOUT

### Week 3: source-length + property tests + bead hygiene
- Day 1-2: 7 over-limit file splits (compiled_slug.rs first)
- Day 3: 4 SHIP-BLOCKER proptests (concurrency_safety, bytecode_ast_parity, taint_propagation, error_recovery)
- Day 4: ResourceContract admission gate
- Day 5: `test-determinism` 1,088 findings triage T0-T2

### Post-Week 3: Full Backend / IR Interpreter Complete DoD
- MVS-1..9 lands → SHIP score ≥ 80 by 2026-06-19
- Full backlog completes → SHIP score 90+ by 2026-06-26

---

## Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| `await_timer` fix breaks existing tests (most use synthetic `deadline_slot: SlotIdx::new(0)`) | HIGH | Tests fail; spec said "interpreter returns AwaitingWait without consulting the slot value" | Update tests; do not consult slot only in the "no deadline" path; preserve existing test behavior |
| `Runtime::recover()` introduces race conditions (concurrent recovery + new submits) | MEDIUM | Recovery could resurrect a run that was already killed | Make recovery check terminal_runs first; idempotent |
| Section 50 ArrayQueue migration changes MPMC → SPSC contract | HIGH | Single-producer/single-consumer becomes the only model | Use `crossbeam_queue::ArrayQueue` which is MPMC-safe; document the migration |
| Section 65 enum rename breaks 28 call sites | HIGH | 866-test "all pass" claim evaporates | Land in single commit; update tests in same commit |
| Source-length split breaks downstream proof artifacts | HIGH | 6 Flux + 3 fuzz + 2 Kani + 8 proptests re-binding | Cross-document citation update before split |
| Section 17 dead-letter code implementation requires CoreError variant additions | MEDIUM | CoreError is `#[non_exhaustive]` already | Use named-field variants |
| Duplicate IR type deletion breaks agents reading master | MEDIUM | Agents who edit `nodes.rs` per master will be confused | Update master contract in same commit |
| `test-determinism` re-inclusion breaks CI on first run | HIGH | CI is red on 1,088 findings immediately | Use `--baseline` file; archive current as `.beads/vb-rdet/baseline.json` |
| Beads closed without proper evidence | MEDIUM | `bd list` shows closed but no proof | Require raw log + `.evidence/<bead>/` file before `bd close` |

---

## Final Verdict: HOLD

**Estimated time-to-MVS-ship:** ~3 weeks wall-clock with parallel femdation dispatch (~130h focused).
**Estimated time-to-full-DoD-ship:** ~5 weeks wall-clock (~382h focused).
**Confidence level:** 95% that MVS-1..9 will lift SHIP to ≥ 80. 60% that the full backlog will reach 90+ (some Round 4 findings may surface as new defects during remediation).

The orchestrator works for the happy path (verified by Phase 6 E2E). The 41/100 SHIP score reflects the gap between "happy path works" and "Backend / IR Interpreter Complete per master Section 44." That gap is 100+ new beads, 380+ hours, and 5 weeks.
