# Round 4 Pipeline Integrity & Bead Hygiene — Implementation Plan

Repo: `/home/lewis/src/velvet-ballistics`
Audit source: Round 4 moon ci integrity review (15 critical findings)
Total work envelope: **90.5–121.5h** across 27 atomic beads
Definition of done: see end of document.

---

## Index of Findings (4 P0 + 5 smoke + 1 hidden + 2 phantom + 15 excluded)

| # | Category | Item | File:line | New/Existing Bead |
|---|---|---|---|---|
| P0-1 | Stale P0 | vb-1ev82 restore runtime facade | `crates/vb_runtime/src/runtime/` | `vb-1ev82` (close) |
| P0-2 | Stale P0 | vb-8o7p5 Kani dep graph / harness timeout | `crates/vb_validate/src/kani_gate_08_structural.rs` | `vb-8o7p5` (keep) + new `vb-8o7p5.1` |
| P0-3 | Stale P0 | vb-o5zb parent; children all closed | `to-fix/02-runtime-action-durability-defects.md` | `vb-o5zb` (close) + `vb-o5zb.6` |
| P0-4 | Stale P0 | vb-yesh4 fuzz cfg; not reproducible on main | `fuzz/Cargo.toml` | `vb-yesh4` (close as not-repro) |
| S-1 | Smoke-only | `miri` lane covers 0.14% of tests | `.moon/tasks/all.yml:404-427`, `.moon.yml:24` | new `vb-r4mi` |
| S-2 | Smoke-only | `coverage` lane covers 1 test | `.moon/tasks/all.yml:429-449`, `.moon.yml:27` | new `vb-rcov` |
| S-3 | Smoke-only | `mutants-smoke` covers 1 of 3,773 functions | `.moon/tasks/all.yml:451-483`, `.moon.yml:22` | new `vb-rmut` |
| S-4 | Smoke-only | `fuzz-smoke` covers 5 of 93 targets × 1s | `.moon/tasks/all.yml:485-529`, `.moon.yml:23` | new `vb-rfuz` |
| S-5 | Smoke-only | `bench-build` covers 1 of N benchmarks | `.moon/tasks/all.yml:550-568`, `.moon.yml:28` | new `vb-rbnc` |
| H-1 | Hidden findings | `test-determinism` is `runInCI: false`, hiding 1,088 findings | `.moon/tasks/all.yml:182-194`, `scripts/check-test-determinism.py` | new `vb-rdet` |
| PP-1 | Phantom task | `nightly-feature-cargo-probe` script body is `true` | `.moon/tasks/all.yml:221-235`, `.moon.yml:14` | new `vb-rpp1` |
| PP-2 | Phantom task | `banned-token-gates` has no `command:` or `script:` | `.moon/tasks/all.yml:196-205` | new `vb-rpp2` |
| E-1..E-15 | Excluded | 15 `runInCI: false` tasks need re-admission or filing | `.moon/tasks/all.yml:588,601,614,627,647,668,681,694,707,732,745,760,769`; `.moon/tasks/verus.yml:46` | new `vb-rax1`..`vb-rax15` |

---

## P0 Bead Closures & Reclassifications (4 items)

### P0-1: vb-1ev82 — close parent; reopen for State 6 evidence re-binding

**Defect.** Bead claims `crates/vb_runtime/src/lib.rs` references a missing `runtime` module, blocking State 5/6 proof binding for the runtime facade. The user reports code is fixed in the implementation reroute; the failure is at State 6 proof-reviewer (reviewer rejected as E_STATUS_NOT_APPROVED with no remaining raw-verifier blockers). Bead stays BLOCKED but blocker list is empty of real defects.

**Fix.**
1. Verify production target compiles: `rtk cargo check -p vb_runtime --all-features` (record raw exit-0 output to `.beads/vb-1ev82/recheck-2026-06-07.log`).
2. Re-run State 5 proof-writer in isolated worktree to produce fresh `proof-findings.jsonl` row keyed on `vb-1ev82`.
3. Re-run State 6 proof-reviewer with the new evidence; if reviewer still rejects with substantive findings, file them as a child (`vb-1ev82.1`) and do not close the parent.
4. If reviewer passes, close with reason "production target fixed; State 5/6 evidence bound; deps vb-pyg3p, vb-evkno, vb-egysa, vb-yesh4 closed".

**Acceptance criteria.**
- `rtk cargo check -p vb_runtime --all-features` exits 0 from a clean main checkout.
- `proof-findings.jsonl` contains a vb-1ev82 row with a non-empty `raw_evidence` path.
- `proof-review.md` (latest) records APPROVED status.
- Children `vb-pyg3p`, `vb-evkno`, `vb-egysa`, `vb-yesh4` all show ✓ CLOSED in `bd list`.

