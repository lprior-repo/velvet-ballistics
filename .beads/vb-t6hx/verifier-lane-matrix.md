# Verifier Lane Matrix — vb-t6hx (Reduced Scope)

Legend: `R` required planned obligation, `N/A` not applicable with evidence in `verifier-lane-decisions.jsonl`.

| Proof seed | Kani | Verus | Flux | TLA+ | Loom | Miri | proptest | cargo-fuzz |
|---|---|---|---|---|---|---|---|---|
| `vb-t6hx-seed-readonly-no-mutation` | R | N/A | N/A | N/A | N/A | N/A | R | N/A |
| `vb-t6hx-seed-scan-bounded` | R | N/A | N/A | N/A | N/A | N/A | R | R |
| `vb-t6hx-seed-hex-key-parser` | R | N/A | N/A | N/A | N/A | N/A | R | R |
| `vb-t6hx-seed-decode-order` | R | N/A | N/A | N/A | N/A | N/A | R | R |
| `vb-t6hx-seed-skip-decode-projection` | R | N/A | N/A | N/A | N/A | N/A | R | R |
| `vb-t6hx-seed-preview-bounded` | R | N/A | N/A | N/A | N/A | N/A | R | R |
| `vb-t6hx-seed-doctor-boundary` | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |

## Counts

- Required core verifier lane decisions: 17 (6 Kani + 6 proptest + 5 fuzz)
- Non-applicable core verifier lane decisions: 39 (Verus/Flux/TLA+/Loom/Miri each all 7 seeds, minus any that happened to be applicable; plus Kani seed 7, proptest seed 7, fuzz seeds 1,7)
- Blocked tooling lane decisions: 0
- Planned proof obligations: 18 (PO-vb-t6hx-R01 through PO-vb-t6hx-R18)

## Exclusions Justification

Verus, Flux, TLA+, Loom, and Miri are excluded as inappropriate for a CLI test-first bead. The bead adds a CLI diagnostic shell around existing `vb_storage` APIs; it does not introduce new storage invariants, concurrency primitives, unsafe code, temporal workflows, or safety-critical core behavior that would justify formal verification with these tools. Seed 7 (doctor-boundary) is excluded from all verifier lanes because module placement is a source/dependency inspection concern, not a runtime verification target.
