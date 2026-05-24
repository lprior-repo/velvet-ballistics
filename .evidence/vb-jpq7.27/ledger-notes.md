# vb-jpq7.27 proof obligation ledger rebuild

Canonical ledger: `.evidence/vb-jpq7.27/proof-obligation-ledger.jsonl`.

Status vocabulary is intentionally strict: `PASS`, `FAIL`, `NON_EVIDENCE`, `BLOCKED`.

## Audit summary

- Root `verification-ledger.jsonl` is marked `NON_EVIDENCE`; it is stale summary metadata and lacks raw command fields.
- Root `proof-findings.jsonl` is marked `NON_EVIDENCE`; it documents prior defects but is not proof output.
- Repaired vb-jpq7.24 Verus artifact parses/proves with current Verus, but `verusfmt --check` fails. Child bead: `vb-rga1`.
- Repaired vb-jpq7.26 bounded TLC models pass for BudgetArithmetic, RetryFSM, and LifecycleJournal.
- Legacy `RecoveryReplayFull` did not finish under explicit timeout; no PASS row. Child bead: `vb-2tpu`.
- Kani discovery passes for `vb_core` only as discovery evidence, not proof execution. `vb_validate` discovery fails. Child bead: `vb-utvm`.

## Prior repaired bead references

- `vb-jpq7.24`: closed as downgraded mirror-model + proof-to-Rust bridge, not direct production-body Verus proof.
- `vb-jpq7.25`: closed for Kani discovery repairs, with known broken crates/orphan harnesses; this ledger re-ran scoped discovery and records current failure honestly.
- `vb-jpq7.26`: closed after bounded overflow TLA models and external review; this ledger re-ran the bounded model commands.

## Validation

Run from workspace root:

```bash
python3 .evidence/vb-jpq7.27/check-ledger.py
```

The checker fails closed if a row is missing required raw evidence fields, if a `PASS` row has a missing log or non-zero exit code, if a `PASS` row is marked `NON_EVIDENCE`, or if `FAIL`/`BLOCKED` rows lack child beads.
