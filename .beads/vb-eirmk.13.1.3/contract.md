# Contract: vb-eirmk.13.1.3 - Fix record kind test classification

## Goal
Repair `unknown_record_kind_rejection::is_known_record_kind_returns_false_for_invalid_kinds` in `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` to match actual RecordKind IDs (GateOpened=48, TenantAdmission=49 now known kinds).

## Domain Facts (independent of production from_id)
Actual known RecordKind IDs from `crates/vb_storage/src/records/kind.rs`:
- IDs 1, 2, 3, 4 (WorkflowSource, CompiledIr, RunHeader, RunInputs)
- IDs 10..50 (RunAccepted through IndexUpdate) with no internal gaps
- Total: 45 known IDs; unknown IDs are all others in u16::MIN..=u16::MAX

The current known domain is 1..4 and 10..50 (no 0, no 5..9). This is maintained as an independent canonical array in the test, not derived via production from_id.

## Test Requirements
1. Enumerate explicit array of all 45 known RecordKind IDs independently (NOT via production from_id)
2. Iterate full u16 domain (0..=65535) to verify:
   - Known IDs → exact family variant (not Unknown)
   - Unknown IDs → UnknownRecordKind{kind} with exact echoed ID
3. Positive decode tests (decode_accepts_all_known_journal_event_kinds) must include 48/49
4. Explicit positive record_kind value checks for kinds 38, 48, 49
5. Retain all original test names, structure, and non-affected assertions

## Constraints
- Only edit `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs`
- No production code changes
- No skipped cases, deletions, suppressions
- No panic/unwrap/expect/unchecked arithmetic/casts/indexes
- No new dependencies
- Minimal test logic: each helper <= 25 logic lines
- Use assert!(matches!(...)) for exact matches, no assert_eq! in domain loop

## Acceptance
`timeout 300 rustup run nightly-2026-04-28 cargo nextest run -p velvet-ballistics-workspace-tests --all-features --test vb_eepg_bdd_tests` exits 0.
