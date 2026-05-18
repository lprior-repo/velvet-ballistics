# vb-kyyf State 14 Landing Report

STATUS: APPROVED

## Scope

- Bead: `vb-kyyf` only.
- State: 14 landing only.
- Attempt: 3 of 7.
- Isolated workspace: `/home/lewis/src/bd-vb-kyyf-bdd`.
- Source checkout `/home/lewis/src/velvet-ballistics` was not touched.
- Branch landed: `landing/vb-kyyf` to `main`.

## Approval Verification

State 13 and repair inputs were checked before landing:

```text
.beads/vb-kyyf/final-evidence-decision.md: STATUS: APPROVED
.beads/vb-kyyf/truth-serum-report.md: STATUS: APPROVED
.beads/vb-kyyf/assurance-bundle.md: STATUS: APPROVED
.beads/vb-kyyf/state13-provenance-waiver.md: STATUS: APPROVED
.beads/vb-kyyf/build-cache-cleanup-rerun-report.md: STATUS: APPROVED
.beads/vb-kyyf/vb-m5gp-global-repair-report.md: STATUS: APPROVED
```

The approved global repair changed only `crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs`, updating the documented expected generated digest for the accepted minimal workflow. The report states focused tests passed twice, `cargo fmt --check` passed, and `moon ci` passed.

## Exact Commands

```text
rtk git status --short --branch
rtk git log --oneline --decorate -8
read .beads/vb-kyyf/dispatch-state14-landing-attempt3.json
read .beads/vb-kyyf/final-evidence-decision.md
read .beads/vb-kyyf/truth-serum-report.md
read .beads/vb-kyyf/assurance-bundle.md
read .beads/vb-kyyf/state13-provenance-waiver.md
read .beads/vb-kyyf/build-cache-cleanup-rerun-report.md
read .beads/vb-kyyf/vb-m5gp-global-repair-report.md
rtk git diff -- crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs
rtk git add crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs .beads/vb-kyyf/build-cache-cleanup-rerun-report.md .beads/vb-kyyf/dispatch-state14-build-cache-cleanup-rerun-attempt1.json .beads/vb-kyyf/dispatch-state14-global-format-repair-attempt1.json .beads/vb-kyyf/dispatch-state14-landing-attempt2.json .beads/vb-kyyf/dispatch-state14-landing-attempt3.json .beads/vb-kyyf/dispatch-state14-mutants-tmpdir-repair-attempt1.json .beads/vb-kyyf/dispatch-state14-quota-cleanup-rerun-attempt1.json .beads/vb-kyyf/dispatch-state14-vb-m5gp-global-repair-attempt1.json .beads/vb-kyyf/global-format-repair-report.md .beads/vb-kyyf/mutants-tmpdir-repair-report.md .beads/vb-kyyf/quota-cleanup-rerun-report.md .beads/vb-kyyf/vb-m5gp-global-repair-report.md && rtk git commit -m "chore(vb-kyyf): record landing repairs"
rtk git fetch origin main && rtk git rebase origin/main
rtk ls -ld /tmp/opencode
rtk rm -rf /tmp/opencode/vb-kyyf-moon-ci-tmp && rtk mkdir -p /tmp/opencode/vb-kyyf-moon-ci-tmp && TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci
rtk git fetch origin main && rtk git rebase origin/main && rtk git checkout main && rtk git merge --ff-only landing/vb-kyyf && rtk git push origin main
rtk git status --short --branch
rtk git rev-parse --short HEAD && rtk git rev-parse --short origin/main && rtk git merge-base --is-ancestor HEAD origin/main && rtk git merge-base --is-ancestor origin/main HEAD
bd show vb-kyyf --json
bd close vb-kyyf --reason "Landed vb-kyyf to main at 644db87a after State 13 approval, approved global repair, and green moon ci" && bd show vb-kyyf --json && bd dolt push
bd show vb-kyyf --json | jq -r 'if type == "array" then .[0] else . end | "status=" + .status, "closed_at=" + (.closed_at // ""), "close_reason=" + (.close_reason // ""), "updated_at=" + .updated_at'
```