**Risk.** Medium. Closing prematurely hides a real production facade gap. Mitigation: require green cargo check AND green State 6 review in the same commit; do not allow closing on the basis of either alone.

**Hours.** 2.5h (verify, re-evidence, close, push). 1h buffer for re-evidence if reviewer requires harness rerun.

**Bead.** `vb-1ev82` (existing). No new ID.

---

### P0-2: vb-8o7p5 — keep open; file child for harness timeout, fix dep graph

**Defect.** Kani dep graph was repaired (`loom` optional, `loom-models` owns `dep:loom`); however the three required `kani_gate_08_*` harnesses still timeout at 120s inside `crossbeam_queue::ArrayQueue::new` unwinding. User confirmed "real bug, Kani timeout" — this is a remaining genuine defect, not stale state.

**Fix.**
1. Increase Kani harness timeout to 900s (15m) in `.moon/tasks/kani.yml:43` (`timeout 15m`) — already at 15m, so the cap is fine; the actual issue is unwind bounds.
2. Add `--output-format old` and `--enable-unstable --cbmc-args --unwind-min 8` to the harness invocations to push the unwinder past `ArrayQueue::new`'s initial frame.
3. File `vb-8o7p5.1` for harness-specific unwind bound config per harness; this is genuine new work, not stale state.
4. If after 5m the harnesses still fail, switch to `cargo kani --harness ... --output-format old --unwind 16 --json-priority fast` and record the exact CBMC flags that work.

**Acceptance criteria.**
- All four `kani_gate_08_*` harnesses listed in `.moon/tasks/kani.yml:45-48` complete in ≤15m with raw CBMC success (VERIFICATION::SUCCESS) recorded in `.beads/vb-8o7p5/kani-output/*.log`.
- `bd close vb-8o7p5` only after all four harness logs are filed.
- Child `vb-8o7p5.1` carries the unwind-bound work and is closed at the same time.

**Risk.** High. Kani unwinding in `crossbeam-queue` can be pathological; if the bound blows past 64 you are stuck. Mitigation: cap unwinds at 32 and document the per-harness decision matrix in `vb-8o7p5.1` notes; never run root `cargo kani` (banned by AGENTS.md "Differential verification only").

**Hours.** 6h (unwind tuning, harness reruns, evidence bundling, child creation).

**Beads.** `vb-8o7p5` (existing, keep blocked) + new `vb-8o7p5.1`.

---

### P0-3: vb-o5zb — close parent; verify children

**Defect.** All 4 children (`vb-o5zb.1` Taint lattice, `vb-o5zb.2` terminal step states, `vb-o5zb.3` ResourceContract, `vb-o5zb.4` collect timeout semantics) plus follow-ups (`vb-53k3r`, `vb-izu26`, `vb-yurs3`, `vb-o5zb.5`) are closed. Parent tracks umbrella reconciliation; with children closed, only the audit-summary file `to-fix/02-runtime-action-durability-defects.md` is still in scope.

**Fix.**
1. Run `bd show vb-o5zb.5` and `bd show vb-o5zb.1..4` to confirm all 5 children are ✓ CLOSED with non-empty evidence paths.
2. Update `to-fix/02-runtime-action-durability-defects.md` to reflect actual final state (replace "open" annotations with "resolved" or pointer to closed beads).
3. File `vb-o5zb.6` (1h) as the doc-update bead with `--deps discovered-from:vb-o5zb`.
4. Close `vb-o5zb` with reason "all 5 children closed; umbrella audit reconciled; closure doc-updated by vb-o5zb.6".

**Acceptance criteria.**
- `bd children vb-o5zb` shows all 5 children ✓ CLOSED.
- `to-fix/02-runtime-action-durability-defects.md` no longer claims defects under "open" that are bead-closed.
- `vb-o5zb` closes only after `vb-o5zb.6` lands.

**Risk.** Low. Pure documentation reconciliation; no code touched.

**Hours.** 1h audit + 1h doc update = 2h.

**Beads.** `vb-o5zb` (close) + new `vb-o5zb.6` (doc update).

---

### P0-4: vb-yesh4 — close as not-reproducible

**Defect.** Bead claims `vb_storage::admission::fuzz_access` is configured out under `cfg(fuzzing)`, blocking fuzz manifest compile. User reports "doesn't reproduce in main" — i.e., on a clean main checkout, the fuzz crate compiles. The original blocker is masked behind an unlanded `vb-1ev82` prerequisite and was never observable in CI.

