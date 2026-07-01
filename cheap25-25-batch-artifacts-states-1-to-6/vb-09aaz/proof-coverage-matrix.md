# Proof Coverage Matrix — vb-09aaz

bead_id: vb-09aaz
state: 4 (proof-planner)
maps contract clauses to proof obligations and verifier lanes

## Source artifacts

- contract.md (9 contract clauses C1..C9)
- proof-seeds.jsonl (8 proof seeds PS-001..PS-008)
- delivery-scope.jsonl (27 file entries)
- traceability-matrix.jsonl (9 requirements, C1..C9)

## Coverage Matrix

### Contract Clause C1 — Abort-on-Fallible-Step Invariant (Cross-Method)

| Source | Proof Obligation | Verifier Lane | Status |
| --- | --- | --- | --- |
| contract.md#C1; type-contracts.md#G8; hazard-analysis.md#H1, H2 | PO-09aaz-001 (verus) | verus (WEAK_EXTERN mirror update) | planned |
| contract.md#C1 | PO-09aaz-003 (proptest) | proptest | planned |
| contract.md#C1 | PO-09aaz-002 (regression test) | rust-local | planned |

### Contract Clause C2 — G8 Guard Precedence

| Source | Proof Obligation | Verifier Lane | Status |
| --- | --- | --- | --- |
| contract.md#C2; type-contracts.md#guard-precedence | PO-09aaz-001 (verus) | verus (Guard enum + lemma_guard_order_is_valid extended) | planned |

### Contract Clause C3 — Typed Error Propagation

| Source | Proof Obligation | Verifier Lane | Status |
| --- | --- | --- | --- |
| contract.md#C3; error-taxonomy.md | (no new obligation; reuse existing Err(KeyCapacity) variant) | type-system invariant | covered by PO-09aaz-001 (verus witness for KeyCapacity) |

### Contract Clause C4 — Post-Condition: Aborted State on G8 Err

| Source | Proof Obligation | Verifier Lane | Status |
| --- | --- | --- | --- |
| contract.md#C4; type-contracts.md#post-condition | PO-09aaz-001 (verus) | verus (assume_specification match arm for Err(KeyCapacity) with spec_state_preserved_except_aborted) | planned |
| contract.md#C4 | PO-09aaz-002 (regression test) | rust-local (assertions 1-4 of test plan) | planned |
| contract.md#C4 | PO-09aaz-004 (persistence integration) | persistence (events_for_run(run).is_empty() + no pending-action-index entries) | planned |

### Contract Clause C5 — No Partial Persistence (Master §49)

| Source | Proof Obligation | Verifier Lane | Status |
| --- | --- | --- | --- |
| contract.md#C5; hazard-analysis.md#H1; velvet-ballistics-MASTER.md:2521-2567 | PO-09aaz-004 (persistence integration) | persistence (end-to-end with real Fjall instance) | planned |

### Contract Clause C6 — Public API Stability

| Source | Proof Obligation | Verifier Lane | Status |
| --- | --- | --- | --- |
| contract.md#C6; type-contracts.md#api-stability | PO-09aaz-005 (api-surface-check) | rust-local (non-behavior-affecting) | planned |

### Contract Clause C7 — Verus Spec Extension (PS-008, PS-009)

| Source | Proof Obligation | Verifier Lane | Status |
| --- | --- | --- | --- |
| contract.md#C7; type-contracts.md#verus-spec-extension; boundary-map.md#verifier-boundary | PO-09aaz-001 (verus) | verus (WEAK_EXTERN mirror update for both PS-008 and PS-009 production mirrors) | planned |

### Contract Clause C8 — Test Coverage

| Source | Proof Obligation | Verifier Lane | Status |
| --- | --- | --- | --- |
| contract.md#C8; boundary-map.md#test-boundary; hazard-analysis.md#H9 | PO-09aaz-002 (regression test) | rust-local (mirrors t_putters_b.rs:177-209) | planned |
| contract.md#C8 | PO-09aaz-003 (proptest variant) | proptest (arbitrary ActionId/RunId/StepIdx) | planned |

