# Proof Coverage Matrix — vb-t6hx (Reduced Scope)

| Requirement | Proof seed | Domain claim | Required obligation IDs | Non-applicable core lanes |
|---|---|---|---|---|
| REQ-08 | `vb-t6hx-seed-readonly-no-mutation` | Doctor storage scan/get cannot mutate journal records or user keys. | `PO-vb-t6hx-R17`, `PO-vb-t6hx-R18` | Verus, Flux, TLA+, Loom, Miri: CLI test-first bead, no new invariants; cargo-fuzz: capability selection is not hostile byte input. |
| REQ-02 | `vb-t6hx-seed-scan-bounded` | Scan emits no more than `ScanLimit` rows and avoids unbounded collection. | `PO-vb-t6hx-R01`, `PO-vb-t6hx-R02`, `PO-vb-t6hx-R03` | Verus, Flux, TLA+, Loom, Miri: CLI test-first bead, single-invocation, no unsafe. |
| REQ-04 | `vb-t6hx-seed-hex-key-parser` | Invalid hex is rejected before storage open; valid keys become bytes. | `PO-vb-t6hx-R04`, `PO-vb-t6hx-R05`, `PO-vb-t6hx-R06` | Verus, Flux, TLA+, Loom, Miri: CLI test-first bead, pure parser boundary. |
| REQ-09 | `vb-t6hx-seed-decode-order` | Envelope length/integrity precedes Postcard decode. | `PO-vb-t6hx-R07`, `PO-vb-t6hx-R08`, `PO-vb-t6hx-R09`, `PO-vb-t6hx-R10` | Verus, Flux, TLA+, Loom, Miri: CLI test-first bead, existing storage codec spine. |
| REQ-01 | `vb-t6hx-seed-skip-decode-projection` | Projection scan lists previews without decoding malformed envelopes by default. | `PO-vb-t6hx-R14`, `PO-vb-t6hx-R15`, `PO-vb-t6hx-R16` | Verus, Flux, TLA+, Loom, Miri: CLI test-first bead, synchronous branch. |
| REQ-06 | `vb-t6hx-seed-preview-bounded` | Large values render bounded previews with truncation metadata and hint. | `PO-vb-t6hx-R11`, `PO-vb-t6hx-R12`, `PO-vb-t6hx-R13` | Verus, Flux, TLA+, Loom, Miri: CLI test-first bead, cold diagnostic output. |
| REQ-10 | `vb-t6hx-seed-doctor-boundary` | Doctor scanner types/formatting remain outside runtime core/hot paths. | (behavior tests only; seed 7 has no formal obligation) | Kani, Verus, Flux, TLA+, Loom, Miri, proptest, cargo-fuzz: module placement is source/dependency inspection evidence. |

## Active Verifier Layers

| Verifier | Required obligations | Seeds covered |
|---|---|---|
| Kani | 6 (R01, R04, R07, R11, R14, R17) | 1-6 |
| proptest | 6 (R02, R05, R08, R12, R15, R18) | 1-6 |
| cargo-fuzz | 5 (R03, R06, R09, R10, R13, R16) | 2-6 |
| Behavior tests (nextest) | Primary evidence channel (not obligation rows) | 1-7 (all) |

Total formal obligations: 17 (6 Kani + 6 proptest + 5 fuzz = 17). Wait, let me recount: R01-R18 = 18. Kani: R01, R04, R07, R11, R14, R17 = 6. proptest: R02, R05, R08, R12, R15, R18 = 6. fuzz: R03, R06, R09, R10, R13, R16 = 6. That's 18 total formal obligations.

## Traceability Notes

- Traceability rows with no proof seed (`REQ-05`, `REQ-07`, `REQ-12`) remain behavior-test or implementation contract coverage for downstream states; State 4 proof obligations are driven by the seven State 3 proof seeds.
- Seed 7 (`vb-t6hx-seed-doctor-boundary`) has no formal obligations; its coverage is source inspection + behavior tests, not verifier tools.
- Every `(requirement_id, contract_clause, proof_seed_id)` has eight core verifier lane decisions in `verifier-lane-decisions.jsonl`.
