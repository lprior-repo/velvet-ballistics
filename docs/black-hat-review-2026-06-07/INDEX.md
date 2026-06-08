# Full Transcript — Velvet-Ballistics Black-Hat Review + E2E Test

**Date:** 2026-06-07
**Workspace:** /home/lewis/src/velvet-ballistics
**Checkout:** 106b5b621 (clean main)
**Total subagents dispatched:** 60 (5 rounds × 12 agents)
**Total wall-clock agent time:** ~5 hours (parallel)
**Final verdict:** **HOLD — SHIP score 41/100**

---

## File Layout

```
/tmp/opencode/transcripts/
├── MASTER-ROADMAP.md                        (12.8 KB) — Final SHIP decision
├── e2e-test-results.md                       (7.2 KB) — Phase 6 hands-on test
├── e2e-workflow.yaml                        (256 B) — Test workflow
├── e2e-db/                                  (Fjall journal from test run)
├── round1/
│   └── INDEX.md                             (R1 inventory summaries)
├── round2/                                  (R2 raw gate evidence)
│   ├── r2-a1-fmt.txt                        (12s PASS)
│   ├── r2-a1-lint.txt                       (55s PASS)
│   ├── r2-a2-check.txt                      (27s PASS)
│   ├── r2-a2-hardened.txt                   (53s FAIL: source-length RED)
│   ├── r2-a3-test.txt                       (617s PASS, 14,305 tests)
│   ├── r2-a4-doctest.txt                    (1s PASS, 3 tests)
│   ├── r2-a4-doc.txt                        (4s PASS)
│   ├── r2-a5-source-length.txt              (FAIL, 7 over-limit files)
│   ├── r2-a5-nightly-feature.txt            (PASS)
│   ├── r2-a6-fuzz.txt                       (34s PASS)
│   ├── r2-a6-mutants.txt                    (59s PASS, 1/3773 caught)
│   ├── r2-a7-miri.txt                       (61s PASS, 3 tests)
│   ├── r2-a7-asan.txt                       (1.9 MB, 14,182 tests PASS)
│   ├── r2-a8-supply-chain.txt               (17s PASS)
│   ├── r2-a8-feature-powerset.txt           (58s PASS, 71 combos)
│   ├── r2-a9-bench-build.txt                (53s PASS)
│   ├── r2-a9-coverage.txt                   (75s PASS, 2.58% smoke)
│   ├── r2-a10-kani.txt                      (31s PASS, 4 harnesses)
│   ├── r2-a11-verus.txt                     (8s PASS, 21/21)
│   ├── r2-a11-tlc.txt                       (2s PASS, 2/2)
│   └── r2-a12-moon-ci.txt                   (2 MB, 11m26s, 2 FAIL)
├── round3/
│   └── INDEX.md                             (R3 gap analysis summaries)
├── round4/
│   ├── r4-a1-section50.md                   (Section 50 LETHAL)
│   ├── r4-a2-section65.md                   (Section 65 LETHAL)
│   ├── r4-a3-resource-contract.md           (ResourceContract SHIP-BLOCKER)
│   ├── r4-a4-section17.md                   (Section 17 dead-letters)
│   ├── r4-a5-coverage.md                    (Test density + coverage)
│   ├── r4-a6-source-length.md               (Source-length drift)
│   ├── r4-a7-section38.md                   (Property tests gap)
│   ├── r4-a8-duplicate-ir.md                (Duplicate IR types)
│   ├── r4-a9-attempt-scope.md               ($attempt.number gap)
│   ├── r4-a10-bench-duplication.md          (Bench duplication)
│   ├── r4-a11-moon-ci.md                    (Pipeline integrity)
│   └── r4-a12-integration.md                (Final integration)
└── round5/
    └── INDEX.md                             (R5 plan summaries)
```

---

## Phase 1-5: 60 Subagent Outputs

See individual round INDEX files for summaries. Each round had 12 parallel subagents dispatched.

### Round 1: Codebase Inventory (12 agents)
- **Type:** explore
- **Output:** 12 markdown reports (1.5K-7K lines each)
- **Coverage:** vb_core, vb_yaml, vb_validate, vb_expr, vb_compile, vb_storage, vb_runtime, vb_ipc, vb_cli, vb_benchmark, workspace_tests, fuzz/xtask, .moon/supply-chain
- **Index:** `round1/INDEX.md`

