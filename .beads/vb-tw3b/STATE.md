bead_id: vb-tw3b
bead_title: expr: Bytecode vs generated Rust parity evidence
phase: 13
updated_at: 2026-05-18T00:00:00Z
attempt: 1-of-7

# Go-skill state

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /tmp/opencode/go-skill-vb-tw3b-close

Path isolation evidence captured in-session:

```text
pwd -P => /tmp/opencode/go-skill-vb-tw3b-close
source_checkout => /home/lewis/src/velvet-ballistics
guard => isolated path is not equal to source checkout and is not nested under it
```

Current routing:

- Explicit bead ID: vb-tw3b; no bead swap.
- Red Queen: not invoked.
- State 1 initialized in isolated workspace.
- States 2-13 completed by evidence-only closure review: no production/test/proof code changes were needed because merged `vb_codegen` tests already cover this bead's parity scope.
- Landing allowed next: final-evidence-decision.md says `STATUS: APPROVED`.

Retry counters:

- State 1: attempt 1/7
- State 11: attempt 1/7 had local target/linker resource failure, rerun with isolated cache target passed focused gates.
