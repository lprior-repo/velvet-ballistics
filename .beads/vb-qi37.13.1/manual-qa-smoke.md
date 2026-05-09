# Manual QA Smoke Report — vb-qi37.13.1

**Bead**: `vb-qi37.13.1` — cli: Define structured envelope schemas
**Date**: 2026-05-09
**Phase**: State 7 — Manual Smoke QA

---

## Command Run

```bash
cd /home/lewis/src/Velvet-ballistics && cargo test -p velvet_ballastics --test envelope_schema_tests --no-run
```

---

## Output Summary

Compilation failed with **140 errors**.

### Critical Errors

#### 1. Missing crate `vb_ui_model` (E0433)

```
error[E0433]: cannot find module or crate `vb_ui_model` in this scope
   --> crates/velvet_ballastics/tests/envelope_schema_tests.rs:287:20
```

All tests in `envelope_schema_tests` reference `vb_ui_model::envelope::*` types:
- `SchemaVersion`
- `EnvelopeKind`
- `MetadataEnvelope`
- `DiagnosticEnvelope`
- `PayloadEnvelope`
- `OutputEnvelope`
- `CURRENT_SCHEMA_VERSION`

The `vb_ui_model` crate does not exist or is not a dependency of `velvet_ballastics`.

#### 2. Wrong method name `RunId::from_u64` (E0599)

```
error[E0599]: no associated function or constant named `from_u64` found for struct `RunId`
   --> crates/velvet_ballastics/tests/envelope_schema_tests.rs:131:16
```

The `numeric_id!` macro generates `from` or `new`, not `from_u64`. Error appears in ~20 test cases.

#### 3. Non-exhaustive match in main.rs (E0004)

```
error[E0004]: non-exhaustive patterns: `&ValidationError::AccessorPathTooDeep { .. }`
              and `&ValidationError::AccessorSymbolOutOfBounds { .. }` not covered
   --> crates/velvet_ballastics/src/main.rs:3374:11
```

Unrelated to this bead, but blocking compilation.

---

## Evidence

```
error: could not compile `velvet_ballastics` (test "envelope_schema_tests") due to 140 previous errors; 1 warning emitted
```

Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e0efcf7d5001nRMiKQrq2u2cNv`

---

## Diagnosis

The test file at `crates/velvet_ballastics/tests/envelope_schema_tests.rs` is in **RED PHASE** (as documented on line 12 of the test file itself). The tests are written to document the expected API contract for envelope types that do not yet exist.

The test file is correctly structured and the API design is documented, but:
1. The `vb_ui_model::envelope` module has not been implemented
2. The `vb_ui_model` crate may not exist or is not wired as a test dependency
3. API usage errors (`from_u64` vs `from`) indicate test编写 drift from actual `RunId` API

---

## Files Reviewed

| File | Status |
|------|--------|
| `.beads/vb-qi37.13.1/contract.md` | NOT ON DISK (bead exists in Dolt only) |
| `.beads/vb-qi37.13.1/test-plan.md` | NOT ON DISK |
| `.beads/vb-qi37.13.1/red-phase.md` | NOT ON DISK |
| `crates/velvet_ballastics/tests/envelope_schema_tests.rs` | EXISTS — RED PHASE, 626 lines |

---

## Bead Status (from `bd show`)

```
vb-qi37.13.1 · cli: Define structured envelope schemas [● P0 · IN_PROGRESS]
```

Parent: `vb-qi37.13` — cli: Reconcile structured output contract
Blocks: `vb-qi37.13.2`, `vb-qi37.13.3`

---

## Verdict

**Tests do not compile.** The envelope schema implementation does not exist yet. This is expected RED PHASE behavior per the test file's own documentation.

The test file correctly documents the expected API contract. Implementation of `vb_ui_model::envelope` module is required before tests can compile.

---

## Recommendation

1. Implement `vb_ui_model::envelope` module with all envelope types per the contract
2. Fix `RunId::from_u64` → `RunId::new` throughout tests
3. Resolve `ValidationError` exhaustive match in `main.rs` (unrelated to this bead)
4. Re-run `cargo test -p velvet_ballastics --test envelope_schema_tests --no-run`

---

**STATUS: FAIL**
