# Bead vb-kbjwj: Test Strengthening — Policy Digests & Records

## Scope

Fix and strengthen tests in these NEW test files only:
- `crates/vb_core/src/policy/new_tests.rs`
- `crates/vb_runtime/src/policy/tests.rs`
- `crates/vb_runtime/src/policy/ticket_field_variation_tests.rs`
- `crates/vb_runtime/src/policy/taint_tests.rs`
- `crates/vb_runtime/src/policy/emergency_and_admission_tests.rs`

## Issues to Fix

### core new_tests.rs
1. `policy_digest_roundtrip`: unchecked indexing `digest.as_bytes()[0]`/`[31]` → compare whole expected array via helper
2. `decision_code_discriminants`: `as u8` enum repr casts aren't serde proof → test actual enum variant matching with stable encoding
3. `policy_layer_discriminants`: same `as u8` cast issue → test actual enum variant matching
4. `decision_record_preserves_all_fields`: tautology `assert_eq!(record.ticket, record.ticket)` → compare to original ticket
5. `influence_state_values`: tautology `assert_eq!(InfluenceState::NotEvaluated, InfluenceState::NotEvaluated)` → remove

### runtime tests.rs
1. `decision_record_preserves_ticket_identity`: tautology `assert_eq!(record.ticket, record.ticket)` → compare to original
2. `decision_record_preserves_policy_identities`: only checks admission, not live digest → add assertions for both admission and configured live digest

### runtime ticket_field_variation_tests.rs
1. `make_ticket` has 8 params (violates <=5) → use baseline struct update instead

### runtime taint_tests.rs
1. `make_ticket` has 8 params (violates <=5) → use baseline struct update instead
2. `all_taint_variants_produce_distinct_digests`: unchecked indexing and arithmetic `digests[i]/[j]`, `i+1` → use iterator combinations

### runtime emergency_and_admission_tests.rs
1. `make_ticket` has 8 params (violates <=5) → use baseline struct update instead

## Constraints

- TESTS ONLY: no production code changes
- Original `original_tests.rs` and original 7 bodies untouched
- Preserve or strengthen existing test names/cases/assertions
- No deletion of existing test cases or assertions
- Use `?` for error propagation, `assert_eq` against expected values
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`
- Functions <= 25 logical lines, files <= 300 logical lines

## Verification Gates

1. `cargo test -p vb_core -p vb_runtime --lib -- policy --test-threads=1`
2. `cargo clippy -p vb_core -p vb_runtime --lib -- -D warnings`
3. `cargo fmt -p vb_core -p vb_runtime --check`
