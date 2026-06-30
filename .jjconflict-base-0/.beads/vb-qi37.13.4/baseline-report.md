bead_id: vb-qi37.13.4
bead_title: cli: Structured output contract tests
phase: State 1 baseline
updated_at: 2026-05-11T00:00:00Z

# Baseline Report

Command: `moon ci`
Workdir: `/home/lewis/src/Velvet-ballistics-vb-qi37-13-4-go`
Exit: non-zero before bead-local edits.

Output excerpt:

```text
Loading changed files
Base revision: N/A
Head revision: HEAD
Affected by changes: all
Error: process::failed

  × Process git failed: exit code 128
  │
  │ fatal: ambiguous argument 'main': unknown revision or path not in the
  │ working tree.
  │ Use '--' to separate paths from revisions, like this:
  │ 'git <command> [<revision>...] -- [<file>...]'
```

Classification: DEFERRED_GLOBAL candidate for later State 8 comparison if unchanged; this is a pre-edit Moon/Git baseline failure and not bead-local source behavior.
