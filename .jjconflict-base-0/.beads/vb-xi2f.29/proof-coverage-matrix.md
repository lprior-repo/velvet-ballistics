# Proof Coverage Matrix: vb-xi2f.29

## Requirements → Obligations Mapping

| Requirement | Contract Clause | Proof Seeds | Obligations | Status |
|---|---|---|---|---|
| REQ-xi2f29-001 | C-01 | PS-001, PS-010 | PO-001 (kani), PO-008 (kani), PO-015 (unit) | planned |
| REQ-xi2f29-002 | C-02 | PS-002 | PO-002 (proptest), PO-010 (kani), PO-014 (unit) | planned |
| REQ-xi2f29-003 | C-03 | PS-003 | PO-003 (proptest), PO-014 (unit) | planned |
| REQ-xi2f29-004 | C-04 | PS-004, PS-009, PS-012 | PO-004 (proptest), PO-009 (kani), PO-010 (kani), PO-012 (unit), PO-014 (unit) | planned |
| REQ-xi2f29-005 | C-05 | PS-005 | PO-005 (proptest) | planned |
| REQ-xi2f29-006 | C-06 | PS-006, PS-011 | PO-006 (proptest), PO-011 (unit), PO-013 (unit) | planned |
| REQ-xi2f29-007 | C-07 | PS-007 | PO-007 (proptest) | planned |
| REQ-xi2f29-008 | C-08 | PS-008 | PO-001 (kani) | planned |
| REQ-xi2f29-009 | C-04 | PS-009 | PO-009 (kani) | planned |
| REQ-xi2f29-010 | C-01 | PS-010 | PO-008 (kani) | planned |
| REQ-xi2f29-011 | C-06 | PS-011 | PO-011 (unit) | planned |
| REQ-xi2f29-012 | C-04 | PS-012 | PO-012 (unit) | planned |
| REQ-xi2f29-013 | POST-006 | — | PO-009 (kani) — no-panic property | planned |
| REQ-xi2f29-019 | C-07 | PS-006, PS-007 | PO-007 (proptest) — regression gate | planned |
| REQ-xi2f29-021 | ALL | — | Dead code in compile/mod.rs — out of scope | monitored |

## Coverage by Verifier Lane

| Verifier | Obligations | Requirements Covered |
|---|---|---|
| kani | PO-001, PO-008, PO-009, PO-010 | C-01, C-04, C-06, C-08 |
| proptest | PO-002, PO-003, PO-004, PO-005, PO-006, PO-007 | C-02, C-03, C-04, C-05, C-06, C-07 |
| unit | PO-011, PO-012, PO-013, PO-014, PO-015 | C-01, C-02, C-03, C-04, C-06 |

## Coverage by Source File

| Source File | Obligations | Risk |
|---|---|---|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:105` | PO-001, PO-008, PO-015 | CANONICAL_NAME_BUG |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140-162` (new Together arm) | PO-002, PO-003, PO-004, PO-005, PO-006, PO-010, PO-014 | DIGEST_INSENSITIVITY |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (new digest_sub_step) | PO-004, PO-009, PO-012 | NESTED_STEP_BLINDNESS, RECURSION |
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116-138` (canonical_digest) | PO-006, PO-013 | REGRESSION |
| `crates/vb_compile/src/kani_canonical_name.rs:42-62` | PO-001 | KANI_PROOF (existing, regression) |
| `crates/vb_compile/src/kani_canonical_name.rs:121-175` | PO-008 | EXHAUSTIVENESS (existing) |
| `crates/vb_compile/tests/v1_primitive_lowering.rs:828` | PO-007 | REGRESSION (existing, must pass) |

## Coverage Gaps (Explicitly Out of Scope)

| Gap | Reason |
|---|---|
| `for_each` nested-step blindness | Future bead; same root cause as together |
| `collect` nested-step blindness | Future bead |
| `aggregate` nested-step blindness + canonical name | Future bead |
| `repeat` nested-step blindness | Future bead |
| `compile/mod.rs` dead code | Separate cleanup bead |
| `StepAst.name` field in digest | Non-goal per contract |
| `StepAst.condition` field in digest | Non-goal per contract |
| `StepAst.with` field in digest | Non-goal per contract |
| `StepAst.retry` field in digest | Non-goal per contract |
| `StepAst.on_error` field in digest | Non-goal per contract |
| `StepAst.then` field in digest | Non-goal per contract |
