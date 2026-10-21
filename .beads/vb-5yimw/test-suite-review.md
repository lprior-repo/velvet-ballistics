# Independent Test Suite Review: vb-9agmk Artifact Repairs

**STATUS: REJECTED**

## Findings

- **P1 — Journaled execution binding is not covered at the runtime boundary.** The acceptance contract says the exact supplied workflow parts and contracts are bound for both Strict and Journaled execution (C8 in `.beads/vb-9agmk/contract.md`). The new runtime tests build only a Strict runtime (`runtime_cases.rs:8-22`) and exercise altered IR and replacement contracts under that configuration (`runtime_cases.rs:32-150`). The separate Journaled-to-Strict test calls `admit_artifact_run`/`admit_run`, not `Runtime` submission or shard processing (`admission_cases.rs:100-137`). A regression that makes `execution_binding::validate` bypass verification for Journaled while retaining Strict validation would pass the new runtime tests. Add public runtime and processing tests configured with Journaled policy that accept a matching tuple and reject substituted IR/contracts before enqueue and before run-state allocation.

## Reviewed scope

- Candidate source: `57f307dac5dd59153f46f1d3e7b92ea73e2de35b` (`codex/vb-9agmk-artifact-repairs`).
- Reviewed the C1-C8 artifact contract, the candidate’s artifact evidence suite and support modules, and the public runtime execution-binding implementation.
- Confirmed concrete assertions for the exact ten ledger obligations and their per-obligation closures; forged closure and duplicate ledger rejection; non-empty capability requirements derived from a `Do` node with exact, missing, and excess grants; rehashed capability tampering; public Journaled artifact escalation rejection; and Strict runtime execution tuple substitutions.
- No candidate source files were modified.

## Focused execution

Command, from the isolated review worktree:

```text
cargo test -p velvet-ballistics-workspace-tests --test vb_9agmk_artifact_evidence -j 2 -- --test-threads=2
```

Result: **20 passed, 0 failed, 0 ignored, 0 filtered out** (exit 0). Raw stdout, stderr, and exit status are in `evidence/focused.stdout`, `evidence/focused.stderr`, and `evidence/focused.exit`; `evidence/commands.txt` records the exact source SHA and command.

The passing focused run does not close the Journaled runtime execution-binding gap above, so this suite is not approved for the full C8 contract.