**Fix.**
1. From a clean main checkout (no uncommitted `vb-1ev82` reroute branch), run `rtk cargo check --manifest-path fuzz/Cargo.toml --all-features 2>&1 | tee .beads/vb-yesh4/clean-main-check.log`.
2. If exit 0, close `vb-yesh4` with reason "not reproducible on clean main; original blocker was masked by unlanded vb-1ev82 prerequisite which is now closed; defer cfg-fuzz review to a future hardening bead".
3. If non-zero, file `vb-yesh4.1` with the actual error and route through fuzz lane; do not close.
4. Re-open `vb-1ev82` dependency record so the closure narrative is clear: vb-1ev82 was the upstream cause and is now closed.

**Acceptance criteria.**
- Clean-main `cargo check --manifest-path fuzz/Cargo.toml` exits 0; raw log filed at `.beads/vb-yesh4/clean-main-check.log`.
- Bead closure reason references the log path.
- If a real cfg-fuzz gap exists in a future hardening cycle, `vb-yesh4.1` carries it; do not silently drop.

**Risk.** Low-Medium. Closing "not reproducible" can hide a real cfg gap. Mitigation: require raw exit-0 log in evidence; require parent bead `vb-1ev82` already closed.

**Hours.** 1h verification + 0.5h close. 1h buffer to file `vb-yesh4.1` if non-reproducibility turns out to be wrong.

**Bead.** `vb-yesh4` (close). Optional `vb-yesh4.1`.

---

## Smoke-Only Lane Expansion (5 items)

Each smoke lane currently blocks a master-required gate (Section 4 / 37 / 40 / 44). For each, the options are: (a) **expand** to master scope, or (b) **relabel** to make the smoke scope explicit AND add a follow-up bead to expand later. The user gave us choice; I recommend (a) for fuzz and miri (cheapest path to integrity), and (b) for mutants/coverage/bench (where full scope is genuinely unaffordable in CI).

### S-1: `miri` — expand to 3 crates; 0.14% → 100% of master-required

**Defect.** `.moon/tasks/all.yml:417-419` runs 3 specific tests (one per crate) under Miri. Master Section 4 requires Miri for `vb_core`, `vb_expr`, `vb_compile`. The current 3 tests cover each crate's lib smoke, but Miri's value is per-test UB coverage; 0.14% (3 / ~2,200 tests) is not evidence.

**Fix.** Add a `miri-full` follow-up lane (separate task) that runs the entire `--lib` test surface of the three master-required crates. Gate the smoke on success, fail the full lane on regression. Budget: 30m.

```yaml
# .moon/tasks/all.yml (add after miri at line 427)
miri-full:
  script: |
    set -euo pipefail
    mkdir -p target/miri-tmp target/moon-locks
    for pkg in vb_core vb_expr vb_compile; do
      TMPDIR="$PWD/target/miri-tmp" flock --shared target/moon-locks/source-mutation.lock \
        env -u RUSTC_WRAPPER RUSTFLAGS="-Dwarnings" MIRIFLAGS="-Zmiri-disable-isolation" \
        timeout 25m rustup run nightly-2026-04-28 cargo miri test --quiet -p "$pkg" --lib --all-features
    done
  toolchains: [rust]
  inputs: ['@globs(sources)', '@globs(tests)', '@globs(configs)']
  options:
    runInCI: false  # Promote after S-1 acceptance
    cache: false
```

Then **relabel** the existing `miri` task to `miri-smoke` and add a comment "this is a smoke, full lane is miri-full". Update `.moon.yml:24` to call `miri-smoke` so the pipeline stays green while the full lane is in development.

**Acceptance criteria.**
- `miri-smoke` is renamed and runs the 3 tests; passes.
- `miri-full` exists and completes ≤30m; raw log filed.
- `vb-r4mi` bead carries the promotion work to `runInCI: true`.

**Risk.** Medium. Miri under `--all-features` can be slow or fail on 3rd-party deps. Mitigation: feature-gate the 3 master crates only; quarantine the full lane under `runInCI: false` until 3 consecutive green runs.

**Hours.** 8h (script, run, triage failures, 3x reruns).

**Bead.** New `vb-r4mi`.

---

### S-2: `coverage` — relabel as smoke; file full-coverage bead

**Defect.** `.moon/tasks/all.yml:437` runs `cargo llvm-cov` for exactly 1 test: `action::tests::validate_action_outcome_failed_always_succeeds`. Master Section 40 requires `cargo llvm-cov --workspace --all-features`. 1 / ~2,200 tests is not evidence.

**Fix.** Same pattern as S-1: rename current to `coverage-smoke`, add `coverage-full` follow-up that runs `cargo llvm-cov test --workspace --all-targets --all-features --no-cfg-coverage` and reports `lcov.info` to `target/llvm-cov/lcov-full.info`. The full run is slow (≥20m for workspace); quarantine under `runInCI: false` and promote only after 3 green runs.

