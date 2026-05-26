# Codebase Map — vb-core-lower-coverage-matrix

## Bead Context
- **Bead ID**: vb-core-lower-coverage-matrix
- **Goal**: Prove every v1 YAML construct is accepted/rejected consistently across vb_yaml, vb_validate, and vb_compile
- **Excludes**: codegen/generated mode

## Source Checkout
`/home/lewis/src/velvet-ballistics`

## Isolated Workspace
`/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-lower-coverage-matrix`

---

## Relevant Crates

### vb_yaml (`crates/vb_yaml/`)
Cold-path YAML parsing and profile enforcement.

| File | Purpose |
|------|---------|
| `src/lib.rs` | Public API: `parse_yaml_events`, `parse_workflow_source`, `validate_yaml_profile` |
| `src/ast/types.rs` | `WorkflowSource`, `StepAst`, `StepPrimitive` (Set, Save, Do, Choose, ForEach, Together, Collect, Reduce, Repeat, Wait, Ask, Finish) |
| `src/ast/parse.rs` | AST parsing from YAML events |
| `src/profile.rs` | Strict YAML profile enforcement |
| `src/profile_validation.rs` | Profile validation rules |
| `src/profile_tests.rs` | Profile acceptance tests |
| `src/profile_tests_adversarial.rs` | Adversarial profile tests |
| `src/events.rs` | YAML event types |
| `src/error.rs` | `YamlResult`, `YamlError` |

### vb_validate (`crates/vb_validate/`)
Cold-path workflow validation.

| File | Purpose |
|------|---------|
| `src/lib.rs` | `ValidationError` enum, gate functions |
| `src/gates.rs` | Gate validation functions (gate_07 through gate_15) |
| `src/references.rs` | Reference validation (`RefTables`, `validate_single_reference`) |
| `src/type_taint.rs` | Type/taint checking |
| `src/control_flow.rs` | Control flow validation |
| `src/schema.rs` | Schema validation |
| `src/schema/validation.rs` | Schema validation logic |
| `src/gate_tests.rs` | Gate tests |

### vb_compile (`crates/vb_compile/`)
Cold-path YAML compiler boundary.

| File | Purpose |
|------|---------|
| `src/lib.rs` | `YamlCompiler`, `compile_source`, `compile_workflow` |
| `src/lower/mod.rs` | Step lowering to IR |
| `src/ast/parse.rs` | AST parsing |
| `src/ast/types.rs` | AST types |
| `src/compile/mod.rs` | Compilation logic |
| `src/compile/expression.rs` | Expression compilation |
| `src/references.rs` | Reference compilation |
| `src/strict_yaml.rs` | Strict YAML rejection |
| `tests/v1_primitive_lowering.rs` | **KEY: v1 primitive lowering coverage tests** |

---

## v1 YAML Construct Taxonomy

### Top-Level Fields
| Construct | vb_yaml | vb_validate | vb_compile | Notes |
|-----------|---------|-------------|------------|-------|
| `version` | ✓ AST | ✓ Schema check | ✓ Parse | Required, must be "velvet-ballistics/v1" |
| `name` | ✓ AST | ✓ Schema check | ✓ Parse | Required |
| `when` (trigger) | ✓ AST | ✓ Gate 10/11 | ✓ Parse | Manual, Schedule, Event, Webhook |
| `inputs` | ✓ AST | ✓ Schema | ✓ Rejected | UnsupportedTopLevelDeclaration |
| `vars` | ✓ AST | ? | ? | Likely accepted but not validated |
| `secrets` | ✓ AST | ? | ? | Likely accepted but not validated |
| `steps` | ✓ AST | ✓ Gate 07 | ✓ Lowering | Core workflow body |
| `result` | ✓ AST | ✓ Schema | ✓ Rejected | UnsupportedTopLevelResult |
| `examples` | ✓ AST | ? | ? | Likely ignored |

