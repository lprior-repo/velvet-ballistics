# Codebase Map: vb-xi2f.36 — Accept Canonical Together Primitive Name

## Bead Scope
Accept `"together"` as a valid YAML workflow primitive key name alongside the existing
`"parallel"` alias. The internal AST type `StepPrimitive::Together` and all IR/nodes
(`TogetherStart`, `TogetherBranch`, `TogetherJoin`) are already correctly named;
only the YAML-facing key name recognition is missing.

---

## Key Finding

The YAML parsing layer only recognizes `"parallel"` as the step primitive key that
produces `StepPrimitive::Together`. There is NO recognition of `"together"` anywhere.
The canonical name for the YAML key should arguably be `"together"` (matching the
Rust type and user-facing CLI messages that say `'together' (parallel)`).

---

## 1. YAML Parsing — `crates/vb_yaml/src/ast/parse_steps.rs`

| Line | Symbol | Finding |
|------|--------|---------|
| 74 | `parse_step_primitive` match arm | Only `"parallel" => parse_parallel(sub)` — no `"together"` arm |
| 85–102 | `is_primitive()` | `"parallel"` listed; `"together"` **absent** |
| 105–131 | `reject_unknown_step_fields()` | `"parallel"` in field allowlist; `"together"` **absent** |
| 192–204 | `parse_parallel()` | Already named `parse_parallel`; produces `StepPrimitive::Together { branches }` — this function body is correct; only the key dispatch is wrong |

**Files to change:** `crates/vb_yaml/src/ast/parse_steps.rs`

**Changes needed:**
- Add `"together"` to `is_primitive()` match list
- Add `"together" => parse_parallel(sub)` arm in `parse_step_primitive()`
- Add `"together"` to `reject_unknown_step_fields()` allowlist

---

## 2. Validation — `crates/vb_validate/src/`

Three separate `STEP_PRIMITIVES` constants (schema.rs, schema_fields.rs, schema/validation.rs)
all list `"parallel"` but not `"together"`:

| File | Line | Finding |
|------|------|---------|
| `schema.rs` | 38–50 | `STEP_PRIMITIVES` — `"parallel"` present, `"together"` absent |
| `schema_fields.rs` | 34–46 | `STEP_PRIMITIVES` — `"parallel"` present, `"together"` absent |
| `schema/validation.rs` | 36–39 | `STEP_PRIMITIVES` — `"parallel"` present, `"together"` absent |

**No dedicated `validate_together()` function exists** — validation of the Together structure
appears to be handled at the compile/IR layer, not at the schema layer. `InvalidTogether`
error variant exists (validation error) but is only raised by compile-layer checks, not
by schema validation of the primitive name.

**Files to change:** `schema.rs`, `schema_fields.rs`, `schema/validation.rs`

---

## 3. Compilation / IR Lowering — `crates/vb_compile/src/compile/mod.rs`

| Line | Symbol | Finding |
|------|--------|---------|
| 203–218 | `canonical_primitive_name()` | Maps `StepPrimitive::Together` → `"parallel"` (line 210) |
| 416–454 | `lower_together()` | IR-lowers `Together` AST to `CompiledNodeKind::TogetherStart/TogetherBranch/TogetherJoin` — correct |
| 243–261 | `digest_step_primitive()` | Uses `canonical_primitive_name()` for hashing — correct |
| 424 | Lowering error message | Uses `"parallel"` as primitive field name in error |

**Note:** `canonical_primitive_name()` is used for hashing/digest and error messages.
If both `"parallel"` and `"together"` are accepted as YAML keys, they must produce
identical digests. The fix should keep `"parallel"` as the canonical/inner name
(the `canonical_` function maps `Together → "parallel"`), while accepting `"together"`
as an input alias at parse time.

**Files to change:** `crates/vb_compile/src/compile/mod.rs` — likely no changes needed
if the parse-layer aliasing is done correctly (both keys map to `StepPrimitive::Together`,
which then canonicalizes to `"parallel"`).

---

## 4. Core IR — `crates/vb_core/src/`

| File | Symbol | Finding |
|------|--------|---------|
| `workflow/mod.rs` | `CompiledNodeKind::TogetherStart/Branch/Join` | Already correct; no changes needed |
| `nodes.rs` | `TogetherStart/Branch/Join` variants | Already correct; no changes needed |
| `errors.rs` | `TogetherBranchLimitExceeded` | Already correct; no changes needed |
| `budget.rs` | `max_together_branches` tracking | Already correct; no changes needed |

**Risk:** No changes expected in vb_core for this bead.

---

## 5. Runtime Execution — `crates/vb_runtime/src/engine/execute.rs`

Lines 105–132 dispatch `TogetherStart`, `TogetherBranch`, `TogetherJoin` nodes.
No changes needed — execution is driven by `CompiledNodeKind` variants, not YAML key names.

---

## 6. IPC Serialization — `crates/vb_ipc/src/payloads.rs`

Lines 259–261, 299–301, 340–342 define `NodeKind::TogetherStart/Branch/Join` and their
string serializations. No changes needed.

---

## 7. CLI Error Rendering — `crates/vb_cli/src/app_impl.rs`

Line 4791–4800: `ValidationError::InvalidTogether` renders user message
`'together' (parallel) construct`. This is already user-friendly; no change needed.

---

## 8. Related Compilation Tests — `crates/vb_compile/tests/v1_primitive_lowering.rs`

Lines 986–1113: Test helpers for `TogetherStart/Branch/Join` use string names
`"TogetherStart"`, `"TogetherBranch"`, `"TogetherJoin"` (CompiledNodeKind-level,
not YAML-level). No changes needed for this bead.

---

## Open Questions

1. **Should `"together"` replace `"parallel"` as the canonical YAML key, or coexist as an alias?**
   Current `canonical_primitive_name()` function maps `Together → "parallel"`. If `"together"`
   is the user-facing canonical name, this function may need updating, which would affect
   workflow digests. The safest path is to accept both keys at parse time and keep
   `"parallel"` as the canonical inner name.

2. **Should `reject_unknown_step_fields()` in parse_steps.rs be updated to also accept
   `"together"`? YES** — this is needed so that `together:` in a YAML step is not
   rejected as an unknown field before `parse_step_primitive` is even reached.

3. **Are there existing YAML test fixtures that use `"together"` key that are currently
   passing/failing?** No evidence of `"together"` as a YAML key in any test fixtures.
   All tests use `"parallel"`.

---

## Risk Tags

- **parser/codec**: YAML key name aliasing — adding `"together"` as accepted primitive key
- **no_unsafe**: No unsafe code in affected paths
- **no_concurrency**: No new concurrency introduced
- **no_temporal**: No timing/scheduling changes

---

## Recommended Downstream Owners

- **rust-contract**: Model `StepPrimitive::Together` aliasing behavior; confirm that
  `canonical_primitive_name` digest stability is preserved when both keys are accepted
- **proof-planner**: Kani harness for `parse_step_primitive` to confirm `"together"`
  is accepted and `"togetherxyz"` is rejected
- **test-planner**: Add YAML parsing tests for `"together"` key and round-trip tests
  confirming `"parallel"` and `"together"` produce identical digests
