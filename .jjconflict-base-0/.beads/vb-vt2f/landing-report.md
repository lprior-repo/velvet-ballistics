# Landing Report - vb-vt2f

STATUS: APPROVED

## Scope

- Bead: `vb-vt2f` only.
- State: 14 landing only.
- Attempt: 2 of 7.
- Isolated workspace: `/home/lewis/src/bd-vb-vt2f-bdd`.
- Source checkout: `/home/lewis/src/velvet-ballistics` was not modified.
- No subagents, master agents, nested orchestrators, force-push, or hook skipping were used.

## Input Approval Verification

Command:

```text
python - <<'PY'
from pathlib import Path
import json
base = Path('.beads/vb-vt2f')
required = [
    'state13-provenance-waiver.md',
    'assurance-bundle.md',
    'truth-serum-report.md',
    'final-evidence-decision.md',
    'global-gate-repair-report.md',
    'black-hat-review.md',
    'formal-verification-report.md',
]
print('PWD=' + str(Path.cwd()))
for name in required:
    statuses = [(idx, line) for idx, line in enumerate((base / name).read_text().splitlines(), 1) if line.startswith('STATUS:')]
    print(f'{name}: {statuses}')
    if 'STATUS: APPROVED' not in [line for _, line in statuses]:
        raise SystemExit(f'missing approval: {name}')
rows = []
for idx, line in enumerate((base / 'verification-ledger.jsonl').read_text().splitlines(), 1):
    obj = json.loads(line)
    rows.append(obj)
bad = [obj.get('id', f'line-{idx}') for idx, obj in enumerate(rows, 1) if obj.get('result') in {'FAIL','FAIL_LOCAL','FAIL_REGRESSION','DEFERRED_GLOBAL'}]
print(f'verification-ledger.jsonl: valid_jsonl_rows={len(rows)} bad_results={bad}')
if len(rows) != 40 or bad:
    raise SystemExit('ledger not approved')
print('STATE13_AND_GLOBAL_REPAIR_APPROVALS=APPROVED')
PY
rc=$?; printf 'APPROVAL_VERIFY_EXIT=%s\n' "$rc"; exit "$rc"
```

Output:

```text
PWD=/home/lewis/src/bd-vb-vt2f-bdd
state13-provenance-waiver.md: [(3, 'STATUS: APPROVED')]
assurance-bundle.md: [(3, 'STATUS: APPROVED')]
truth-serum-report.md: [(3, 'STATUS: APPROVED')]
final-evidence-decision.md: [(3, 'STATUS: APPROVED')]
global-gate-repair-report.md: [(3, 'STATUS: APPROVED')]
black-hat-review.md: [(3, 'STATUS: APPROVED')]
formal-verification-report.md: [(3, 'STATUS: APPROVED')]
verification-ledger.jsonl: valid_jsonl_rows=40 bad_results=[]
STATE13_AND_GLOBAL_REPAIR_APPROVALS=APPROVED
APPROVAL_VERIFY_EXIT=0
```

## Landing Gate Evidence

Initial post-repair gate command:

```text
TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci; rc=$?; printf 'MOON_CI_EXIT=%s\n' "$rc"; exit "$rc"
```

Observed summary:

```text
Full output saved to: /home/lewis/.local/share/opencode/tool-output/tool_e3cf73de1001pk1nSRq0Hgdd56
Tasks: 20 completed (4 cached)
Time: 1m 57s 486ms
11087 tests run: 11087 passed (1 slow), 0 skipped
MOON_CI_EXIT=0
```

Post-rebase gate command:

```text
TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci; rc=$?; printf 'MOON_CI_REBASE_EXIT=%s\n' "$rc"; exit "$rc"
```

Observed summary:

```text
Full output saved to: /home/lewis/.local/share/opencode/tool-output/tool_e3cf97d3f001Nb7NOB0BmtTbpS
Tasks: 20 completed (3 cached)
Time: 2m 32s 874ms
11087 tests run: 11087 passed (1 slow), 0 skipped
MOON_CI_REBASE_EXIT=0
```

## Version-Control Evidence

Branch and repair commit commands:

