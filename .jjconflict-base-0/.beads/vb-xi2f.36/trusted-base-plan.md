# Trusted Base Plan: vb-xi2f.36

## Purpose

This document defines the **trusted base** for the `together` primitive acceptance proof — the set of functions, types, and behaviors that are assumed correct and not verified as part of this bead's proof obligations.

---

## Trusted Functions

### vb_yaml/src/ast/parse.rs

| Function | Location | Trust Justification |
|----------|----------|---------------------|
| `lookup()` | parse.rs:1–50 | Preexisting; used by all parse functions; covered by vb_yaml integration tests |
| `mapping()` | parse.rs:51–100 | Preexisting; saphyr library wrapper; no custom logic |
| `sequence()` | parse.rs:101–150 | Preexisting; saphyr library wrapper; no custom logic |
| `require_str_in()` | parse.rs:151–200 | Preexisting; simple scalar extraction; covered by tests |
| `require_u16()` | parse.rs:201–230 | Preexisting; numeric parsing; no overflow risk (bounded) |
| `opt_str()` | parse.rs:231–260 | Preexisting; simple Option wrapper |
| `opt_u32()` | parse.rs:261–290 | Preexisting; simple Option wrapper |
| `reject_unknown_fields()` | parse.rs:291–350 | Preexisting; field filtering; covered by tests |
| `require_scalar_in()` | parse.rs:351–400 | Preexisting; scalar validation |
| `reject_unknown_step_fields()` | parse.rs:401–450 | Preexisting; calls reject_unknown_fields |

**Trust level**: HIGH — all functions are simple wrappers around saphyr library; no complex logic.

---

### vb_yaml/src/ast/types.rs

| Type | Trust Justification |
|------|---------------------|
| `StepPrimitive` enum | Preexisting; variant `Together` already exists; we only add string mapping |
| `TogetherBranch` struct | Preexisting; fields `label: String`, `steps: Vec<StepAst>` |
| `StepAst` struct | Preexisting; complete type definition |
| `YamlError` enum | Preexisting; error variants already defined |

**Trust level**: HIGH — data structures are preexisting; no changes required to types themselves.

---

### vb_core (compiled types)

| Type/Function | Location | Trust Justification |
|---------------|----------|---------------------|
| `CompiledNodeKind::TogetherStart` | vb_core/src/workflow/mod.rs | Preexisting; already used for `parallel`; we only change string name |
| `CompiledNode` struct | vb_core/src/workflow/mod.rs | Preexisting; fully tested via budget tests |
| `StepIdx` | vb_core/src/types.rs | Preexisting; newtype wrapper |
| `SlotIdx` | vb_core/src/types.rs | Preexisting; newtype wrapper |

**Trust level**: HIGH — these types are preexisting and fully tested.

---

### vb_compile/src/compile/mod.rs

| Function | Trust Justification |
|----------|---------------------|
| `lower_together()` | Preexisting; already handles `parallel`; we only need to ensure it's reachable with `Together` variant. Branch count limit check (`u16::try_from`) is preexisting and covered by `vb_qi37.2.4` budget tests. |

**Trust level**: MEDIUM — logic is preexisting but the `Together` variant reachability is new.

---

## Untrusted (Verified in This Bead)

| Function | File | Verification Lane |
|----------|------|-------------------|
| `is_primitive()` | vb_yaml/src/ast/parse_steps.rs:85-102 | Kani + Verus |
| `parse_step_primitive()` | vb_yaml/src/ast/parse_steps.rs:68-83 | Kani + Verus |
| `parse_parallel()` | vb_yaml/src/ast/parse_steps.rs:192-204 | Kani + Verus + Proptest |
| `STEP_PRIMITIVES` arrays (3) | vb_validate/src/schema.rs, schema_fields.rs, validation.rs | Kani + Proptest |
| `ALLOWED_STEP_FIELDS` arrays (2) | vb_validate/src/schema.rs, schema_fields.rs | Kani |
| `StepPrimitive::from_field()` | vb_compile/src/mod_compile_lowering/part_09.rs:16-33 | Kani + Verus |
| `StepPrimitive::as_str()` | vb_compile/src/mod_compile_lowering/part_09.rs:35-51 | Verus |

---

## Dependency Graph

```
TRUSTED (not verified in this bead)
├── saphyr YAML library (external)
├── vb_yaml/src/ast/parse.rs (all helpers)
├── vb_yaml/src/ast/types.rs (data structures)
├── vb_core/workflow/mod.rs (CompiledNode, TogetherStart)
├── vb_core/types.rs (StepIdx, SlotIdx)
└── lower_together() logic (preexisting)

UNTRUSTED (verified in this bead)
├── is_primitive() ←─ depends on: trusted parse.rs helpers
├── parse_step_primitive() ←─ depends on: is_primitive, parse_parallel
├── parse_parallel() ←─ depends on: trusted parse.rs helpers
├── STEP_PRIMITIVES arrays ←─ depends on: none (static data)
├── StepPrimitive::from_field() ←─ depends on: none (pure fn)
└── StepPrimitive::as_str() ←─ depends on: none (pure fn)
```

---

## Trust Boundaries

1. **Parse boundary**: YAML bytes → `saphyr::Yaml` → `StepPrimitive::Together`. The saphyr library is trusted. Our code sits between saphyr output and our AST types.

2. **Compile boundary**: `StepPrimitive::Together` → `CompiledNodeKind::TogetherStart`. The `StepPrimitive` enum and `CompiledNodeKind` are trusted; the string mappings are untrusted.

3. **Validation boundary**: YAML map → `WorkflowDoc` → `validate_workflow_schema()`. The document model is trusted; the `STEP_PRIMITIVES` arrays are untrusted.

---

## Waiver Requests

None — all functions in the trust base are either preexisting tested code or external libraries assumed correct by the project.

---

## Change Log

| Date | Change | Justification |
|------|--------|---------------|
| 2026-05-24 | Initial trusted base | New bead — no prior trust base |
