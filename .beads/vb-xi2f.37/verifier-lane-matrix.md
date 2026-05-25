# Verifier Lane Matrix - vb-xi2f.37

## Lane Coverage Map

| Contract Clause | Kani | Verus | Cargo-Test | TLA+ | Miri | Loom | Flux | Proptest | Fuzz |
|-----------------|------|-------|------------|------|------|------|------|----------|------|
| CC-001: reduce recognized | ✅ PS-001 | ❌ N/A | ✅ PS-014 | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A | ⚠️ WAIVED |
| CC-002: reduce not rejected | ✅ PS-006 | ❌ N/A | ✅ PS-007 | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A |
| CC-003: Reduce variant | ❌ N/A | ✅ PS-009 | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A |
| CC-004: canonical name | ❌ N/A | ❌ N/A | ✅ PS-012 | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A | ❌ N/A |

## Legend
- ✅ = Required lane
- ⚠️ = Waived with reason
- ❌ N/A = Not applicable with rationale on record

## Non-Applicable Lanes Summary

| Lane | Evidence |
|------|----------|
| TLA+ | Parsing is stateless local transformation; no temporal properties |
| Miri | #![forbid(unsafe_code)] in vb_yaml/src/ast/types.rs and parse_steps.rs |
| Loom | No concurrency primitives in YAML parsing |
| Flux | StepPrimitive is plain enum, no refinement types needed |
| Proptest | Single string mapping has no property to test |
