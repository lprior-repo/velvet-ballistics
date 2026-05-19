bead_id: vb-0sps
phase: 1
updated_at: 2026-05-18T23:13:00Z
attempt: 1-of-7

# Baseline Report — vb-0sps

STATUS: BASELINE_CAPTURED_WITH_NO_ACTION_PIPELINE

## Workspace

- Source checkout: `/home/lewis/src/velvet-ballistics`
- Isolated workspace: `/home/lewis/src/bd-vb-0sps-bdd`
- Path guard: PASS; isolated workspace is not equal to and is not nested under source checkout.
- Bead status source: `/tmp/opencode/vb-0sps-bd-show.json` captured `status=in_progress`, `assignee=Lewis`.

## Baseline Commands

```text
pwd -P -> /home/lewis/src/bd-vb-0sps-bdd
bd show vb-0sps --json -> exit 0
TMPDIR=/tmp/opencode/vb-0sps-baseline-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci -> exit 0 with no affected tasks/action pipeline
```

## Classification

- Baseline is sufficient for State 1 isolation and initial resume.
- It is not final quality evidence. State 11 must run required scoped/canonical gates after implementation and compare failures to this baseline note.