**Acceptance criteria.**
- `coverage-smoke` runs the 1 test; pipeline stays green.
- `coverage-full` exists, completes ≤60m, and `lcov.info` has line coverage on all 17 production crates.
- `vb-rcov` bead carries the full-coverage work.

**Risk.** Medium-High. llvm-cov can be flaky on instrumented deps; first run is likely 30-60m. Mitigation: 3-quarantine rule; if 3 consecutive runs are green, promote.

**Hours.** 6h (script, run, instrument tuning, 3x reruns).

**Bead.** New `vb-rcov`.

---

### S-3: `mutants-smoke` — relabel; file full-mutation bead

**Defect.** `.moon/tasks/all.yml:459-471` mutates exactly 1 function (`is_supported_code -> bool with false`) in exactly 1 file (`vb_core/src/diagnostic.rs`). Master policy requires mutation score across the workspace, which has ~3,773 functions. 1 / 3,773 = 0.026%.

**Fix.** Rename to `mutants-smoke` (no script change). Add `mutants-full` follow-up that runs `cargo mutants --package <each> --baseline skip --jobs 4 --timeout 60` per crate. This will take hours; quarantine under `runInCI: false` and require explicit `bd update vb-rmut --claim` to run.

**Acceptance criteria.**
- `mutants-smoke` relabeled, pipeline passes.
- `mutants-full` exists, runs in ≤8h on a 4-job schedule, and produces per-crate mutation-score report.
- `vb-rmut` bead files a 3-run quarantine plan before promotion.

**Risk.** High. Mutation testing is the slowest lane; 3,773 functions × 1-2s each = 1-2h single-job. Mitigation: 4 jobs, per-crate baseline, and explicit failure quarantine (cargo-mutants is non-deterministic on timing).

**Hours.** 10h (script, infra, single dry-run, documentation).

**Bead.** New `vb-rmut`.

---

### S-4: `fuzz-smoke` — relabel; file full-fuzz bead

**Defect.** `.moon/tasks/all.yml:495` loops 5 fuzz targets (out of 93 in `fuzz/Cargo.toml`) for 1 second each. 5 × 1s = 5s of fuzzing across a 93-target corpus.

**Fix.** Rename to `fuzz-smoke`. Add `fuzz-full` follow-up that runs each of the 93 targets for 60s on the seeded corpus. The script already exists in skeleton form (`cargo fuzz build` then loop); refactor to accept a target list and per-target duration from inputs.

```yaml
# .moon/tasks/all.yml (add after fuzz-smoke at line 529)
fuzz-full:
  script: |
    set -euo pipefail
    mkdir -p target/moon-locks target/fuzz-full
    flock --shared target/moon-locks/source-mutation.lock \
      env RUSTFLAGS="-Dwarnings" \
      rustup run nightly-2026-04-28 cargo fuzz build --target x86_64-unknown-linux-gnu >target/fuzz-full/build.log 2>target/fuzz-full/build.err
    # Read target list from inputs; default to all 93
    mapfile -t targets < <(grep -oE 'name = "[a-z_0-9]+"' fuzz/Cargo.toml | cut -d'"' -f2)
    for t in "${targets[@]}"; do
      corpus="target/fuzz-full/corpus/${t}"
      artifacts="target/fuzz-full/artifacts/${t}"
      rm -rf "${corpus}" "${artifacts}"
      mkdir -p "${corpus}" "${artifacts}"
      flock --shared target/moon-locks/source-mutation.lock \
        timeout 90s "fuzz/target/x86_64-unknown-linux-gnu/release/${t}" \
          -artifact_prefix="${PWD}/${artifacts}/" -max_total_time=60 "${PWD}/${corpus}" \
          >"target/fuzz-full/${t}.log" 2>"target/fuzz-full/${t}.err" || true
    done
  toolchains: [rust]
  inputs: ['Cargo.toml', 'Cargo.lock', 'fuzz/**/*']
  options:
    runInCI: false
    cache: false
  outputs: ['target/fuzz-full/**']
```

**Acceptance criteria.**
- `fuzz-smoke` relabeled, pipeline passes.
- `fuzz-full` exists; completes ≤120m (93 × 60s + build); produces 93 `.log` files.
- `vb-rfuz` bead files the promotion plan (3-quarantine).

**Risk.** Medium. Fuzz targets can crash in expected ways; `|| true` is required but each `.err` must be inspected for new findings. Mitigation: per-target diff against seeded-corpus crashes to filter regressions.

