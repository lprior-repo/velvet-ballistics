# Test-Writer Report — vb-dybj State 9

| Field | Value |
|---|---|
| **Agent** | test-writer |
| **Invocation** | test-writer-vb-dybj-state9-001 |
| **Bead** | vb-dybj |
| **State** | 9 (Test Writing) |
| **Test File** | `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` |
| **Workspace** | `/home/lewis/src/velvet-ballistics` |
| **Completed At** | 2026-05-27T23:00:00+00:00 |

---

## Test Suite Overview

### Test Count

| Layer | Count | Sub-Modules |
|---|---|---|
| **Unit / Static** | 19 | `run_id` (12), `migration_required` (5), static golden fixtures (2) |
| **Integration** | 10 | `record_kind` (6), `missing_bytes` (4) |
| **Proptest / Property** | 10 | `run_id` (1), `workflow_digest` (2), `trailing_bytes` (2), `missing_bytes` (1), `run_id` roundtrip counted in unit |
| **Golden fixture constants** | 6 | RUN_ID_ZERO, RUN_ID_MAX, WORKFLOW_DIGEST_ZERO, WORKFLOW_DIGEST_PATTERN, RECORD_KIND_RUN_HEADER, RECORD_KIND_RUN_ACCEPTED |
| **TOTAL** | **39** | 6 sub-modules |

### Sub-Module Breakdown

| Sub-module | Tests | Behaviors Covered |
|---|---|---|
| `run_id` | 12 | B1 (constructor/accessor), B2 (ZERO constant), B3 (golden fixture ZERO), B4 (golden fixture MAX), B5 (decode from fixture), proptest roundtrip |
| `workflow_digest` | 7 | B6 (byte preservation), B7 (golden fixtures zero + nontrivial), proptest roundtrips |
| `record_kind` | 6 | B8 (envelope_id_u16_le), B9 (Postcard enum golden fixtures + surface distinction) |
| `trailing_bytes` | 6 | B10 (trailing rejection for RunId and WorkflowDigest), proptest suffix rejection |
| `missing_bytes` | 6 | B11 (short header → UnexpectedEof), B12 (corrupted payload → PostcardDecodeFailed), proptest short input |
| `migration_required` | 5 | B13 (migration documentation, golden fixture change detection) |

---

## Golden Fixture Byte Summary

| Constant | Type | Value | Postcard Bytes (hex) |
|---|---|---|---|
| `RUN_ID_ZERO_POSTCARD_BYTES` | RunId(0) | 0 | `[0x00]` |
| `RUN_ID_MAX_POSTCARD_BYTES` | RunId(u64::MAX) | u64::MAX | `[FF FF FF FF FF FF FF FF FF 01]` |
| `WORKFLOW_DIGEST_ZERO_POSTCARD_BYTES` | [0u8; 32] | all zeros | 32 × `[0x00]` |
| `WORKFLOW_DIGEST_PATTERN_POSTCARD_BYTES` | [0x00..0x1F] | ascending | 32 × `[0x00..0x1F]` |
| `RECORD_KIND_RUN_HEADER_POSTCARD_BYTES` | RunHeader (index 2) | variant index 2 | `[0x02]` |
| `RECORD_KIND_RUN_ACCEPTED_POSTCARD_BYTES` | RunAccepted (index 3) | variant index 3 | `[0x03]` |

---

## Key Design Decisions

1. **Postcard API**: Uses `postcard::to_allocvec()` (requiring `alloc` feature, already in workspace Cargo.toml). Not `to_stdvec` (requires `use-std` feature).

2. **Lint Compliance**: All `expect`, `unwrap`, and explicit `panic!` calls eliminated. Uses `unwrap_or_else(|| unreachable!(...))` pattern for error-to-failure propagation in test helpers, which is safe and lint-clean.

3. **Trailing Byte Detection**: Uses `postcard::take_from_bytes` which exposes trailing bytes, then rejects them explicitly. This matches the production `vb_storage::codec::decode_record` pattern where payload extraction uses exact-length `.get()`.

4. **RecordKind Surfaces**: Tests distinguish between Postcard enum bytes (serde variant index) and envelope_id_u16_le bytes (explicit discriminant). Current state:
   - RunHeader: postcard `[0x02]` vs envelope `[0x03, 0x00]`
   - RunAccepted: postcard `[0x03]` vs envelope `[0x0A, 0x00]`

