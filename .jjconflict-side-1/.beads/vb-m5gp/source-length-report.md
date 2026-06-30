# Source Length Report

STATUS: PASS

Exact LEN-001 command:

```text
{'crates/vb_compile/src/lib.rs': 61, 'crates/vb_compile/src/mod_compile_core.rs': 230, 'crates/vb_compile/src/mod_compile_errors.rs': 12, 'crates/vb_compile/src/mod_compile_validation.rs': 17, 'crates/vb_compile/src/mod_compile_lowering.rs': 42}
```

Canonical recursive gate `bash scripts/check-source-length.sh`: PASS with DEFERRED_GLOBAL notices only for pre-existing unrelated files:

- `crates/vb_compile/src/expression_bytecode.rs`: 2242 physical lines.
- `crates/vb_compile/src/expression.rs`: 881 physical lines.
- `crates/vb_compile/src/references.rs`: 342 physical lines.
- `crates/vb_compile/src/schema.rs`: 729 physical lines.
- `crates/vb_compile/src/type_taint.rs`: 511 physical lines.

All bead-local `lib.rs` and `mod_compile_*` split sources remain below the `<300` line threshold.
