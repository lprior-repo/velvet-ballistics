bead_id: vb-qi37.16.3
bead_title: cli/runtime: Implement durable retry transition
phase: state-1
updated_at: 2026-05-11T00:00:00Z

# Baseline Report

Baseline captured before any Wave 2 bead-local edits at shared parent commit `qwxtlxqq 5fb2d246`.

Command:

```bash
moon ci
```

Result: FAIL before edits.

Evidence from `/home/lewis/src/Velvet-ballistics-vb-qi37-16-2-go` at same parent commit:

```text
Loading changed files
Base revision: N/A
Head revision: HEAD
Affected by changes: all
Error: process::failed

× Process git failed: exit code 128
│ fatal: ambiguous argument 'main': unknown revision or path not in the working tree.
```

Classification for later comparison: DEFERRED_GLOBAL baseline/tooling failure unless a bead-local change introduces an additional failure in scoped files or dependencies.