### Contract Clause C9 — Doc-Comment Update

| Source | Proof Obligation | Verifier Lane | Status |
| --- | --- | --- | --- |
| contract.md#C9; type-contracts.md#C6-doc-comment-update | PO-09aaz-005 (documentation gate) | rust-local (non-behavior-affecting; documentation only) | planned |

## Production Mirror Coverage

| Production Site | Mirror File | Status |
| --- | --- | --- |
| `crates/vb_storage/src/batch/append_event.rs:42-121` (production exec body) | `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs:78-95` (DRIFT POLICY header at L5-14) | 🔄 regenerate |
| `crates/vb_storage/src/batch/append_event.rs:42-121` (production exec body) | `verification/verus/production_inner/vb_vzcuf_PS_009_production.rs:67-93` (DRIFT POLICY header at L5-32) | 🔄 regenerate |
| Production `append_event` extern surface | `verification/verus/extern_vb_vzcuf_PS_008.rs` | 🔄 update mirror binding |
| Production `append_event` extern surface | `verification/verus/extern_vb_vzcuf_PS_009.rs` | 🔄 update mirror binding |

## Verification Script Coverage

| Script | Triggered By | Status |
| --- | --- | --- |
| `bash scripts/verify-verus.sh` | PO-09aaz-001 (verus mirror update + spec extension) | planned |
| `bash scripts/check-verus-production-binding.sh` | PO-09aaz-001 (production-binding gate, AGENTS.md mandatory) | planned |
| `bash scripts/check-production-inner-drift.sh` | PO-09aaz-001 (drift-gate header, zero tolerance) | planned |
| `cargo test -p vb_storage batch_append_event_index_key_error_aborts_commit` | PO-09aaz-002 (regression test) | planned |
| `cargo test -p vb_storage proptest_vb_vzcuf_PS_004` (or `proptest_vb_hyog0_PS_010`) | PO-09aaz-003 (proptest) | planned |
| `cargo test -p vb_storage master_section_49_integration` | PO-09aaz-004 (persistence integration) | planned |

## Coverage Summary by Risk

| Risk Tag | Seeds | Obligations | Verifier Lanes |
| --- | --- | --- | --- |
| persistence / partial-write | PS-001, PS-006, PS-008 | PO-09aaz-001, PO-09aaz-004 | verus + persistence |
| rust-local / abort invariant | PS-001, PS-002 | PO-09aaz-001, PO-09aaz-002 | verus + rust-local |
| verifier-binding / spec drift | PS-003 | PO-09aaz-001 | verus (WEAK_EXTERN) |
| test coverage gap | PS-002, PS-005 | PO-09aaz-002, PO-09aaz-003 | rust-local + proptest |
| public-api stability | PS-004, PS-007 | PO-09aaz-005 | rust-local (non-behavior-affecting) |
| guard precedence (C2) | PS-008 | PO-09aaz-001 | verus |

## Total Obligation Count

| Verifier | Count |
| --- | --- |
| verus | 1 obligation (PO-09aaz-001), covers PS-001/003/008 |
| proptest | 1 obligation (PO-09aaz-003), covers PS-001/005 |
| rust-local | 2 obligations (PO-09aaz-002 regression, PO-09aaz-005 surface+doc), covers PS-002/004/007 |
| persistence | 1 obligation (PO-09aaz-004), covers PS-006 |
| **Total planned** | **5 obligations** (matches user constraint of 4-5) |

## Cross-References

- Production site: `crates/vb_storage/src/batch/append_event.rs:104-115` (G8 IndexKeyConstruction)
- Verus specs: `verification/verus/vb-vzcuf-PS-008.rs`, `verification/verus/vb-vzcuf-PS-009.rs`
- Production mirrors: `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs`, `_PS_009_production.rs`
- Reference pattern: `crates/vb_storage/src/batch/putters.rs:188-200` (28 occurrences)
- Reference test: `crates/vb_storage/src/batch/t_putters_b.rs:177-209`
- Master §49: `velvet-ballistics-MASTER.md:2521-2567` (Crash-Consistency Rule)