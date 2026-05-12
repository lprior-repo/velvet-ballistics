bead_id: vb-qi37.16.2
bead_title: cli/runtime: Implement durable resume transition
phase: state-1
updated_at: 2026-05-11T00:00:00Z

# Baseline Report

Baseline captured before any bead-local edits in isolated JJ workspace `/home/lewis/src/Velvet-ballistics-vb-qi37-16-2-go`.

Command:

```bash
moon ci
```

Result: FAIL before edits.

Evidence:

```text
Loading changed files
Base revision: N/A
Head revision: HEAD
Affected by changes: all
Error: process::failed

× Process git failed: exit code 128
│ fatal: ambiguous argument 'main': unknown revision or path not in the working tree.
│ Use '--' to separate paths from revisions, like this:
│ 'git <command> [<revision>...] -- [<file>...]'
```

Classification for later comparison: DEFERRED_GLOBAL baseline/tooling failure unless a bead-local change introduces an additional failure in scoped files or dependencies.
