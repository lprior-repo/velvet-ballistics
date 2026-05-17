# Proof Repair Guide: vb-qi37.10

## Required Repair

1. Reconcile `.beads/vb-qi37.10/proof-obligations.jsonl` with `.beads/vb-qi37.10/proof-obligations.planned.jsonl`.
   - Do not leave `TLA-PARITY-001`, `SUPPORT-001`, or `VERUS-STORE-001` as `required:true`/`planned` if State 5 is intentionally deferring formal lanes.
   - Either produce production-bound TLA+/Kani/Verus artifacts plus raw verifier evidence, or explicitly defer/waive the original obligations with owner, expiry, compensating evidence, and follow-up tracking.

2. Replace placeholder waiver owner `future formal-proof bead` with concrete ownership.
   - Preferred: create or reference bead IDs for TLA+, Verus, and Kani follow-up work.
   - Acceptable alternative: record an approved scope decision naming the owner, expiry trigger, and compensating evidence required before any formal proof coverage may be claimed.

3. Preserve the good parts of the current packet.
   - Keep `PO-001` through `PO-012` as acceptance-critical executable lanes.
   - Keep the non-claim statements in `proof-evidence.md`.
   - Keep future-proof constraints: bounded TLA+ with typed Err/overflow states, production-bound Verus, and no hardcoded Kani shapes.

## Rerun Targets For Next Review

```bash
pwd -P
jq -c . ".beads/vb-qi37.10/proof-obligations.jsonl" >/dev/null
jq -c . ".beads/vb-qi37.10/proof-obligations.planned.jsonl" >/dev/null
jq -c 'select(.required==true and ((.status=="blocked") or (.status=="planned")) and (.checker|test("blocked")))' ".beads/vb-qi37.10/proof-obligations.jsonl"
jq -c 'select((.id=="PO-013") or (.id=="PO-014") or (.id=="PO-015")) | {id,required,status,waiver}' ".beads/vb-qi37.10/proof-obligations.planned.jsonl"
```

The third command should return no unresolved required blocked formal obligations unless raw verifier evidence exists and is cited.

STATUS: REJECTED