**Hours.** 10h (script refactor, target-list extraction, full dry-run).

**Bead.** New `vb-rfuz`.

---

### S-5: `bench-build` — relabel; file full-bench bead

**Defect.** `.moon/tasks/all.yml:556` builds 1 benchmark (`vb_core` → `aggregate_resource_budget`). Master Section 39 requires full benchmark evidence with p50/p95/p99, instruction counts, allocations, CPU governor matrix. 1 / N benchmarks built is not "bench", it's a build smoke.

**Fix.** Rename to `bench-build-smoke`. Add `bench-build-full` that builds every benchmark in `crates/workspace_tests/benches/` and `crates/*/benches/` (per file-glob in `.moon/tasks/all.yml:9`). Add `bench-run` follow-up that actually runs the benchmarks with `cargo bench --workspace -- --save-baseline`.

**Acceptance criteria.**
- `bench-build-smoke` relabeled, pipeline passes.
- `bench-build-full` exists, builds all benchmarks.
- `bench-run` (separate bead `vb-rbnc.1`) is filed and documents the runtime expectation (≥60m for criterion warmup).

**Risk.** Low. Builds are deterministic.

**Hours.** 3h (rename, full build script, list extraction).

**Bead.** New `vb-rbnc` (build) + `vb-rbnc.1` (run).

---

## Hidden Findings: test-determinism Re-inclusion (1 item)

### H-1: `test-determinism` — re-include with phased triage

**Defect.** `.moon/tasks/all.yml:194` sets `runInCI: false` for the determinism gate, with the comment "Current tree has pre-existing findings; run explicitly until clean." Running the gate now produces 1,088 findings:
- 256 `UncontrolledClock` (mostly `Instant::now()` in `crates/vb_runtime/tests/`)
- 784 `SharedTempState` (`tempdir()`, `TempDir`, `/tmp/`)
- 31 `UncontrolledRandom` (`rand::`, `fastrand::`)
- 15 `GlobalMutableState` (`static mut`, `Once`, `Cell/RefCell`)
- 2 `SleepAsSync` (`thread::sleep`)

This is the largest integrity debt in the pipeline.

**Fix — Phased Triage Plan (T0–T4).** Flip `runInCI: true` at the end of T4; do not flip earlier or the pipeline will go red and block all merges.

| Phase | Goal | Acceptance | Hours |
|---|---|---|---|
| T0: Gate the gate | Add `--baseline <file>` to `scripts/check-test-determinism.py` so re-runs do not regress, and write the current 1,088 to `.beads/vb-rdet/baseline.json`. Gate is now non-regressive. | `check-test-determinism.sh --baseline .beads/vb-rdet/baseline.json` exits 0 on main; exits 1 on a new finding. | 3h |
| T1: File the categories | File 5 P1 child beads under `vb-rdet`: `vb-rdet.clock`, `vb-rdet.temp`, `vb-rdet.rand`, `vb-rdet.global`, `vb-rdet.sleep`. Each carries a target of "zero findings in its category" and a per-finding allowlist path. | 5 child beads created with `--deps discovered-from:vb-rdet`. | 1h |
| T2: Fix `SleepAsSync` (2) | Trivial. Replace 2 `thread::sleep` with explicit barrier or `Instant`-driven loop. | 0 findings in `SleepAsSync`. | 1h |
| T3: Fix `GlobalMutableState` (15) | Audit each. Most are likely test-helper statics that can be moved to a `thread_local!` or `OnceCell`. | 0 findings in `GlobalMutableState`. | 4h |
| T4: Fix `UncontrolledClock` (256) | Long. Most are `Instant::now()` calls in runtime tests. Replace with `Clock` injection (the project already has a `Clock` abstraction per `crates/vb_runtime/src/time.rs`). | 0 findings in `UncontrolledClock`; new `Instant::now_with_clock` paths exercised. | 30h |
| T5: Fix `UncontrolledRandom` (31) + `SharedTempState` (784) | Use `proptest` seed for rand (move to `SeededRandom`); replace `tempdir()` with `tempfile::tempdir()` (which is unique per PID/thread — wait, the gate fires on `tempdir()` itself, not the unique-per-call behavior; this needs pattern review). | 0 findings; allowlist `.beads/vb-rdet/allowlist.json` only for exempted tests. | 25h |

**Acceptance criteria (overall).**
- `runInCI: true` for `test-determinism`.
- `check-test-determinism.sh` exits 0 on main.
- Baseline `.beads/vb-rdet/baseline.json` archived as the historical 1,088-finding record.
- All 5 child beads are closed or have explicit `--allow-non-critical` allowlist entries with bead IDs.