### Round 2: Build/Verify/Test Gate Execution (12 agents)
- **Type:** general
- **Output:** 19 raw evidence files (`r2-*.txt`) + summary reports
- **Coverage:** moon :fmt, :lint-src, :check, :hardened-build, :test, :doc-test, :doc, :source-length, :nightly-feature-gate, :fuzz-smoke, :mutants-smoke, :miri, :sanitizer-address-check, :supply-chain, :feature-powerset, :bench-build, :coverage, :verify-kani, :verify-verus, :verify-tlc, :moon ci
- **Result:** 17/19 individual gates PASS, 2 FAIL (source-length + sanitizer-address-check), full `moon ci` RED
- **Evidence:** `round2/r2-*.txt` (raw) — these are the actual command outputs

### Round 3: Master-Contract Gap Analysis (12 agents)
- **Type:** general
- **Output:** 12 markdown reports
- **Coverage:** §13, §14, §15, §18, §19+§20, §21+§30+§33, §46+§47, §50 (LETHAL), §16+§17, §65 (LETHAL), §8+§9+§10, §36+§37+§38+§39
- **Index:** `round3/INDEX.md`

### Round 4: Adversarial Review (12 agents)
- **Type:** black-hat-reviewer
- **Output:** 12 markdown reports (each 48-142 lines)
- **Coverage:** All LETHALs + SHIP-BLOCKERs attacked; P0 bead reconciliation; final integration
- **Index:** `round4/r4-a*.md` (12 files in this directory)

### Round 5: Evidence-Pack + Remaining-Work (12 agents)
- **Type:** general
- **Output:** 12 implementation plans written to `to-fix/`, `states/`, `docs/`, `.bead-progress/`, `.evidence/`
- **Total backlog:** 100+ new beads, 382h estimated work
- **Index:** `round5/INDEX.md`

---

## Phase 6: Hands-On End-to-End Test

See `e2e-test-results.md` for the full report.

**15/15 E2E steps PASS:**
1. `velvet-ballistics version` → "velvet-ballistics 0.1.0"
2. `velvet-ballistics help` → 30 subcommands listed
3. `validate e2e-workflow.yaml` → "valid"
4. `simulate e2e-workflow.yaml` → 3 steps dry-run
5. `run` → 11 events emitted, 3 steps executed
6. `inspect 1` → "status=finished, events=11"
7. `events 1` → 11 events with correct kinds
8. `trace 1` → step-by-step execution trace
9. `replay 1` → recovers 11 events from Fjall
10. `status` → "status: running / active_runs: 0 / step_budget_per_tick: 1000"
11. `verify --profile full` → 5 gates passed
12. `doctor` → "all checks passed"
13. `agent-context` → 200-line JSON schema
14. `submit` → "status: submitted" (queues to journal)
15. `ai-context 1` → structured JSON

**The orchestrator works for the happy path. The 41/100 SHIP score reflects the gap between "happy path works" and "Backend / IR Interpreter Complete per master Section 44."**

---

## Master Roadmap

See `MASTER-ROADMAP.md` for the full synthesis.

**SHIP SCORE: 41/100**

| Component | Weight | Score |
|---|---:|---:|
| Build + Test + Lint | 0.20 | 70 |
| Master contract | 0.20 | 64 |
| Verified runtime correctness | 0.25 | 25 |
| CI integrity | 0.15 | 30 |
| Coverage evidence | 0.10 | 5 |
| Bead hygiene | 0.10 | 30 |

**RELEASE DECISION: HOLD**

**9 MVS work items** (130h) bring SHIP to ≥ 80 by **2026-06-19** (3 weeks with femdation parallel dispatch).

**Full Backend / IR Interpreter Complete DoD** requires 382h of work, landing by **2026-06-26** (5 weeks).

---

## Critical Findings Summary

### Top 5 SHIP-BLOCKERS

