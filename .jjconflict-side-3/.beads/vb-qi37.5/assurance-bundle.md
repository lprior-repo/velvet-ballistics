# State 13 Assurance Bundle

STATUS: APPROVED

Claims:
- Compile and validate idempotency decisions agree for all 45 combinations.
- Side-effecting DeterministicPure is rejected by compile and validation gates.
- Runtime idempotency key checks remain Kani-covered.
- Duplicate/stale replay behavior remains TLA-covered.

Raw evidence is listed in `machine-gate-report.md` and `verification-ledger.jsonl`.
