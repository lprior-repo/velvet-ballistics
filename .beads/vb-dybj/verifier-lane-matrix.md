# Verifier Lane Matrix - vb-dybj

Legend: R = required, NA = not applicable with evidence in `verifier-lane-decisions.jsonl`.

| Seed | Requirement | TLA+ | Verus | Kani | Flux RS | Loom | Miri | proptest | cargo-fuzz |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| vb-dybj-seed-001 | REQ-001 RunId fixtures/zero/max | NA | R | R | NA | NA | NA | R | NA |
| vb-dybj-seed-002 | REQ-009 WorkflowDigest 32 bytes | NA | R | NA | R | NA | NA | R | NA |
| vb-dybj-seed-003 | REQ-002 RecordKind surface distinction | NA | R | R | NA | NA | NA | R | NA |
| vb-dybj-seed-004 | REQ-004 missing bytes typed short error | NA | NA | R | NA | NA | NA | R | R |
| vb-dybj-seed-005 | REQ-003 trailing bytes typed decode error | NA | NA | R | NA | NA | NA | R | R |
| vb-dybj-seed-006 | REQ-008 no JSON/Bilrost/Protobuf | NA | NA | NA | NA | NA | NA | NA | NA |
| vb-dybj-seed-007 | REQ-007 named migration for golden changes | R | NA | NA | NA | NA | NA | R | NA |

Non-core planned lane: source/dependency scan for REQ-008.
