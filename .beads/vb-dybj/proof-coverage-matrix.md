# Proof Coverage Matrix - vb-dybj

| Requirement | Proof seed | Domain claim coverage | Planned obligations | Required lanes | Non-core gates |
|---|---|---|---|---|---|
| REQ-001 | vb-dybj-seed-001 | RunId selected values, zero, representative, u64::MAX, Postcard bytes, decode equality | PO-VB-DYBJ-001, PO-VB-DYBJ-002, PO-VB-DYBJ-003 | Verus, Kani, proptest | Scoped nextest bridge |
| REQ-009 | vb-dybj-seed-002 | WorkflowDigest exact 32-byte construction/accessor/Postcard roundtrip | PO-VB-DYBJ-004, PO-VB-DYBJ-005, PO-VB-DYBJ-006 | Verus, Flux RS, proptest | Scoped nextest bridge |
| REQ-002 | vb-dybj-seed-003 | RecordKind Postcard enum bytes and envelope ID naming are distinct | PO-VB-DYBJ-007, PO-VB-DYBJ-008, PO-VB-DYBJ-009 | Verus, Kani, proptest | Mutation-sensitive fixture naming in bridge |
| REQ-004 | vb-dybj-seed-004 | Missing storage bytes return `JournalError::UnexpectedEof` before payload decode | PO-VB-DYBJ-010, PO-VB-DYBJ-011, PO-VB-DYBJ-012 | Kani, proptest, cargo-fuzz | Scoped storage decode tests |
| REQ-003 | vb-dybj-seed-005 | Trailing bytes reject exact decode and map to selected typed surface | PO-VB-DYBJ-013, PO-VB-DYBJ-014, PO-VB-DYBJ-015 | Kani, proptest, cargo-fuzz | Scoped raw/storage decode tests |
| REQ-008 | vb-dybj-seed-006 | No JSON wrapper, Bilrost, Protobuf, HTTP, YAML runtime path in touched compatibility path | PO-VB-DYBJ-018 | Core lanes not applicable with evidence | Source/dependency scan, moon ci |
| REQ-007 | vb-dybj-seed-007 | Golden byte change requires named migration, not silent fixture regeneration | PO-VB-DYBJ-016, PO-VB-DYBJ-017 | TLA+, proptest | Assertion names/messages, mutation-sensitive tests |

No proof seed is uncovered. Every core verifier lane has a decision row in `verifier-lane-decisions.jsonl`.