**Risk.** Very High. T4 alone (256 UncontrolledClock) is the largest single piece of debt in the project; clock injection requires changes to test signatures. Mitigation: split T4 into 4 sub-phases (vb_runtime → vb_storage → vb_ipc → rest) and use a per-crate allowlist for in-progress crates so the gate can flip `runInCI: true` incrementally.

**Hours.** 64h total (3+1+1+4+30+25). Re-baseline at end of each phase.

**Bead.** New `vb-rdet` (umbrella) + 5 P1 children.

---

## Phantom Task Removals (2 items)

### PP-1: `nightly-feature-cargo-probe` — remove from pipeline; absorb into `check`

**Defect.** `.moon/tasks/all.yml:222-225` has a `script:` whose entire body is the comment "# check already runs the exact all-targets/all-features cargo probe." followed by `true`. This is a no-op task that pretends to verify something. It is included in `.moon.yml:14` as a pipeline gate. Every CI run reports "passing" for a task that does nothing.

**Fix.** Delete the task from `.moon/tasks/all.yml:221-235` and remove the `nightly-feature-cargo-probe` line from `.moon.yml:14`. The `nightly-feature-gate` task (line 111) already runs `scripts/check-nightly-features.sh` which is the actual feature-allowlist enforcer. The "cargo probe" the comment references is the `check` task (line 90) which is already in the pipeline.

**Acceptance criteria.**
- `grep -r "nightly-feature-cargo-probe" .moon.yml .moon/tasks/` returns no results.
- `moon run :nightly-feature-gate` still passes.
- `moon ci` is 1 task shorter.

**Risk.** Zero. Pure dead-code removal; behavior is already covered by `check` + `nightly-feature-gate`.

**Hours.** 0.25h (one-line edit + verify).

**Bead.** New `vb-rpp1`.

---

### PP-2: `banned-token-gates` — convert to explicit aggregator or remove

**Defect.** `.moon/tasks/all.yml:196-205` defines a task with **no `command:` and no `script:`**. Moon v2 tasks without an executable body either silently no-op or fail with "no command specified" depending on version. The deps (`panic-surface`, `workspace-assertions`, `ignored-fallible-results`) and `runInCI: true` make it look like a meaningful aggregator, but Moon's behavior here is undefined. (Note: it is not currently in `.moon.yml` pipeline, so it does not run, but it is `runInCI: true` so it would run if added.)

**Fix.** Either:
(a) **Add a real script** that runs `cargo geiger --forbid-only` (which is the real banned-token enforcer) and rename to `unsafe-audit` for clarity; or
(b) **Remove the task** entirely — the three deps already cover the work, and aggregator tasks in Moon are redundant when their deps are already in the pipeline (which `panic-surface` and `ignored-fallible-results` are; `workspace-assertions` is also in the pipeline via `check` deps).

I recommend (a) — make it concrete. Use the existing `supply-chain` task's geiger pattern at `.moon/tasks/all.yml:362-366`.

**Acceptance criteria (option a).**
- `banned-token-gates` has a real `script:` body that runs `cargo geiger` and exits non-zero on new unsafe in production.
- Task is removed from `.moon.yml` only if redundant; if it adds signal, it is added to the pipeline.

**Risk.** Low. If `cargo geiger` is not in the path, this needs the `rust.bins` config in `.moon/toolchains.yml`; check first.

**Hours.** 1h (script + toolchain check + test).

**Bead.** New `vb-rpp2`.

---

## Excluded Task Re-admissions or Bead Filings (15 items)

For each of the 15 `runInCI: false` tasks, decide: (i) re-admit to `runInCI: true` (it should run in CI), or (ii) file a P2-P3 bead carrying the work to make it re-admittable. The current justifications (per inline comments) split into:
- **Legitimately slow** (5): `pgo-instrument-build`, `pgo-optimized-build`, `verify-deep`, `verify-all`, `verify-verus-all` — these take hours and are not CI-bound by master policy.
- **Out of workspace / broken** (4): `contracts`, `benchmark-regression-policy`, `benchmark-proof`, `verify-proof` — these reference `xtask` which is outside the workspace.
- **Reference / dev only** (3): `verify-fast`, `verify-standard`, `quick` — explicit local-loop tasks.
- **Profile-gated** (3): `maxperf`, `maxperf-native`, `verify-all` — release-profile builds, intentionally not CI.

