# State

- Bead: `vb-qi37.4.4`
- Current state: State 14 final manual QA approved after State 12 formal repair.
- Highest completed state: 14.
- Next gate: landing/merge gate; do not push or touch root from this isolated workspace handoff.
- Retry class: `DEFERRED_GLOBAL` remains only for unrelated Moon CI red items; no bead-local formal blocker remains.
- Closed: no.

## Evidence
- State 9/10/11 downstream rerun were already approved after State 13 refactor per handoff.
- State 12 repair restored `API-ADM-001` in the approved obligation path `crates/velvet_ballastics/tests/admission_evidence_integration.rs`.
- `cargo test -p velvet_ballastics --test admission_evidence_integration api_envelope_preserves_admission_durability_code` PASS: 1 passed, 6 filtered out.
- Added bounded TLA model and config at `specs/admission_header_before_ack.tla` / `.cfg` for `TLA-ERR-001`.
- `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla` PASS: 8 states generated, 4 distinct states, diameter 2, no errors.
- `moon run :verify-proof` PASS: Kani found no proof harnesses; Lean skipped because no proof directory exists.
- `formal-verification-report.md` is `STATUS: APPROVED`.
- `verification-ledger.jsonl` counts: PASS=6, WAIVED=1, DEFERRED_GLOBAL=1, FAIL_LOCAL=0.
- State 14 final manual QA smoke: canonical binary `--help` and `--version` PASS; `manual-qa-final.md` is `STATUS: APPROVED`.

## Blockers
- None bead-local.
- Remaining non-blocking gate: `REL-GATE-004` is `DEFERRED_GLOBAL` for unrelated workspace debt previously classified outside this bead scope.
