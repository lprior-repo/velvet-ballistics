# Contract v3 Assessment: Corrected Unsupported Claims

**Bead:** vb-8eocj.4
**Prior artifact:** `.beads/vb-8eocj.4/contract.md` (raw draft, not accepted)

## Errors in Prior Draft Contract

### 1. `allocate_input_slots` described as SlotCompiler method
**Draft claim:** "Input slots are allocated in `compile_source` via `SlotCompiler.allocate_input_slots`"
**Correction:** `allocate_input_slots` is a FREE FUNCTION defined in part_01.rs (line 95).
It takes `inputs: &[InputField]` and `builder: &mut SlotCompiler` as arguments; it is not
a method on SlotCompiler. The builder's `record_slot` and `record_input_slot` methods are
called by this free function.

### 2. Resolver mismatch not identified
**Draft claim:** "Be resolvable by `resolve_slot_reference` in Set expressions"
**Correction:** The draft did not identify the critical resolver mismatch.
`prepare_finish_input_slots` inserts `"$input.<name>"` (with `$`) into `outputs`, but
`resolve_slot_reference` strips the `$` prefix and looks up `"input.<name>"`. This mismatch
must be fixed in the proposal — insert names WITHOUT the `$` prefix so the standard
resolver path can find them.

### 3. Wrong type map name
**Draft claim:** "`prior_types` HashMap (name -> ExprType) drives static type checking"
**Correction:** The actual current code uses `slot_types: HashMap<SlotIdx, ExprType>`
(SlotIdx as key, not name). There is no `prior_types` map in the v18 codebase.

### 4. Wrong input schema type name
**Draft claim:** `"type: i64"` in acceptance fixture
**Correction:** The v18 YAML parser accepts `text`, `number`, `boolean`, `object`, `any`,
and `list<...>` variants. There is no `i64` shorthand. The fixture should use `text` or
`number` to match actual parser behavior.

### 5. Do expression generalization claimed without basis
**Draft claim:** Implied that Do input handling requires new expression generalization
**Correction:** `lower_canonical_do` already passes the input expression string directly to
`resolve_slot_reference` via `resolve_do_input`. No expression generalization is needed;
the Do path already supports `$name` references.

### 6. Non-string schema described as invalid
**Draft claim:** "Non-string input schema" as error case
**Correction:** Mapping schemas `{ is: "<type>" }` are valid. Only unrecognized type names
or non-string schema values are invalid. The error case should be "unrecognized type name"
not "non-string schema."

### 7. Wrong error kind for unknown input ref
**Draft claim:** `UnknownReferenceName { kind: "output", name: "input.missing" }`
**Correction:** The error includes both `kind: "output"`, `reference: "$input.missing"`,
and `name: "input.missing"`. The `kind` remains "output" because the resolver treats
input references as output name resolution.

## Preserved Correct Elements

The following elements of the draft contract remain correct:
- The overall domain problem (input refs only work in Finish)
- The use of existing compilation infrastructure (no new maps/allocators)
- The O(input_count) memory bound
- The collision detection requirement
- The Finish bridge preservation requirement

## Verification

The corrected contract was validated against actual v18 source code:
- `crates/vb_compile/src/mod_compile_lowering/part_01.rs` (allocation timing)
- `crates/vb_compile/src/mod_compile_lowering/canonical_references.rs` (resolver logic)
- `crates/vb_compile/src/mod_compile_lowering/finish_inputs.rs` (Finish bridge)
- `crates/vb_compile/src/mod_compile_lowering/input_schema.rs` (schema types)
- `crates/vb_compile/src/mod_compile_lowering/canonical_types.rs` (slot_types map)