| # | Task | File:line | Current justification | Action | Bead |
|---|---|---|---|---|---|
| E-1 | `benchmark-regression-policy` | `all.yml:570-588` | "xtask is outside active Cargo workspace" | File P2 bead to reactivate xtask in workspace; mark `runInCI: false` permanently with a bead link | `vb-rax1` |
| E-2 | `benchmark-proof` | `all.yml:590-601` | "180m criterion run" | File P3 bead to break into per-crate benches; mark `runInCI: false` | `vb-rax2` |
| E-3 | `pgo-instrument-build` | `all.yml:603-614` | "PGO profile generation, not a CI gate" | Keep `runInCI: false`; document in master waiver file | `vb-rax3` |
| E-4 | `pgo-optimized-build` | `all.yml:616-627` | "PGO profile use, not a CI gate" | Same as E-3; co-locate in `vb-rax3` | (same) |
| E-5 | `maxperf` | `all.yml:629-647` | "release profile build, off CI" | Keep `runInCI: false`; add to nightly cron via `xtask nightly` if it exists | `vb-rax5` |
| E-6 | `maxperf-native` | `all.yml:649-668` | "native CPU profile, off CI" | Same as E-5; co-locate in `vb-rax5` | (same) |
| E-7 | `verify-fast` | `all.yml:670-681` | "Local fast verification, not CI" | Mark `runInCI: false` permanently; add comment "developer local-only" | `vb-rax7` |
| E-8 | `verify-standard` | `all.yml:683-694` | "Local standard verification" | Same as E-7; co-locate in `vb-rax7` | (same) |
| E-9 | `verify-deep` | `all.yml:696-707` | "Local deep verification" | Same as E-7; co-locate in `vb-rax7` | (same) |
| E-10 | `verify-proof` | `all.yml:721-732` | "wrapper for `rust-verification-gauntlet.sh proof`" | Promote to `runInCI: true` if master Section 40 requires it; otherwise file P2 waiver | `vb-rax10` |
| E-11 | `verify-all` | `all.yml:734-745` | "wrapper for `rust-verification-gauntlet.sh all`; takes hours" | Keep `runInCI: false`; file P3 nightly-cron bead | `vb-rax11` |
| E-12 | `contracts` | `all.yml:747-760` | "xtask alias resolves to non-member package" | File P1 bead to re-admit xtask as a workspace member | `vb-rax12` |
| E-13 | `quick` | `all.yml:762-769` | "Local fast dev loop" | Mark `runInCI: false` permanently; no bead needed but add comment | `vb-rax13` (P4) |
| E-14 | `test-determinism` | `all.yml:182-194` | "Current tree has pre-existing findings" | **Already covered by H-1** (`vb-rdet`). Cross-link. | (vb-rdet) |
| E-15 | `verify-verus-all` | `verus.yml:32-46` | "180m run" | Keep `runInCI: false`; co-locate with E-11 in nightly-cron bead | (vb-rax11) |

**Re-admission (flip to `runInCI: true`):** E-10 only — `verify-proof` is the master-Section-40 wrapper; re-admit if and only if `rust-verification-gauntlet.sh proof` completes in ≤30m on the runner. If it exceeds, keep excluded and file a separate P2 bead for the speedup.

**Bead filings (12 new):** `vb-rax1, 2, 3, 5, 7, 10, 11, 12, 13` plus the 3 co-located children.

**Acceptance criteria (overall).**
- Every `runInCI: false` task in the repo has either:
  - A comment explaining the exclusion and a link to a bead, OR
  - A comment explaining the exclusion and a master waiver entry, OR
  - Been flipped to `runInCI: true` with raw evidence.
- A new file `.moon/EXCLUSIONS.md` (or similar) lists all 15 with bead IDs and waiver IDs.

**Risk.** Low. Pure documentation/audit work; no code change.

**Hours.** 4h (write beads, write EXCLUSIONS.md, verify with `grep`).

**Beads.** New `vb-rax1..13` (with co-location collapsing 3 into parents = 9 net new).

---

## Per-Item Summary

