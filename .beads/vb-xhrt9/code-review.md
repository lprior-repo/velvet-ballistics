# Code Review — vb-xhrt9 (independent DEEPSEEK reviewer)

**Verdict: APPROVE**

Scope: `.moon/tasks/all.yml` delta `1cf3453f9a36..c56f652099b4` — `supply-chain` and `benchmark-proof` tasks. No other production files changed (`git diff --name-only` shows only `.moon/tasks/all.yml` plus `.beads/*` evidence/docs). `crates/*/Cargo.toml` diff is empty.

## V1 rejection points (527f27a1) — all resolved

| # | V1 defect | Final state | Verdict |
|---|---|---|---|
| 1 | `run_advisory_report` used `cmd || true; rc=$?` → always 0 | `if "$@" >out 2>err; then rc=0; else rc=$?; fi` | Fixed |
| 2 | audit `if !cmd; then rc=$?` → 0; top-level `local`; `return` outside function | `if timeout ... cargo audit; then ... else rc=$?; ... exit "$rc"; fi`; no top-level `local`; only `return 0` is inside `run_advisory_report` | Fixed |
| 3 | geiger ran bare under `set -e`, aborted task on report failure | per-package `if ...; then geiger_rc=0; else geiger_rc=$?; fi`, loop continues | Fixed |
| 4 | new Cargo.toml bench decl + invented targets (`workspace_tests`) | Cargo.toml restored (empty diff); single real target `-p vb_core --bench aggregate_resource_budget` | Fixed |

## Correctness of final shell (verified by extraction, not just read)

- `set -euo pipefail` at task top. No `set +e`, no `|| true`, no `! cmd` negation at top level, no `return` outside a function (static grep + awk on extracted script → PASS).
- `run_advisory_report` writes exact exit code to `${tool}.status`, never relabels PASS, returns 0 so advisory tools continue. Exit 124 (timeout) and 127 (missing tool) flow through `$?` naturally.
- `cargo audit` is fail-closed: nonzero → `exit "$rc"` (writes `audit.status` first). This preserves the required CVE fail-closed property under the 2026-05-23 owner waiver (MASTER 114/1683/1721/4786 make deny/vet/geiger/machete advisory; audit stays blocking).
- Geiger per-package: 11 status files, each capturing real exit code, loop never aborts the task.

## benchmark-proof

- `script` runs `timeout 300 ... cargo bench --all-features -p vb_core --bench aggregate_resource_budget -- --save-baseline vb-current`.
- `options.runInCI: false` — consistent with removed/quarantined maxperf path (no perf bench in CI).
- Target is real: `crates/vb_core/benches/aggregate_resource_budget.rs` exists; `crates/vb_core/Cargo.toml` has pre-existing `[[bench]] name="aggregate_resource_budget" harness=false`. No new bench declaration introduced (Cargo.toml diff empty).
- `moon query tasks --id benchmark-proof` confirms `runInCI: false`, `unixShell: bash`, single-target script.

## Scoped actual verification (immutable `/tmp/vb-run-gate.py`)

- `.beads/vb-xhrt9/evidence/extract-real.stdout` — extracted the real `supply-chain` script body from `.moon/tasks/all.yml` and executed it: static no-mask/no-top-level-return PASS; real `run_advisory_report` captured 0/1/2/124/127 exactly; full script with fake tools (deny=1, vet=2, geiger=1, machete=1, audit=0) → task exit 0 with all status files exact (11 geiger statuses=1).
- `.beads/vb-xhrt9/evidence/extract-real-failclosed.stdout` (exit 0) — full script with audit exit 1 → task exit 1, `audit.status=1`; audit exit 3 → task exit 3. Fail-closed confirmed.
- `extract-real.json` records exit_code 1 for the FIRST harness run; that exit was my harness's own `set -e` aborting on the intentional fail-closed scenario, not a production defect (all advisory checks in that run printed PASS; fail-closed re-verified separately at exit 0).

## Non-blocking notes

- `rc` / `geiger_rc` are not declared `local` (leak to task scope). Harmless under `set -u` (always assigned before read); style only.
- Older `.beads/vb-xgj4v/handoff.md` still describes the reverted 3-target bench + Cargo.toml bench decl. `handoff-v2.md` supersedes; stale doc is cosmetic, not a code defect.

No production source edits required. No proof-completion or speed claims are made here.
