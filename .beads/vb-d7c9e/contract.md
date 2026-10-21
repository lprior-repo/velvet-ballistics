# Contract: kind_validation lint-only assert normalization

**File:** `fuzz/fuzz_targets/kind_validation.rs`

**Change:** Replace two `assert_eq!(expr, true, msg)` with `assert!(expr, msg)`.

**Equivalence:** For boolean `expr`, `assert_eq!(expr, true, msg)` and `assert!(expr, msg)` are behaviorally identical — both panic with `msg` when `expr` is false. No assertion weakening.

**Preserved:**
- Same boolean expressions evaluated
- Same panic messages
- Two-way oracle: fuzz input → `RecordKind` conversion → `is_known_record_kind` validation
- Deterministic recordkind replay test
- Bounded fuzz smoke test

**Not changed:** Schema, production code, test coverage, gate status.

**Acceptance:** Execute the existing deterministic oracle replay command and bounded fuzz smoke; no command has run for this change yet. The generated draft invented a kind_validation test target and future pass; corrected before implementation. Assertion truth condition and custom messages remain, standard assert macro diagnostic formatting may differ.