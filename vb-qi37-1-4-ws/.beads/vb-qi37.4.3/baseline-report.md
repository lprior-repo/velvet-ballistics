bead_id: vb-qi37.4.3
bead_title: runtime/storage: Persist run header before acknowledgement
phase: State 1 - baseline
updated_at: 2026-05-11T00:00:00Z

# Baseline Report

Command: `moon ci`
Workdir: `/home/lewis/src/Velvet-ballistics-vb-qi37-4-3-go`
Exit: non-zero

```text
Loading changed files
Base revision: N/A
Head revision: HEAD
Affected by changes: all
Error: process::failed

Process git failed: exit code 128
fatal: ambiguous argument 'main': unknown revision or path not in the working tree.
Use '--' to separate paths from revisions, like this:
'git <command> [<revision>...] -- [<file>...]'
```

Classification: `DEFERRED_GLOBAL`
Follow-up text: Fix Moon/JJ workspace baseline configuration so `moon ci` does not require an absent Git ref named `main` in isolated JJ workspaces.
