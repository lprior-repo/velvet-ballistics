# Rust Contract: Repair WorkflowParts Fixture Initializers

## Scope
- **Target**: Core unit test fixtures missing newly required `input_slots` field
- **Diagnostic**: Compile and inventory all 115 E0063 (missing field) errors
- **Constraint**: Fix only reported initializers; do not modify production code

## Repair Rules
1. **Self-contained fixtures** (no external ingress): Add empty boxed declaration `input_slots: Box::new([])` after verifying fixture semantics
2. **Intentionally external fixtures**: Declare actual slots/kinds appropriate to the fixture; do not blindly empty
3. **Preservation**: Maintain every existing assertion and production behavior unchanged

## Execution
- **Compile**: `cargo test -p vb_core --lib --no-run --message-format=json` scoped to corelib only
- **Inventory**: Record exact rustc primary spans and containment files for all 115 errors
- **Fix**: Update only the reported fixture initializers per repair rules
- **Verify**: Compile and run corelib tests; execute exact scoped Miri IDs from previous CI

## Reporting
- No proof closure claimed
- Miri setup failure must be reported, not waived
- Production Rust domain remains unchanged
- Input ports schema feature reviewed separately
- Code and tests reviewed by independent local models
Coordinator command clarification: --no-run is cargo-test syntax; rustc is the diagnostic producer.
