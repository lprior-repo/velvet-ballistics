# Proof Plan Review Input - vb-0253.1

## Review Request
Please review the proof strategy and obligations for vb-0253.1 (Wrap shard command queue boundary).

## Bead Context
- **Scope**: vb_runtime shard command queue
- **Risk**: Concurrency (queue access), Public API (shard config)
- **Discovery artifacts**: codebase-map.md, contract.md, delivery-scope.jsonl

## Proof Obligations Summary
| ID | Verifier | Risk | Required | Status |
|----|----------|------|----------|--------|
| PO-001 | Kani | high | Yes | planned |
| PO-002 | Verus | high | Yes | planned |

## Questions for Review
1. Is Kani appropriate for the queue capacity bounded checking?
2. Is Verus sufficient for invariant proofs?
3. Should any obligation be added, removed, or reclassified?

## Assumptions
- Queue is local to a shard, no cross-shard coordination
- Capacity is bounded at construction and never changes
- No temporal behavior requiring TLA+

## Reviewer Action
Write proof-review.md with STATUS: APPROVED or STATUS: REJECTED and specific findings.
