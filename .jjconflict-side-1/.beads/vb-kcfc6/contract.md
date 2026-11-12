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
- `SlotIndexOutOfRange`: Existing variant remains unchanged and is not synthesized by this positional mapping repair.

**Boundaries:**
- Exactly one Postcard message per invocation; no indexed addressing, no default value generation, no runtime I/O.
- Empty input always yields empty mapping (requiredness is runtime admission, not decoder concern).
- Durability handling, non-core value validation, and durable submit persistence are unchanged parent-scope responsibilities.

**Hazards:**
- `postcard::from_bytes` ignores trailing bytes; must use `take_from_bytes` to detect remainder.
- Decode must succeed before count/kind checks; otherwise partial decode leads to misattribution.
- Current8SlotValue variants retain existing kind mapping; unsupported future shapes return DecodeFailed.


## Acceptance and scope clarification

Zeroports: rawempty/encodedempty =>Okempty; malformed255/encodedempty+trailingbyte =>DecodeFailed; encodedI64=>SlotCountExceeded. Withports: rawempty=>Okempty, nonzero declaredslot2 maps correctly, partialprefix and2valueorder preserved, Bool for I64port=>KindMismatch{slot:2,expected_kind:I64}, excess=>SlotCountExceeded, validpayload+trailingbyte=>DecodeFailed before count/kind. Actual implementation uses postcard::take_from_bytes with remainder rejection; no synthesized missing inputs or indexed wire. Handle ownership and requiredness remain runtime admission; durable submit remains parentvb-ezymz. No claimed proof planning.

Existing CLIstatus is preserved: cmd_run mappingfailureexit1, cmd_run_compiled mappingfailureexit3. Exact existing diagnostics preserved in both commands. CLIstdout reports exact summary/terminalstatus, not result42. Fivepayloads x2commands=10subcases in1Rustintegrationtest, plus13functionunit tests. No broaderCI/v1closure.