### Step Primitives (v1)
| Primitive | vb_yaml | vb_validate | vb_compile | Parity Tests |
|-----------|---------|-------------|------------|--------------|
| `set` | ✓ Parse | ✓ Gate 07 | ✓ Lowering | `v1_primitive_lowering.rs` |
| `save` | ✓ Parse | ✓ Gate 07 | ✓ Lowering | `v1_primitive_lowering.rs` (Set variant) |
| `do` | ✓ Parse | ✓ Gate 07 | ✓ Rejected | `v1_primitive_lowering.rs` - UnsupportedStepPrimitive |
| `choose` | ✓ Parse | ✓ Gate 07 | ✓ Rejected | `v1_primitive_lowering.rs` - UnsupportedStepPrimitive |
| `for_each` | ✓ Parse | ✓ Gate 07+08 | ✓ Lowering | `v1_primitive_lowering.rs` |
| `together` | ✓ Parse | ✓ Gate 07+09 | ✓ Lowering | `v1_primitive_lowering.rs` |
| `collect` | ✓ Parse | ✓ Gate 07+09 | ✓ Lowering | `v1_primitive_lowering.rs` |
| `reduce` | ✓ Parse | ✓ Gate 07+09 | ✓ Lowering | `v1_primitive_lowering.rs` |
| `repeat` | ✓ Parse | ✓ Gate 07+09 | ✓ Lowering | `v1_primitive_lowering.rs` |
| `wait` | ✓ Parse | ✓ Gate 07+09 | ✓ Lowering | `v1_primitive_lowering.rs` |
| `ask` | ✓ Parse | ✓ Gate 07+09 | ✓ Lowering | `v1_primitive_lowering.rs` |
| `finish` | ✓ Parse | ✓ Gate 07 | ✓ Lowering | `v1_primitive_lowering.rs` |

### Step Fields
| Field | vb_yaml | vb_validate | vb_compile | Notes |
|-------|---------|-------------|------------|-------|
| `id` | ✓ Parse | ✓ Uniqueness | ✓ Uniqueness | Duplicate check |
| `name` | ✓ AST | ✓ Gate 10 | ✓ Rejected | UnsupportedStepControlField |
| `condition` | ✓ Parse | ? | ✓ Expr parse | Expression field |
| `with` | ✓ AST | ? | ? | Connector reference |
| `retry` | ✓ AST | ✓ Gate 12 | ✓ Lowering | Retry policy |
| `on_error` | ✓ AST | ✓ Gate 13 | ✓ Lowering | Error handler |
| `then` | ✓ AST | ? | ? | Next-step label |

### Trigger Variants
| Variant | vb_yaml | vb_validate | vb_compile |
|---------|---------|-------------|------------|
| `manual` | ✓ | ✓ | ✓ |
| `schedule` | ✓ | ✓ | ✓ |
| `event` | ✓ | ✓ | ✓ |
| `webhook` | ✓ | ✓ | ✓ |

---

## Known Gaps / Questions

1. **`vars` validation**: Does vb_validate check `vars` declarations?
2. **`secrets` validation**: Does vb_validate check `secrets` declarations?
3. **`examples` handling**: Are examples validated or ignored?
4. **`with` connector reference**: Is this validated anywhere?
5. **`then` field**: Is next-step label validated?
6. **`condition` field**: Is the expression syntax validated?

---

## Verification Artifacts

| Artifact | Path | Status |
|----------|------|--------|
| v1 primitive lowering tests | `crates/vb_compile/tests/v1_primitive_lowering.rs` | EXISTS - 1350+ lines |
| Verus proof | `verification/verus/v1_primitive_lowering.rs` | EXISTS - 357 lines |
| Profile tests | `crates/vb_yaml/src/profile_tests.rs` | EXISTS |
| Profile adversarial tests | `crates/vb_yaml/src/profile_tests_adversarial.rs` | EXISTS |
| Gate tests | `crates/vb_validate/src/gate_tests.rs` | EXISTS |

---

## Risk Tags
- **parser/codec**: YAML parsing and AST construction
- **no_codegen**: This bead explicitly excludes codegen mode
- **parity**: Ensuring consistent accept/reject across 3 crates
- **coverage**: Proving complete construct coverage

---

## Downstream Owners
- `rust-contract`: Requirements and invariants
- `proof-planner`: Verus/Kani lanes
- `test-planner`: Missing parity tests
- `holzman-rust`: Implementation changes if needed
