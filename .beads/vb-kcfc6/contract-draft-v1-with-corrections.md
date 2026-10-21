**Domain:** CLI input decoder for compiled workflow execution. Decodes raw positional binary into `SlotValue` sequence, validates against declared input slots, emits mapped `(slot, value)` pairs. Non-core values (`List`, `Object`, `Blob`) are accepted; their internal structural validation is a separate parent-scope concern (vb-ezymz).

**Value Types:**
- `input_data`: `&[u8]` — raw positional input payload
- `compiled.input_slots()`: declared input slot descriptors, each with `slot: SlotIdx` and `kind: InputSlotKind`
- `SlotValue`: Null, Bool, I64, F64, Symbol, List, Object, Blob, and other core variants
- `InputMappingError`: DecodeFailed, SlotCountExceeded, SlotIndexOutOfRange, KindMismatch{slot, expected_kind}

**Workflow:**
1. Decode `input_data` as Postcard `Box<[SlotValue]>`. Empty input yields an empty list. Decoder error or trailing unused bytes yields `DecodeFailed`.
2. If decoded value count exceeds declared input slot count, return `SlotCountExceeded`.
3. Positionally map each supplied value to its corresponding declared slot:
   - Derive value kind (Null/Bool/I64/F64/Symbol/List/Object/Blob; other core variants yield `DecodeFailed`).
   - If value kind != slot's declared kind, return `KindMismatch{slot, expected_kind}`.
4. Return boxed slice of `(SlotIdx, SlotValue)` pairs. Unsupplied slots are omitted; missing values are not invented.

**Errors:**
- `DecodeFailed`: Postcard decode error, non-core value variants, or trailing unused bytes after decoding.
- `SlotCountExceeded`: More supplied values than declared input slots.
- `KindMismatch{slot, expected_kind}`: Supplied value kind does not match declared slot kind.
- `SlotIndexOutOfRange`: Reserved for indexed addressing; not synthesized in positional wire.

**Boundaries:**
- Exactly one Postcard message per invocation; no indexed addressing, no default value generation, no runtime I/O.
- Empty input always yields empty mapping (requiredness is runtime admission, not decoder concern).
- Durability handling, non-core value validation, and durable submit persistence are unchanged parent-scope responsibilities.

**Hazards:**
- `postcard::from_bytes` ignores trailing bytes; must use `take_from_bytes` to detect remainder.
- Decode must succeed before count/kind checks; otherwise partial decode leads to misattribution.
- Non-core variants (e.g., `Bytes`, `Tagged`) must be rejected rather than silently mapped.

**Concrete Acceptance Assertions:**
- `map_runtime_inputs(compiled_0_slots, [])` => `Ok([])`
- `map_runtime_inputs(compiled_2_slots, [])` => `Ok([])`
- `map_runtime_inputs(compiled_1_slot_i64, postcard([SlotValue::I64(42)]))` => `Ok([(slot_0, I64(42))])`
- `map_runtime_inputs(compiled_1_slot_i64, postcard([SlotValue::I64(1), SlotValue::I64(2)]))` => `Err(SlotCountExceeded)`
- `map_runtime_inputs(compiled_1_slot_i64, postcard([SlotValue::Bool(true)]))` => `Err(KindMismatch{slot_0, expected_kind: I64})`
- `map_runtime_inputs(compiled_1_slot, postcard([SlotValue::I64(1)]) + [0xff, 0xfe])` => `Err(DecodeFailed)`
- `map_runtime_inputs(compiled_1_slot_i64, [0xde, 0xad])` => `Err(DecodeFailed)`
- `map_runtime_inputs(compiled_1_slot, "")` => `Ok([])`
- `map_runtime_inputs(compiled_2_slots, postcard([SlotValue::I64(1)]))` => `Ok([(slot_0, I64(1))])`
- `map_runtime_inputs(compiled_0_slots, postcard([SlotValue::I64(1)]))` => `Err(SlotCountExceeded)`
- `map_runtime_inputs(compiled_1_slot_i64, postcard([SlotValue::Null]))` => `Err(KindMismatch{slot_0, expected_kind: I64})`
- `map_runtime_inputs(compiled_1_slot_null, postcard([SlotValue::Null]))` => `Ok([(slot_0, Null)])`
- `map_runtime_inputs(compiled_1_slot_blob, postcard([SlotValue::Bytes([1,2,3])]))` => `Err(DecodeFailed)`
- `map_runtime_inputs(compiled_1_slot_blob, postcard([SlotValue::Blob([1,2,3])]))` => `Ok([(slot_0, Blob([1,2,3]))])`

## Inspected application correction

The first local5090contract draft is retained above. It invented Bytes/Tagged examples and Blob array constructors; those examples are REJECTED and not acceptance requirements. Actual current SlotValue has the8existing variants inspected in value.rs; Blob/List/Object carry handles. Do not implement invented variants or assign new semantics to existing SlotIndexOutOfRange, whose branch use is unchanged. Explicit zero-port cases required: empty raw and postcard encoded empty =>empty mapping; malformed nonempty or trailing junk=>DecodeFailed; encoded1value=>SlotCountExceeded. Requiredness, backing handle validity, submit durable persistence remain separate parent scope. Exactly one decoded postcard message before count/kind validation; use take_from_bytes and check remainder.

## Existing public CLI error status

Current cmd_run maps InputMappingError to RuntimeFailed exit1; cmd_run_compiled maps it to CompileFailed exit3. This bounded mapper patch preserves those existing mappings and checks exact error text in both paths. The test-writer prompt initially assumed3forboth, corrected against actualrun.rs/run_compiled.rs; no error-path assertions are weakened to generic nonzero.
