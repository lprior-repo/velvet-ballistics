# Assurance Bundle — vb-qi37.12.2

STATUS: APPROVED

## Scope

- Bead: `vb-qi37.12.2`.
- Goal: propagate journal/storage resume failures without false success, restore resumable state after failed `Resumed` append, and avoid stale/ambient source theft.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-2`.

## Raw Evidence Consumed

- Contract: `.beads/vb-qi37.12.2/contract.md` is narrowed and traceable.
- Contract verification: `.beads/vb-qi37.12.2/contract-verification-review.md` has `STATUS: APPROVED`.
- Proof review: `.beads/vb-qi37.12.2/proof-review.md` has `STATUS: APPROVED`.
- Test reviews: `.beads/vb-qi37.12.2/test-plan-review.md` and `test-suite-review.md` have `STATUS: APPROVED`.
- Formal/machine gates: `.beads/vb-qi37.12.2/machine-gate-report.md`, `regression-diff.md`, `mutation-report.md`, `api-compat-report.md`, and `static-scan-report.md` are approved/pass artifacts.
- Black-hat review: `.beads/vb-qi37.12.2/black-hat-review.md` has `STATUS: APPROVED` after re-review.
- Waiver: `.beads/vb-qi37.12.2/formal-waivers.jsonl` validates and names `WV-TLA-RESUME-WORKFLOW-001` with compensating evidence.

## Current Session Commands

- `TMPDIR=target/tmp rtk cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation --all-features` -> 12 passed.
- `TMPDIR=target/tmp rtk cargo test -p vb_runtime --lib is_resumable` -> 2 passed, 1349 filtered.
- `jq -c .` over proof, planned proof, traceability, verification ledger, and waiver JSONL -> exit 0.
- `rg 'ResumeSourceRegistry|source_runtime_error|JournalAppendFailed|thread_local|SOURCE' crates/vb_runtime/src --glob '*.rs'` -> no ambient registry/thread-local source carrier found; only explicit source-carrying enum/conversion sites remain.

## Decision

State 13 packaging finds enough raw evidence for the narrowed contract. No State 12 blocker remains.
