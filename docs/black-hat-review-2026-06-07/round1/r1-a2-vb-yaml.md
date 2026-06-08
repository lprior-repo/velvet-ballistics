# R1-A2: vb_yaml Inventory

**Agent:** explore · **Date:** 2026-06-07
**Scope:** `crates/vb_yaml/` (saphyr-parser-backed YAML 1.2 core subset)
**Files:** 34 .rs files, 6,234 LoC production + 871 LoC test = 7,105 LoC total
**Module tree:** lib.rs + ast/, diag/, lexer/, parser/, validate/

## File Counts

| Type | Count | LoC |
|------|------:|----:|
| .rs production | 18 | 4,801 |
| .rs test | 11 | 1,189 |
| .rs kani harnesses | 5 | 344 |
| **Total** | **34** | **7,105** |

Largest 3 files:
1. `crates/vb_yaml/src/ast/types.rs` — 512 LoC (WorkflowSource, StepAst, StepPrimitive, Trigger, Version)
2. `crates/vb_yaml/src/ast/parse.rs` — 894 LoC (main YAML→AST lower)
3. `crates/vb_yaml/src/diag/codes.rs` — 387 LoC (11 YAML-specific error codes)

## Public API

- `parse_workflow(&[u8]) -> Result<WorkflowSource, Vec<YamlError>>` — single entry point
- `WorkflowSource { version: String, name: String, when: Trigger, vars: ..., steps: Vec<StepAst>, ... }`
- `StepAst` has 11 primitive variants + 3 aliases (save→set, run→do, foreach→for_each)
- `Trigger` has 4 variants (manual, schedule, event, webhook)

## Kani Harnesses (5)

| File | Kani harness | Compiled? |
|------|--------------|:---------:|
| `kani_yaml_error_code.rs` | `#[kani::proof] fn kani_yaml_error_code_range()` | ✓ in lib.rs |
| `kani_is_primitive_legacy.rs` | `#[kani::proof] fn kani_is_primitive_matches_legacy_names()` | ❌ ORPHANED |
| `kani_all_variants_registered.rs` | `#[kani::proof] fn kani_all_step_primitives_registered()` | ❌ ORPHANED |
| `kani_checked_add.rs` | `#[kani::proof] fn kani_checked_add_no_overflow()` | ❌ ORPHANED |
| `kani_panic_freedom.rs` | `#[kani::proof] fn kani_yaml_panic_freedom()` | ❌ ORPHANED |

**4 of 5 Kani files are ORPHANED** (not in `lib.rs` module tree, not compiled, not exercised by `cargo kani`).

## Active Kani Proof

`kani_yaml_error_code.rs:42-67` is a vacuum proof — it asserts that the YamlError code range 0x0A00..=0x0AFF is not a gap. It does NOT bind to any production `YamlError` variant. **This is a GOD-RULE 1 violation: hardcoded Kani shape that proves only "the constant is registered."**

## is_primitive Defensive Match

`crates/vb_yaml/src/ast/parse.rs:454-460`:
```rust
match primitive_str {
    "set" | "save" => Ok(StepPrimitive::Set(...)),
    "do" | "run" => Ok(StepPrimitive::Do(...)),
    "choose" => Ok(StepPrimitive::Choose(...)),
    "for_each" | "foreach" => Ok(StepPrimitive::ForEach(...)),
    "together" => Ok(StepPrimitive::Together(...)),
    "collect" => Ok(StepPrimitive::Collect(...)),
    "reduce" => Ok(StepPrimitive::Reduce(...)),
    "repeat" => Ok(StepPrimitive::Repeat(...)),
    "wait" => Ok(StepPrimitive::Wait(...)),
    "ask" => Ok(StepPrimitive::Ask(...)),
    "finish" => Ok(StepPrimitive::Finish(...)),
    "parallel" | "aggregate" => { /* legacy: silent reject */ }
    _ => Err(YamlError::UnknownStepPrimitive { ... }),
}
```

The 2 legacy variants "parallel" and "aggregate" produce a silent reject (no error), but the doc comment says "legacy names" implying they should be accepted. The Kani harness `kani_is_primitive_legacy.rs` (orphaned) asserts the legacy match arms are tested. **Brittle defense-in-depth — the orphaned test is the only witness, and it's never compiled.**

## Forbidden Pattern Audit

| Pattern | Production | Test |
|---------|----------:|-----:|
| `unwrap()` | 0 | 1 (test only) |
| `expect()` | 0 | 0 |
| `panic!()` | 0 | 0 |
| `unsafe` | 0 | 0 |
| YAML anchors `&` / aliases `*` / merge keys `<<` | ✓ rejected in lexer | n/a |
| custom tags `!!omap` | ✓ rejected | n/a |
| multi-doc `---` | ✓ rejected | n/a |
| YAML 1.1 booleans (`yes`/`no`/`on`/`off`) | ✓ rejected by saphyr strict | n/a |

## verdict

**70 / 100 — Production correct, Kani formal-verification story broken.**

Top concerns:
1. 4/5 Kani files are orphaned (not in module tree; not compiled; not exercised)
2. The 1 active Kani proof is a vacuum model (GOD-RULE 1+2)
3. `is_primitive` legacy defensive match is brittle
4. The production code itself is clean and master-conformant for §8 YAML 1.2 core subset
