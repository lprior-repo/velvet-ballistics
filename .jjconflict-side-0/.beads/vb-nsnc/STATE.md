# vb-nsnc STATE

- Current State: State 15 Complete — all acceptance tests written and passing
- Title: `verifier/runtime: Define capability contract schema`
- Branch/Workspace: `/home/lewis/src/Velvet-ballistics`
- Bookmark: `main`
- Claim Evidence: `bd update vb-nsnc --claim` succeeded from `/home/lewis/src/Velvet-ballistics`
- Landing Evidence: Implementation in `crates/vb_validate/src/gates.rs`, `lib.rs`, `diagnostic.rs`, `diag_codes.rs`, `diag_convert.rs`, `diag_render.rs`; 990/990 vb_validate tests pass including 4 missing acceptance tests now added to `crates/vb_validate/tests/capability_contract_schema.rs`

## State 1.5 Verification Artifacts (Supplemented)

| Artifact | Lines | Status |
|----------|-------|--------|
| `contract.md` | 257 | ✓ Complete, comprehensive |
| `lean-contract.md` | 95 | ✓ 6 theorems + 2 waivers |
| `verification-layers.md` | 78 | ✓ 26 layers assigned |
| `proof-obligations.jsonl` | 31 | ✓ Valid JSONL, all required |
| `traceability-matrix.jsonl` | 15 | ✓ Valid JSONL, all clauses covered |
| `martin-fowler-tests.md` | 200+ | ✓ 25+ test cases + 10 scenarios |
| `contract-verification-review.md` | 95 | ✓ STATUS: APPROVED |

## Verification Gate Results

```bash
test -s .beads/vb-nsnc/contract.md           # 257 lines
test -s .beads/vb-nsnc/lean-contract.md      # 95 lines
test -s .beads/vb-nsnc/verification-layers.md # 78 lines
jq -c . proof-obligations.jsonl               # 31 valid JSON objects
jq -c . traceability-matrix.jsonl               # 15 valid JSON objects
```

**All mandatory gate files pass.**

## Coverage Summary

- **Contract clauses:** 15 traced to 31 proof obligations
- **Lean theorems:** 6 (grammar_valid, length_bound, first_error_precedence, duplicate_detection, duplicate_scope, action_relation)
- **Verification layers:** 26 (kani:6, proptest:5, cargo-fuzz:1, unit:10, integration:4, e2e:1, static-scan:3, api-compat:1)
- **Waivers:** 2 (WAIVER-001 gate orchestration, WAIVER-002 diagnostic formatting)
- **Independent review:** APPROVED — no lethal findings

## Blocking Status

- **BLOCKS:** `vb-7ode` (runtime: Enforce capabilities at action dispatch)
- `vb-7ode` is at State 1 — **UNBLOCKED** — vb-nsnc contract schema verified complete
- vb-nsnc is at State 15 (Landed)

## vb-7ode Handoff Context

vb-7ode implementer must:
1. Read `.beads/vb-nsnc/contract.md` for capability contract schema (capability name grammar, error taxonomy, invariants)
2. Read `.beads/vb-nsnc/verification-layers.md` for expected verification coverage
3. Read `.beads/vb-nsnc/lean-contract.md` for pure kernel theorems
4. Proceed from State 1 codebase mapping with full contract context
5. Not implement capability checks in runtime hot paths — this bead defined the schema only; enforcement is vb-7ode

## Residual Notes

- Lean theorem implementation (`VBValidate.Capability` module) not yet encoded — Kani + proptest provide compensating evidence per WAIVER-001
- Diagnostic string formatting not Lean-proved — unit tests on exact codes per WAIVER-002

## Acceptance Tests Added (This Session)

The following 4 tests were missing and have been added to `crates/vb_validate/tests/capability_contract_schema.rs`:

| Test | Description | Status |
|------|-------------|--------|
| `test_declared_capability_passes_verification` | Valid lowercase capability name passes | ✓ Pass |
| `test_multiple_declared_capabilities_are_preserved` | Multiple distinct capabilities preserved | ✓ Pass |
| `test_missing_capability_fails_verification` | Empty name fails with CAPABILITY_NAME_EMPTY | ✓ Pass |
| `test_unknown_capability_kind_fails_verification` | Invalid format fails with CAPABILITY_NAME_INVALID | ✓ Pass |