## Canonical Landing Gate

Command:

```text
rtk rm -rf /tmp/opencode/vb-kyyf-moon-ci-tmp && rtk mkdir -p /tmp/opencode/vb-kyyf-moon-ci-tmp && TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci
```

Result: PASS.

```text
velvet-ballastics:test | Summary [  13.814s] 11026 tests run: 11026 passed, 0 skipped
velvet-ballastics:mutants-smoke | 1 mutant tested: 1 caught
velvet-ballastics:source-length | DEFERRED_GLOBAL: crates/vb_compile/src/expression_bytecode.rs has 2242 physical lines (limit <300)
velvet-ballastics:source-length | DEFERRED_GLOBAL: crates/vb_compile/src/expression.rs has 881 physical lines (limit <300)
velvet-ballastics:source-length | DEFERRED_GLOBAL: crates/vb_compile/src/references.rs has 342 physical lines (limit <300)
velvet-ballastics:source-length | DEFERRED_GLOBAL: crates/vb_compile/src/schema.rs has 729 physical lines (limit <300)
velvet-ballastics:source-length | DEFERRED_GLOBAL: crates/vb_compile/src/type_taint.rs has 511 physical lines (limit <300)
Tasks: 21 completed (4 cached)
 Time: 40s 768ms
```

The `source-length` output remains a pre-existing `DEFERRED_GLOBAL` ratchet and did not fail `moon ci`.

## Main And Remote Reachability Proof

Landing push command:

```text
rtk git fetch origin main && rtk git rebase origin/main && rtk git checkout main && rtk git merge --ff-only landing/vb-kyyf && rtk git push origin main
```

Observed result:

```text
Current branch landing/vb-kyyf is up to date.
Switched to branch 'main'
Your branch is up to date with 'origin/main'.
Updating 47985868..644db87a
Fast-forward
ok main
```

Reachability command:

```text
rtk git rev-parse --short HEAD && rtk git rev-parse --short origin/main && rtk git merge-base --is-ancestor HEAD origin/main && rtk git merge-base --is-ancestor origin/main HEAD
```

Observed output and exit status: command exited 0.

```text
644db87a
644db87a
```

Status immediately after the push:

```text
## main...origin/main
?? .beads/vb-kyyf/landing-report.md
```

The only remaining local file at that point was this landing report artifact, written after the push/close evidence existed.

## Bead Close And Sync Evidence

Close/sync command:

```text
bd close vb-kyyf --reason "Landed vb-kyyf to main at 644db87a after State 13 approval, approved global repair, and green moon ci" && bd show vb-kyyf --json && bd dolt push
```

Observed close fields:

```text
"status": "closed"
"updated_at": "2026-05-18T22:20:38Z"
"closed_at": "2026-05-18T22:20:38Z"
"close_reason": "Landed vb-kyyf to main at 644db87a after State 13 approval, approved global repair, and green moon ci"
```

Observed Dolt sync result:

```text
Pushing to Dolt remote...
Push complete.
```

Concise verification command:

```text
bd show vb-kyyf --json | jq -r 'if type == "array" then .[0] else . end | "status=" + .status, "closed_at=" + (.closed_at // ""), "close_reason=" + (.close_reason // ""), "updated_at=" + .updated_at'
```

Observed output:

```text
status=closed
closed_at=2026-05-18T22:20:38Z
close_reason=Landed vb-kyyf to main at 644db87a after State 13 approval, approved global repair, and green moon ci
updated_at=2026-05-18T22:20:38Z
```

## Residual Blockers

None for `vb-kyyf` State 14 landing.

Known non-blocking global ratchet remains: `source-length` reports oversized `vb_compile` files as `DEFERRED_GLOBAL`, and `moon ci` still exits 0.
