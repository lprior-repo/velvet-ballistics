bead_id: vb-qi37.15.2
phase: State 8

# Moon Report

Canonical command: `moon ci`
Result: non-zero, same as baseline.

Output:
```text
Loading changed files
Base revision: N/A
Head revision: HEAD
Affected by changes: all
Error: process::failed
  × Process git failed: exit code 128
  │ fatal: ambiguous argument 'main': unknown revision or path not in the working tree.
```

Bead-local command evidence:
```text
rtk cargo test -p velvet_ballastics --test cli_integration cli_submit -> 4 passed, 74 filtered out
```
