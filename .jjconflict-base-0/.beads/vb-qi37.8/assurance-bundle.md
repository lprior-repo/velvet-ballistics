# Assurance Bundle: vb-qi37.8

**bead_id**: vb-qi37.8
**compiled**: 2026-05-17
**scope**: current-tree proof repair evidence only.

## Requirement-To-Evidence Map

| Requirement | Evidence | Status |
|-------------|----------|--------|
| Gate 8 accepts bounded valid accessors | `kani_gate_08_valid_bounded_parts_pass` | PASS |
| Gate 8 accepts zero accessors | `kani_gate_08_valid_zero_accessors_pass` | PASS |
| Gate 8 accepts index-only accessors with zero symbols | `kani_gate_08_valid_index_without_symbols_pass` | PASS |
| Gate 8 handles bounded arbitrary inputs without panic | `kani_gate_08_no_panic_bounded_inputs` | PASS |
| Gate 8 rejects field symbols outside `symbols_count` | `kani_gate_08_field_symbol_oob_rejected` | PASS |
| Gate 8 rejects `u32::MAX` index sentinel | `kani_gate_08_index_u32_max_rejected` | PASS |
| Gate 8 rejects root slots outside `slot_count` | `kani_gate_08_root_oob_rejected` | PASS |
| StepState runtime predicate matches proof kernel | Kani + Verus evidence | PASS |
| BudgetArithmetic bounded model has no TLC error | TLC evidence | PASS |

## Evidence Ledger

The authoritative machine-readable ledger is `.beads/vb-qi37.8/verification-ledger.jsonl`.

| Area | Status | Raw Evidence |
|------|--------|--------------|
| Gate 8 Kani | PASS | `/home/lewis/.local/share/opencode/tool-output/tool_e34ef1482001qcOlXtLV6oho6J` and sibling Gate 8 raw paths in ledger |
| StepState Kani | PASS | `/home/lewis/.local/share/opencode/tool-output/tool_e34fbcc37001x2PAzA97hgWznY` |
| StepState Verus | PASS | `.beads/vb-qi37.8/evidence/verus-step-state-machine.out` |
| BudgetArithmetic TLC | PASS | `.beads/vb-qi37.8/evidence/tlc-budget-arithmetic.out` |

## Deferred Scope

| Obligation | Status | Reason |
|------------|--------|--------|
| `PO-004` Gate 8 Miri | `DEFERRED_GLOBAL` | Not rerun in this proof repair. |
| `PO-030` full pipeline composition | `DEFERRED_GLOBAL` | Gate 8 Kani evidence is not full-pipeline composition evidence. |
| Gate 8 Verus | `DEFERRED_GLOBAL` | No Gate 8 Verus proof was run or claimed. |

## Review Chain

| Review | Verdict | Date |
|--------|---------|------|
| formal-verifier | APPROVED | 2026-05-17 |
| black-hat-reviewer | APPROVED | 2026-05-17 |
| truth-serum | APPROVED | 2026-05-17 |

## Assurance Statement

This bundle is sufficient for the scoped proof repair. It deliberately does not assert that all historical validation-pipeline requirements or acceptance criteria were refreshed in this session.
