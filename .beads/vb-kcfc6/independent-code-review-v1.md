**STATUS: APPROVED**

**Findings:**

1. **Decoder boundary correct**: `map_runtime_inputs` uses `postcard::take_from_bytes` with explicit remainder check, satisfying the "exactly one Postcard message" requirement and detecting trailing bytes as `DecodeFailed`.

2. **Error precedence correct**: Decode → count → kind checks occur in the required order. Trailing byte detection happens before slot count/kind validation, matching the contract's "DecodeFailed before count/kind" requirement.

3. **Zero-ports handling correct**: Empty input (raw or encoded) returns empty mapping for both zero-port and multi-port workflows. Excess values correctly yield `SlotCountExceeded`.

4. **Positional mapping correct**: Values are zipped positionally with declared slots. Partial prefixes map only supplied values; unsupplied slots are omitted without synthesis.

5. **Kind validation correct**: Value kind derivation matches all 8 core variants (Null, Bool, I64, F64, Symbol, List, Object, Blob). Non-core variants fall through to `DecodeFailed`. `KindMismatch` includes slot index and expected kind.

6. **CLI exit codes preserved**: `cmd_run` returns exit 1 (RuntimeFailed) for mapping errors; `cmd_run_compiled` returns exit 3 (CompileFailed). Error messages use existing constants, preserving exact diagnostics.

7. **vb-iwf1d fixture correct**: Single-line addition of `result: vb_core::SlotIdx::ZERO` to the completed-run incident test fixture. No analyzer logic changes.

8. **Test coverage adequate**: 13 unit tests cover all acceptance criteria cases (zero-ports, with-ports, malformed, trailing bytes, kind mismatch, excess values). Integration test covers 5 payloads × 2 commands = 10 subcases with exact exit code and message assertions.

9. **No scope creep**: No new required-input validation, no handle validity checks, no durable submit changes. Non-core value internal validation remains parent-scope concern.

No actionable in-scope findings.