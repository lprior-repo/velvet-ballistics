# Proof Coverage Matrix: vb-b8i8f

## Requirement-to-Obligation Traceability

| Requirement | Contract Clause | Proof Seed | Verus | Kani | Flux-rs | proptest | cargo-fuzz |
|-------------|----------------|------------|-------|------|---------|----------|------------|
| REQ-cancel-kill-live-only | C1, C2 | vb-b8i8f-seed-001 | PO-VERUS-001 | PO-KANI-001 | PO-FLUX-001 | PO-PROP-001 | — |
| REQ-single-terminal-winner | C3 | vb-b8i8f-seed-002 | PO-VERUS-002 | PO-KANI-002 | PO-FLUX-002 | PO-PROP-002 | — |
| REQ-stale-authority-cleanup | C4 | vb-b8i8f-seed-003 | PO-VERUS-003 | PO-KANI-003 | PO-FLUX-003 | PO-PROP-003 | — |
| REQ-runkilled-kind28-admission | C5 | vb-b8i8f-seed-004 | PO-VERUS-004 | PO-KANI-004 | PO-FLUX-004 | PO-PROP-004 | PO-FUZZ-001 |
| REQ-replay-ordinal-killed | C6 | vb-b8i8f-seed-005 | PO-VERUS-005 | PO-KANI-005 | PO-FLUX-005 | PO-PROP-005 | PO-FUZZ-002 |

## Coverage Legend
- **—**: Verifier is `not_applicable` for this seed (see lane decisions for evidence).
- **PO-XXXX-###**: Planned proof obligation ID; `planned` status, owner_state 5.

## Total Counts
- Proof Seeds: 5
- Verus obligations: 5 (PO-VERUS-001 through PO-VERUS-005)
- Kani obligations: 5 (PO-KANI-001 through PO-KANI-005)
- Flux-rs obligations: 5 (PO-FLUX-001 through PO-FLUX-005)
- proptest obligations: 5 (PO-PROP-001 through PO-PROP-005)
- cargo-fuzz obligations: 2 (PO-FUZZ-001, PO-FUZZ-002)
- **Total planned obligations: 22**
- Loom: 0 required (not_applicable: 5 seeds)
- Miri: 0 required (not_applicable: 5 seeds)