```text
rtk git status --short --branch
rtk git add crates/vb_codegen/src/tests.rs crates/vb_storage/src/kani_recovery_hydrate.rs crates/vb_storage/src/recovery/recover.rs crates/vb_storage/src/recovery/recovery_unit_tests.rs .beads/vb-vt2f/dispatch-state14-global-gate-repair-attempt1.json .beads/vb-vt2f/dispatch-state14-landing-attempt2.json .beads/vb-vt2f/global-gate-repair-report.md && rtk git commit -m "chore(vb-vt2f): repair landing gate"
rtk git fetch origin main && rtk git rebase origin/main
```

Observed outputs:

```text
## land-vb-vt2f-attempt1
 M crates/vb_codegen/src/tests.rs
 M crates/vb_storage/src/kani_recovery_hydrate.rs
 M crates/vb_storage/src/recovery/recover.rs
 M crates/vb_storage/src/recovery/recovery_unit_tests.rs
?? .beads/vb-vt2f/dispatch-state14-global-gate-repair-attempt1.json
?? .beads/vb-vt2f/dispatch-state14-landing-attempt2.json
?? .beads/vb-vt2f/global-gate-repair-report.md
?? .beads/vb-vt2f/landing-report.md
?? .tmp/
ok 7 files changed, 179 insertions(+), 60 deletions(-)
ok land-vb
ok fetched (1 new refs)
Successfully rebased and updated refs/heads/land-vb-vt2f-attempt1.
```

Landing command:

```text
rtk git checkout main && rtk git pull --ff-only origin main && rtk git merge --ff-only land-vb-vt2f-attempt1 && rtk git push origin main && rtk git status --short --branch
```

Observed summary:

```text
Switched to branch 'main'
Your branch is behind 'origin/main' by 1 commit, and can be fast-forwarded.
ok 2 files +8 -8
Updating cd9eed50..411f20ba
Fast-forward
145 files changed, 6991 insertions(+), 86 deletions(-)
ok main
## main...origin/main
?? .beads/vb-vt2f/landing-report.md
?? .tmp/
```

Landed commits:

```text
rtk git log --oneline --decorate -6
411f20ba (HEAD -> main, origin/main, origin/HEAD, land-vb-vt2f-attempt1) chore(vb-vt2f): repair landing gate
200c4a06 fix(runtime): handle future policy variants
268c6724 test(runtime): add direct API BDD coverage
cd9eed50 chore(vb-vcmq): finalize landing evidence
f4c6f081 chore(vb-v7x6): record landing evidence
41161045 fix(doc): stabilize ui release gate
```

## Main/Remote Reachability Proof

Command:

```text
rtk git rev-parse HEAD origin/main && rtk git merge-base --is-ancestor HEAD origin/main; rc=$?; printf 'HEAD_ANCESTOR_ORIGIN_MAIN_EXIT=%s\n' "$rc"; exit "$rc"
```

Output:

```text
411f20bad9038e867da22d7ff92ccbbc45ff23a6
411f20bad9038e867da22d7ff92ccbbc45ff23a6
HEAD_ANCESTOR_ORIGIN_MAIN_EXIT=0
```

Decision: accepted work commit `411f20bad9038e867da22d7ff92ccbbc45ff23a6` is reachable from `origin/main`.

## Bead Close And Sync Evidence

Command:

```text
bd close vb-vt2f --reason "State 14 landing approved: moon ci passed after rebase and main pushed" && bd show vb-vt2f && bd dolt push
```

Observed output:

```text
✓ Closed vb-vt2f — bdd: Direct Rust API acceptance scenarios: State 14 landing approved: moon ci passed after rebase and main pushed
✓ vb-vt2f · bdd: Direct Rust API acceptance scenarios   [● P0 · CLOSED]
Close reason: State 14 landing approved: moon ci passed after rebase and main pushed
Pushing to Dolt remote...
Push complete.
```

Verification command:

```text
bd show vb-vt2f
```

Observed output excerpt:

```text
✓ vb-vt2f · bdd: Direct Rust API acceptance scenarios   [● P0 · CLOSED]
Close reason: State 14 landing approved: moon ci passed after rebase and main pushed
```

## Residual Blockers

- None for `vb-vt2f` State 14 landing.
- `.tmp/` remains untracked local runtime output and was not committed or pushed.