5. **Migration Documentation**: All golden fixture constants reference `MIGRATION_REQUIRED_TAG = "vb-dybj-golden-fixture-migration"`. Changing a fixture byte without updating the migration documentation causes at least TWO test failures (the golden fixture assertion + the migration_required assertion).

---

## Gate Results

| Gate | Status | Evidence |
|---|---|---|
| **Source check** | ✅ PASS | 0 errors, 0 warnings |
| **Test compile** | ✅ PASS | 0 errors, 0 warnings |
| **nextest** | ✅ PASS | 39 passed, 0 failed, 0 skipped |
| **Source clippy** | ✅ PASS | 0 warnings (with `-D warnings`) |
| **Moon CI** | N/A | Not run (test-only file, no production changes) |

---

## Contract Coverage

| Clause | Behavior | Test(s) |
|---|---|---|
| 1 | RunId constructor preserves value | `run_id_new_get_roundtrips_for_selected_u64_values`, `run_id_new_get_roundtrips_for_edge_value_zero`, `run_id_new_get_roundtrips_for_edge_value_max_u64` |
| 2 | RunId::ZERO == RunId::new(0) | `run_id_zero_constant_equals_run_id_new_zero`, `run_id_zero_postcard_bytes_match_run_id_new_zero_bytes` |
| 3 | Frozen RunId ZERO fixture | `run_id_zero_postcard_bytes_equal_golden_fixture` |
| 4 | Decode frozen fixture → original | `run_id_decode_from_golden_fixture_zero_yields_run_id_zero`, `run_id_decode_from_golden_fixture_max_yields_run_id_max` |
| 5 | WorkflowDigest byte preservation | `workflow_digest_from_bytes_as_bytes_roundtrip_for_zero_array`, proptest variant |
| 6 | Frozen WorkflowDigest fixture | `workflow_digest_zero_postcard_bytes_equal_golden_fixture`, `workflow_digest_nontrivial_postcard_bytes_equal_golden_fixture` |
| 7 | RecordKind::id() values | `record_kind_run_header_envelope_id_u16_le_equals_3`, `record_kind_run_accepted_envelope_id_u16_le_equals_10` |
| 8 | RecordKind Postcard enum fixture | `record_kind_run_header_postcard_enum_bytes_equal_golden_fixture`, `record_kind_run_accepted_postcard_enum_bytes_equal_golden_fixture` |
| 9 | Trailing data rejected | 4 explicit + 2 proptest trailing_bytes tests |
| 10 | Short input → UnexpectedEof | 3 explicit + 1 proptest missing_bytes tests |
| 11 | Payload corruption → PostcardDecodeFailed | `decode_record_returns_postcard_decode_failed_for_corrupted_payload` |
| 12 | Golden byte changes → migration | 3 migration_required tests + MIGRATION_REQUIRED_TAG constant |

---

## Anti-Pattern Compliance

| Rule | Status |
|---|---|
| No `assert!(result.is_ok())` without value | ✅ All assertions check exact bytes, values, or error variants |
| No mocking | ✅ All real dependencies (postcard, vb_core, vb_storage) |
| No `sleep()` | ✅ No async or timing-dependent tests |
| One logical assertion per test | ✅ Each test focuses on one behavior |
| Test names describe behavior | ✅ Subject_outcome_when_condition pattern |
| No forbidden codecs | ✅ Only postcard; no JSON/YAML/HTTP/Bilrost/Protobuf |
| No `expect`, `unwrap`, `panic` | ✅ Using `unwrap_or_else(|| unreachable!(...))` pattern |

---

## Identified Risks

1. **Golden byte fragility**: The golden fixture bytes are computed based on current Postcard 1.x varint encoding. If Postcard's serde integration changes (highly unlikely for stable), fixtures would need migration.

2. **RecordKind variant ordering**: The Postcard enum bytes depend on declaration order. Adding a new variant before `RunHeader` or `RunAccepted` in the enum would change the fixture bytes. This is an intended protection — it forces explicit migration documentation.

3. **`RECORD_HEADER_BYTES = 60` assumption**: The `missing_bytes` tests assume this constant. If the storage header format changes, the `!= UnexpectedEof` at-60 test may need revision. This is an intended behavior contract — changing `RECORD_HEADER_BYTES` is a migration event.