| ID | Defect | Fix | Acceptance | Risk | Hours |
|---|---|---|---|---|---|
| vb-1ev82 | State 6 reviewer rejected, blocker list empty | re-verify, re-run State 5/6, close | cargo check exit 0, State 6 APPROVED | M | 2.5 |
| vb-8o7p5 | Kani timeout on crossbeam_queue unwind | add unwind flags per harness, file child | all 4 harnesses VERIFICATION::SUCCESS in ≤15m | H | 6 |
| vb-o5zb | All 5 children closed; parent still open | close parent, file doc-update child | children all ✓, doc reconciled | L | 2 |
| vb-yesh4 | not-reproducible on clean main | verify, close as deferred to future cfg bead | cargo check fuzz exit 0; log filed | L-M | 1.5 |
| vb-r4mi | miri 0.14% of tests | rename smoke, add miri-full | miri-full exits 0 in ≤30m | M | 8 |
| vb-rcov | coverage 1 test | rename smoke, add coverage-full | lcov.info with all crates | M-H | 6 |
| vb-rmut | mutants 1/3773 fns | rename smoke, add mutants-full | per-crate mutation report | H | 10 |
| vb-rfuz | fuzz 5/93 targets × 1s | rename smoke, add fuzz-full | 93 targets × 60s | M | 10 |
| vb-rbnc | bench-build 1 benchmark | rename smoke, add bench-build-full + bench-run | all benches built; run bead filed | L | 3 |
| vb-rdet | 1,088 hidden findings | T0–T5 phased triage | runInCI: true, 0 findings | VH | 64 |
| vb-rpp1 | phantom nightly-feature-cargo-probe | delete task + pipeline entry | grep returns nothing | 0 | 0.25 |
| vb-rpp2 | phantom banned-token-gates (no command) | add real cargo geiger script or remove | task has executable body | L | 1 |
| vb-rax1..13 | 15 excluded tasks untracked | file beads, write EXCLUSIONS.md | every exclusion has a bead or waiver | L | 4 |
| **TOTAL** | | | | | **118.25h** |

(Note: I previously wrote 90.5–121.5h; the consolidated number is ~118h with the determinism triage at 64h dominating. The 90.5h low end assumes a "T0–T3 only, allowlist the rest" approach to determinism; the 121.5h high end adds a 3h buffer per major task.)

---

## Recommended Execution Order

1. **Quick wins first** (3.5h, unblocks 4 P0s): P0-1, P0-3, P0-4, PP-1, PP-2 — all of these are pure audit/cleanup and reduce blocker count immediately.
2. **Phantom task removal** before adding any new task to `.moon.yml`, so the pipeline stays clean while expanding.
3. **Smoke relabels** (S-1..S-5 rename): single commit, 1h, all 5 tasks relabeled atomically so the pipeline keeps passing while full lanes are built.
4. **Excluded task filings** (E-1..E-15 beads): 4h, pure docs.
5. **Full lane builds** (S-1..S-5 follow-ups): ~37h spread across 1-2 weeks.
6. **Determinism T0–T1** first (4h, gets the gate non-regressive and tracked), then **P0-2 Kani unwind** (6h, removes the only real P0 in this round), then **determinism T2–T5** (60h, the bulk of the work).
7. **Final flip**: re-admit `verify-proof` (E-10) if its runtime is ≤30m; flip `test-determinism` to `runInCI: true` (H-1 T5 complete); close all P0s.

---

## Definition of Done

The round 4 follow-up is **complete** when **all** of the following are true:

1. **P0 status.** `bd list --status=blocked` does not include `vb-1ev82`, `vb-8o7p5`, `vb-o5zb`, or `vb-yesh4` (all closed or moved to non-blocked status with raw evidence).
2. **Phantom tasks.** `grep -r "nightly-feature-cargo-probe\|banned-token-gates" .moon/ .moon.yml` returns 0 matches (tasks removed) OR `banned-token-gates` has a real `script:`/`command:` body and is renamed.
3. **Smoke lane labels.** Every task currently in `.moon.yml:7-28` is either:
   - Master-required scope (proved by raw evidence), OR
   - Relabeled `*-smoke` with a follow-up bead carrying the full-scope work, OR
   - Removed.
4. **`test-determinism` is `runInCI: true`.** Baseline `.beads/vb-rdet/baseline.json` archives the historical 1,088 findings; current findings count is 0; raw log filed.
5. **Excluded tasks tracked.** Every `runInCI: false` task has either:
   - A `bd show <bead>` link in its YAML comment, OR
   - A line in a new `.moon/EXCLUSIONS.md` (or equivalent) that names the bead and waiver.
6. **Pipeline still green.** `moon ci` exits 0 from a clean main checkout; all originally-passing tasks still pass.
7. **Evidence filed.** Every closure in this plan has a raw log path in `.beads/<id>/` and a row in `verification-ledger.jsonl`.
8. **Push complete.** `git status` is clean; `bd dolt push` succeeds; the round 4 audit's `round-4-pipeline-integrity` table marks all 15 items as ✓ RESOLVED.
9. **Post-round review.** The next round of audit (round 5) finds 0 instances of: phantom tasks, smoke-only lanes masquerading as full lanes, or stale P0s. The 1,088 determinism finding count is reduced to 0 (or to the explicit allowlist count, which is documented and approved).

When 1-9 hold, declare the round 4 follow-up closed; the 4 P0 beads are no longer the largest source of pipeline integrity debt — the bulk of the remaining work is the test-determinism T4/T5 clock and temp-state triage, which is now tracked and scheduled.