1. **Wait/Ask timer deadline is silently ignored** — `await_timer` in `transitions.rs:171` uses `Instant::now()` instead of reading `deadline_slot`. Every `wait: { until: 2099 }` resumes on the next tick.
2. **No recovery of pending wait timers after process restart** — `pending_timers` is in-memory only; `Runtime::new_with_journal` creates empty shards; Fjall journal has the events but no code path reads them.
3. **Section 50 ArrayQueue LETHAL** — `MemoryIngress` uses `crossbeam_channel::bounded` (forbidden); `BoundedActionCompletionQueue` uses `Mutex<VecDeque>` (wrong backend); no MAJOR-1 bead; scanner only catches unbounded.
4. **Section 65 SideEffect/RetrySafety taxonomy drift** — Production 5+3 vs master 7+4; tests dead; gates enforce broken; `Process` and `UnsafeShell` actions silently retry.
5. **Section 17 dead-letter error codes** — 11/30 runtime codes never constructed; `SECRET_UNAVAILABLE` misrouted to `ARTIFACT_MALFORMED` (security classification failure); `REPLAY_DIVERGED` returns exit code 5 not 8.

### Top 5 HIGH-severity findings

6. **ResourceContract admission gap** — 50,000-step workflow admitted without complaint; `BoundednessPolicy::DEFAULT.max_total_steps = 1_000_000` (1000× master 1000); `vb-o5zb.3` closed against unmet criteria.
7. **Test density 3.99x vs master 5x** — unenforced in CI; `tarpaulin-report.json` is 3 bytes; `frame.rs` at 1.70x; `vb_compile` self-marks 4.00x as [PASS].
8. **Source-length gate RED** — 7 files over 300 lines, 2 stale exceptions; 497-row exception ledger is a permanent waiver list.
9. **4 of 11 Section 38 property tests SHIP-BLOCKER** — `concurrency_safety` race condition in `IntrospectionRegistry`; `bytecode_ast_parity` is a lying disabled test (file doesn't exist); `taint_propagation` is 2,578 lines of hand-coded unit tests with 0 proptest macros.
10. **Duplicate IR types** — `nodes.rs` (4-field) vs canonical `workflow/types.rs` (6-field); master contract cites dead files; 884-line `validation/` directory is a parallel universe that references a `Branch` type that doesn't exist.

### Top 5 MEDIUM-severity findings

11. **$attempt.number scope restriction gap** — `mod restrictions;` not declared; 19 dead tests; cold AST `StepKindAst::Repeat` has no `body` field.
12. **Section 8 reference root gap** — 5 of 8 allowed roots silently rejected as `UnknownReference`; `$time` rejected with wrong diagnostic.
13. **Bench duplication** — 12 `*_root_migrated.rs` files DIVERGED (0 of 12 byte-identical); `action_dispatch_root_migrated.rs:10` has fatal syntax error invisible to compiler; 0 of 12 have real measurement evidence.
14. **moon ci integrity** — `test-determinism` hides 1,088 findings; 5 of 21 pipeline tasks are smoke-only; 2 phantom tasks; 4 stale P0 beads.
15. **Flux annotations vacuous** — 12 `#[flux_rs::trusted]` with `true` bodies in `flux_cancel_kill.rs`; no actual proof.

---

## P0 Bead Reconciliation

| Bead | Status | Effective State | Action |
|------|--------|-----------------|--------|
| vb-1ev82 | BLOCKED | Code fixed, State 6 reviewer REJECTED | Re-verify, re-run State 5/6, close |
| vb-8o7p5 | BLOCKED | Kani timeout on 3 harnesses | Add unwind flags, file child |
| vb-o5zb | BLOCKED | All 5 children closed | Close parent |
| vb-yesh4 | BLOCKED | Not reproducible in main | Close as deferred |

---

## How to Read This Transcript

1. **Start with `MASTER-ROADMAP.md`** for the final SHIP decision
2. **Read `e2e-test-results.md`** to see what actually works on the live binary
3. **For each round's 12 subagent outputs:**
   - `round1/INDEX.md` — what we found
   - `round2/r2-*.txt` — what we ran
   - `round3/INDEX.md` — what the master says vs what we have
   - `round4/r4-a*.md` — what we attacked (12 individual reviews)
   - `round5/INDEX.md` — what to do next
4. **For implementation work:** see individual plan files at `to-fix/*`, `states/*`, `docs/*`, `.evidence/*`

---

## Total Work-to-Ship

- **130h** for MVS (SHIP ≥ 80) — 3 weeks
- **382h** for Full DoD (SHIP ≥ 90) — 5 weeks
- **~100+ new beads** filed across all 5 rounds
- **5,500+** documented findings
- **60** subagent dispatches
- **17/19** individual gates PASS, 2 FAIL
- **15/15** hands-on E2E steps PASS
- **41/100** current SHIP score
- **1** core orchestrator working
- **1** clear decision: HOLD
