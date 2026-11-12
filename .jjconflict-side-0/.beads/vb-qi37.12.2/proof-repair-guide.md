# Proof Repair Guide - vb-qi37.12.2

STATUS: NO_REPAIR_REQUIRED

Proof review is approved for progression to test planning. No proof-writer repair is required for the narrowed-R5 evidence handoff.

Carry-forward constraints for downstream owners:

- Do not reintroduce `PO-SOURCE-PRESERVE-001`.
- Do not claim exact per-error source identity from unit `ResumeError::JournalAppendFailed`.
- Prove `PO-R5-DETERMINISTIC-FALLBACK-001` as deterministic typed fallback only.
- Prove `PO-R5-SOURCE-ONLY-WHEN-CARRIED-001` only through public source carriers/source chains or owner-approved explicit non-ambient APIs.
- Prove `PO-R5-NO-AMBIENT-SOURCE-001` with static/clippy evidence that rejects globals, thread locals, task locals, cached stale errors, and other ambient source side channels.
- Treat `PO-TLA-RESUME-WORKFLOW-001` as waived-by-plan under `WV-TLA-RESUME-WORKFLOW-001`; do not claim TLC PASS unless a model is later added and executed.
- Before final aggregation, pair the TLA waiver with compensating evidence from `PO-R2-NO-FALSE-RESUMED-001`, `PO-R3-RESTORE-RESUMABLE-001`, `PO-R5-DETERMINISTIC-FALLBACK-001`, `PO-R5-NO-AMBIENT-SOURCE-001`, `PO-R5-SOURCE-ONLY-WHEN-CARRIED-001`, and `PO-API-SEMCVER-001`.
