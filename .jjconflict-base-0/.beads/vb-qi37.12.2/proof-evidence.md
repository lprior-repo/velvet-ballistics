# Proof Evidence - vb-qi37.12.2

STATUS: EVIDENCE_ALIGNED_FOR_STATE6_RERUN

## Scope

State 5 re-aligned proof/evidence after State 4 changed `PO-TLA-RESUME-WORKFLOW-001` from an optional unwaived row into concrete planned waiver `WV-TLA-RESUME-WORKFLOW-001`. No production code or tests were changed. The obsolete source-identity framing is not carried forward because narrowed R5 rejects exact per-error source identity from unit `ResumeError::JournalAppendFailed`.

## Validation Evidence

Commands run from isolated workspace `/home/lewis/src/vb-qi37-12-2`:

- `python - <<'PY' ... PY`: NOT_RUN. The shell failed before JSON parsing with `zsh:1: write failed: disk quota exceeded` while creating here-doc content.
- `python -c "..."`: PASS.

Observed validation output from the passing one-liner:

```text
proof-obligations.jsonl: OK JSONL rows=9
proof-obligations.planned.jsonl: OK JSONL rows=9
traceability-matrix.jsonl: OK JSONL rows=8
formal-waivers.jsonl: OK JSONL rows=1
primary_ids=PO-R1-NO-DISCARD-001,PO-R2-NO-FALSE-RESUMED-001,PO-R3-RESTORE-RESUMABLE-001,PO-R4-NOT-RESUMABLE-SHAPE-001,PO-R5-DETERMINISTIC-FALLBACK-001,PO-R5-NO-AMBIENT-SOURCE-001,PO-R5-SOURCE-ONLY-WHEN-CARRIED-001,PO-API-SEMCVER-001,PO-TLA-RESUME-WORKFLOW-001
planned_ids=PO-R1-NO-DISCARD-001,PO-R2-NO-FALSE-RESUMED-001,PO-R3-RESTORE-RESUMABLE-001,PO-R4-NOT-RESUMABLE-SHAPE-001,PO-R5-DETERMINISTIC-FALLBACK-001,PO-R5-NO-AMBIENT-SOURCE-001,PO-R5-SOURCE-ONLY-WHEN-CARRIED-001,PO-API-SEMCVER-001,PO-TLA-RESUME-WORKFLOW-001
id_sets_match=True
removed_id_present=False
formal_waiver_required_keys=True
formal_waiver_id=WV-TLA-RESUME-WORKFLOW-001
formal_waiver_obligation_id=PO-TLA-RESUME-WORKFLOW-001
planned_tla_mode=waived-by-plan
planned_tla_required=False
planned_tla_waiver_id=WV-TLA-RESUME-WORKFLOW-001
waiver_ids_match=True
compensating_count=6
```

Artifact discovery found no `specs/vb_qi37_12_2_resume.*` files. State 5 did not introduce optional TLA artifacts. The planned TLA lane is therefore represented only by `formal-waivers.jsonl` and the matching waiver object in `proof-obligations.planned.jsonl`.

## Obligation Evidence Matrix

| ID | State 5 disposition | Next evidence owner |
| --- | --- | --- |
| `PO-R1-NO-DISCARD-001` | Planned; not executed by State 5. Must prove affected durable-write failures return typed errors, not silent success. | State 8 focused test evidence. |
| `PO-R2-NO-FALSE-RESUMED-001` | Planned; not executed by State 5. Must prove resume drive/append failures never return `Ok(Resumed)`. | State 8 focused test evidence. |
| `PO-R3-RESTORE-RESUMABLE-001` | Planned; not executed by State 5. Must prove failed `Resumed` append restores `RuntimeState::Resumable`. | State 8 focused test evidence. |
| `PO-R4-NOT-RESUMABLE-SHAPE-001` | Planned; not executed by State 5. Must prove `NotResumable` exposes `run_id` and `current_state`. | State 8 focused test/API shape evidence. |
| `PO-R5-DETERMINISTIC-FALLBACK-001` | Planned; not executed by State 5. Must prove deterministic unit `JournalAppendFailed` fallback when no public source carrier exists. No source-identity assertion is allowed for the unit variant. | State 8 focused test evidence. |
| `PO-R5-NO-AMBIENT-SOURCE-001` | Planned; not executed by State 5. Must prove no globals, task locals, thread locals, cached stale errors, or ambient side channels attach source detail to unit `JournalAppendFailed`. | State 10 clippy/static implementation review evidence. |
| `PO-R5-SOURCE-ONLY-WHEN-CARRIED-001` | Planned; not executed by State 5. Must prove exact source assertions exist only where a public carrier/source chain or approved explicit non-ambient API binds the source. | State 8 test/contract review evidence. |
| `PO-API-SEMCVER-001` | Planned; not executed by State 5. Must prove public unit `ResumeError::JournalAppendFailed` remains semver-compatible unless owner chooses a break. | State 10 API compatibility evidence. |
| `PO-TLA-RESUME-WORKFLOW-001` | Planned TLA waiver `WV-TLA-RESUME-WORKFLOW-001`; no TLA artifact exists or was changed. State 5 records no TLC PASS. | State 6 contract-verification can review the concrete waiver; State 11 aggregates only after compensating evidence exists. |

## Assumptions And Non-Claims

- Append failure injection represents failed durable write, not delayed observation failure.
- `handle_resume` evidence is scoped to a run whose authoritative state is read before the transition attempt.
- Unit `ResumeError::JournalAppendFailed` carries semantic failure class only; it is not a public source carrier.
- Source detail may be exposed only through public source chain, public error shape, or owner-approved explicit non-ambient API.
- State 5 does not claim any cargo/clippy/semver/TLC PASS result.
- State 5 does not resurrect `PO-SOURCE-PRESERVE-001` or any stale source-identity requirement for unit `JournalAppendFailed`.
- Older proof-evidence rows for `PO-SOURCE-PRESERVE-001` are superseded and are not current proof evidence after narrowed R5.

## Status

STATUS: EVIDENCE_ALIGNED_FOR_STATE6_RERUN

Next owner state: State 6 proof/contract review. State 6 can rerun because planned IDs match primary IDs, `formal-waivers.jsonl` validates, `WV-TLA-RESUME-WORKFLOW-001` is bound to `PO-TLA-RESUME-WORKFLOW-001`, and no current evidence claims source preservation for unit `JournalAppendFailed`.
