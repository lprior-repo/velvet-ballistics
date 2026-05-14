# Test Plan: Admission durability errors

## Summary
- Behaviors identified: 6
- Trophy allocation: 4 unit / 2 integration / 0 e2e.
- Proptest invariants: 1
- Fuzz targets: 0; no parser boundary added.
- Kani harnesses: 0.

## 1. Behavior Inventory
- Runtime exposes exact admission artifact-not-found diagnostic.
- Runtime exposes exact invalid/stale/digest-mismatch diagnostic.
- Runtime exposes exact capability-denied diagnostic.
- Runtime exposes exact duplicate/idempotency diagnostic.
- Runtime exposes exact header-persistence-failed diagnostic.
- API envelope preserves stable admission durability code.

## 2. Trophy Allocation
| Behavior | Layer | Rationale |
|---|---|---|
| error variant codes | unit | pure enum/display/code mapping |
| duplicate code | unit | existing runtime error |
| source preservation | unit | `Error::source`/fields |
| persistence failure code | integration | journal failure path |
| API envelope | integration | cross-crate public surface |

## 3. BDD Scenarios
### admission_durability_error_variants_are_exhaustive
Given: each admission failure cause.
When: converting to `RuntimeError`.
Then: exact variant, diagnostic code, runtime code, and fields match.

### admission_header_persistence_failure_has_dedicated_diagnostic
Given: journal append fails before header persistence.
When: submit is attempted.
Then: exact header-persistence diagnostic is returned and source is preserved.

### duplicate_run_id_preserves_stable_diagnostic_code
Given: run id already exists.
When: second submit occurs.
Then: `RunAlreadyExists` code differs from admission durability codes.

### api_envelope_preserves_admission_durability_code
Given: public API/CLI/IPC receives admission durability error.
When: envelope is produced.
Then: stable code field equals the exact expected code without parsing display text.

## 4. Proptest Invariants
- Every `RuntimeError` admission durability variant maps to a unique stable diagnostic code.

## 5. Fuzz Targets
- None in scope.

## 6. Kani Harnesses
- None in scope.

## 7. Mutation Checkpoints
- Deleting a match arm must be caught by exhaustive variant test.
- Returning a generic storage code must be caught by dedicated diagnostic test.
- Threshold: 90% mutation kill rate minimum.

## 8. Combinatorial Coverage Matrix
| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| absent artifact | missing digest | exact not-found error | unit |
| invalid/stale/digest | bad accepted artifact | exact invalid/digest error | unit |
| denied capability | missing capability | exact denied error | unit |
| duplicate | existing run | exact duplicate error | unit |
| persistence fail | journal append fail | exact durability error | integration |
| envelope | public surface | exact code field | integration |

## Open Questions
- Exact naming of new durability variant is owned by Holzman implementation.
