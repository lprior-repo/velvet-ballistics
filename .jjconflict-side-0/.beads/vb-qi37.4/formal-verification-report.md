# Formal Verification Report

STATUS: APPROVED

## Inputs

- `proof-obligations.jsonl`: valid JSONL, 16 rows.
- `delivery-scope.jsonl`: valid JSONL.
- `baseline-report.md`: present.
- `tla-spec.md`: present.
- `contract-verification-review.md`: `STATUS: APPROVED`.

## Obligation Results

- All 16 obligations are accounted in `verification-ledger.jsonl`.
- Required proof/model obligations passed by exact TLC, Verus, and Moon proof commands.
- Required admission/storage/runtime realization obligations passed by targeted admission/storage tests, fuzz smoke, mutation smoke, Loom model tests, and Moon CI.

## Waivers

- No blocking waiver required for State 13.

## Residual Risk

- Plain `moon ci` cannot compute Git affected changes in this jj workspace due missing local Git `main`; `moon ci --stdin` was used and passed.
