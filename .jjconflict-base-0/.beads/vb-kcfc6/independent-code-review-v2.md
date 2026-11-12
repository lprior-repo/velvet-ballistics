**Findings:**

1. **Unused error variant:** `SlotIndexOutOfRange` is defined in `InputMappingError` but never synthesized by `map_runtime_inputs`. It's a legacy variant preserved for API compatibility. Consider documenting this explicitly or removing if no external consumers depend on it.

2. **Future variant fragility:** The kind-mapping match uses `_ => DecodeFailed` as catch-all. Any new `SlotValue` variant added to `vb_core` would cause runtime decode failures rather than compile errors. An exhaustive match (no catch-all) would surface this at compile time, but the current design intentionally treats unknown variants as decode failures per the contract.

3. **Test message coupling:** CLI integration tests assert exact error message strings. This provides strong regression protection but creates maintenance coupling if diagnostic wording changes.

**Evidence:**
- 16 unit tests pass, covering all error paths and precedence ordering
- 2 mutation tests killed by named precedence tests (`trailing_bytes_precede_excess_count`, `trailing_bytes_precede_kind_mismatch`)
- 1 CLI integration test with 10 subcases (5 payloads × 2 commands) passes
- Property test covers arbitrary I64 values with 1..16 trailing junk bytes
- `take_from_bytes` correctly detects trailing bytes (addresses postcard's default ignore behavior)
- Decode-before-count/kind ordering verified by mutation testing

**Limitations:**
- Property test bounded by Postcard's I64 encoding (not true arbitrary I64)
- No fuzzing of malformed Postcard structures beyond specific test cases
- Non-core value validation (List/Object/Blob internals) deferred to parent scope per contract
- No performance or large-input testing

**STATUS:** APPROVED