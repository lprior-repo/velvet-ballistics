# Final Evidence Decision — vb-vt2f

STATUS: APPROVED

## Decision Basis

- Provenance waiver: `.beads/vb-vt2f/state13-provenance-waiver.md` is `STATUS: APPROVED` and explicitly discloses owner-authorized substitute packaging.
- Proof review: `.beads/vb-vt2f/proof-review.md:72-74` is `STATUS: APPROVED`.
- Contract verification review: `.beads/vb-vt2f/contract-verification-review.md:3,52-54` is `STATUS: APPROVED`.
- Test plan review: `.beads/vb-vt2f/test-plan-review.md:3,48` is `STATUS: APPROVED`.
- Test suite review: `.beads/vb-vt2f/test-suite-review.md:3,61` is `STATUS: APPROVED`.
- Test/public-surface review: `.beads/vb-vt2f/test-review.md:3-4` is `STATUS: APPROVED` and `PUBLIC_SURFACE_AUDIT: PASS`.
- Machine gate: `.beads/vb-vt2f/machine-gate-report.md:6,14-19` is `STATUS: PASS`, including focused trace-eviction, full direct API, catalog, runtime, and `moon ci` evidence.
- Formal verifier: `.beads/vb-vt2f/formal-verification-report.md:3,34-40,55-57` is `STATUS: APPROVED`, with `PASS: 35`, `WAIVED: 5`, `FAIL_LOCAL: 0`, `FAIL_REGRESSION: 0`, and `DEFERRED_GLOBAL: 0`.
- Black-hat review: `.beads/vb-vt2f/black-hat-review.md:3,11-13,36-38` is `STATUS: APPROVED` with no blocking findings.

## Ledger Counts

- `verification-ledger.jsonl` parsed successfully with `jq -c .`.
- PASS: 35.
- WAIVED: 5.
- FAIL: 0.
- DEFERRED_GLOBAL: 0.

## Final Statement

Approved for femdation State 13 substitute evidence packaging. Approval depends on the explicit provenance waiver and must not be represented as output from the unavailable `evidence-packaging` specialist.
