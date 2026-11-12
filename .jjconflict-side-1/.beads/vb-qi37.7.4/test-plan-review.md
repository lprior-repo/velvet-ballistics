# Test Plan Review: vb-qi37.7.4

STATUS: APPROVED

## VERDICT: APPROVED

The repaired plan closes the previous rejection. It now names concrete boundary scenarios, exact typed error assertions, property invariants, mutation targets, and Holzmann test-structure constraints. This is still only a plan review; implementation and suite gates remain untrusted until State 5+ executes them.

## Mode 1 — Plan Inquisition

### Contract Parity

- PASS: `contract.md:96-98` declares one public function, `validate_gate_08_accessor_path_segments`, and `test-plan.md:66-205` gives named BDD coverage for the Gate 8 API and its active aggregate/core parity surfaces.
- PASS: `contract.md:70-79` names `AccessorSlotOutOfRange` and `AccessorPathInvalid`; `test-plan.md:81-100`, `test-plan.md:113-118`, `test-plan.md:141-153`, `test-plan.md:155-168`, and `test-plan.md:183-194` require exact variant assertions with coordinates/values.

### Assertion Sharpness

- PASS: No planned `Then:` uses bare `is_ok()` or `is_err()`.
- PASS: Previous equivalence-tautology hole is closed. `test-plan.md:170-194` requires focused and aggregate results to assert concrete `Ok(())` or concrete `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })` per implementation; equality alone is explicitly forbidden at `test-plan.md:181` and `test-plan.md:194`.

### Trophy Allocation

- PASS: Planned named scenario count is 18+ for 1 public function at `test-plan.md:16-18`, exceeding the mandatory 5x density floor.
- PASS: Non-trivial input space has five proptest invariants at `test-plan.md:206-236` covering field bounds, overflow-safe above-bound construction, index sentinel classification, first invalid segment, and first invalid accessor/root precedence.
- PASS: No fuzz target is required because parser/deserializer/user-text behavior is out of scope at `contract.md:14-17`, and the plan explicitly justifies typed adversarial property coverage at `test-plan.md:238-242`.

### Boundary Completeness

- PASS: Field minimum, maximum valid, equal-bound invalid, above-bound invalid, zero-symbol underflow guard, index minimum, index maximum valid, index sentinel invalid, empty path, and empty accessor collection are explicitly named at `test-plan.md:66-132` and matrixed at `test-plan.md:296-321`.
- PASS: Previous root-boundary hole is closed. `test-plan.md:134-153` names maximum-valid root `slot_count - 1`, one-past root `root == slot_count`, and greater-than root rejection with exact `AccessorSlotOutOfRange` values.
- PASS: Previous overflow ambiguity is closed. `test-plan.md:88-93`, `test-plan.md:214-218`, `test-plan.md:188-190`, and `test-plan.md:341` require checked construction or a written non-overflow proof for every `symbols_count + 1` fixture.

### Mutation Survivability

- PASS: Required field-bound mutations are mapped to killing tests at `test-plan.md:276-283`.
- PASS: Previous root off-by-one mutation holes are closed at `test-plan.md:284-286`, including `root < slot_count` changed to `root <= slot_count` and mutations that reject `slot_count - 1`.
- PASS: Duplicate implementation drift and equality-only false positives are killed by concrete per-implementation parity oracles at `test-plan.md:288-290`.
- PASS: Empty-structure, wrapping-arithmetic, wrong-coordinate, wrong-variant, and root-before-path mutations are explicitly targeted at `test-plan.md:279-293`.

### Holzmann Plan Audit

- PASS: Previous Rule 2 hole is closed. `test-plan.md:323-330` forbids loops in test bodies, mandates `rstest` or separate named tests for multi-case checks, and forbids shared mutable fixtures/global mutable state.
- PASS: Rule 5 preconditions are explicit in each BDD Given block at `test-plan.md:66-205`.
- PASS: Rule 8 side-effect visibility is addressed at `test-plan.md:327`; fixture helpers must disclose effects.

## LETHAL FINDINGS

None.

## MAJOR FINDINGS (0)

None.

## MINOR FINDINGS (0/5 threshold)

None.

## PRIOR REJECTION VERIFICATION

1. Root equal-bound rejection added: `test-plan.md:141-147`.
2. Root max-valid acceptance added: `test-plan.md:134-139`.
3. Focused/aggregate parity now asserts concrete expected values, not only equality: `test-plan.md:170-194`.
4. Holzmann no-loop/no-shared-fixture rules added: `test-plan.md:323-330`.
5. Root off-by-one mutation targets added: `test-plan.md:284-286`.
6. Overflow-safe `symbols_count + 1` construction rules added: `test-plan.md:88-93`, `test-plan.md:214-218`, `test-plan.md:341`.

## MANDATE

Proceed to implementation/test-writing state. Do not treat this approval as suite approval: implementation must still produce the named tests, exact assertions, property/Kani coverage where applicable, mutation evidence, and canonical `moon ci` evidence before shipping.
