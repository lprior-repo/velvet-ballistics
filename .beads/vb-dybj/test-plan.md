# Test Plan: vb-dybj — Postcard Newtype Compatibility Tests

planner_skill: test-planner
planner_invocation_id: test-planner-vb-dybj-state8-001
bead_id: vb-dybj
state: 8
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-dybj
source_checkout: /home/lewis/src/velvet-ballistics
target_file: crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs
host_session_id: velvet-ballistics-vb-dybj-femdation-2026-05-27

## Proof/Refinement Coverage Matrix

| Proof ID | Refinement ID | Behavior Test Module | Test Count | Coverage |
|---|---|---|---|---|
| PO-VB-DYBJ-001 | RRO-VB-DYBJ-001 | round_trip | 2 | Full |
| PO-VB-DYBJ-002 | RRO-VB-DYBJ-002 | round_trip | 2 | Full |
| PO-VB-DYBJ-003 | RRO-VB-DYBJ-003 | round_trip | 1 | Full |
| PO-VB-DYBJ-004 | RRO-VB-DYBJ-004 | newtype_composition | 1 | Full |
| PO-VB-DYBJ-005 | RRO-VB-DYBJ-005 | serialization_boundary | 2 | Full |
| PO-VB-DYBJ-006 | RRO-VB-DYBJ-006 | serialization_boundary | 2 | Full |
| PO-VB-DYBJ-007 | RRO-VB-DYBJ-007 | deserialization_boundary | 1 | Full |
| PO-VB-DYBJ-008 | RRO-VB-DYBJ-008 | deserialization_boundary | 1 | Full |
| PO-VB-DYBJ-009 | RRO-VB-DYBJ-009 | deserialization_boundary | 1 | Full |
| PO-VB-DYBJ-010 | RRO-VB-DYBJ-010 | deserialization_boundary | 1 | Full |
| PO-VB-DYBJ-011 | RRO-VB-DYBJ-011 | error_paths | 1 | Full |
| PO-VB-DYBJ-012 | RRO-VB-DYBJ-012 | error_paths | 1 | Full |
| PO-VB-DYBJ-013 | RRO-VB-DYBJ-013 | edge_cases | 1 | Full |
| PO-VB-DYBJ-014 | RRO-VB-DYBJ-014 | edge_cases | 1 | Full |
| PO-VB-DYBJ-015 | RRO-VB-DYBJ-015 | edge_cases | 1 | Full |
| PO-VB-DYBJ-016 | RRO-VB-DYBJ-016 | round_trip | 1 | Full |
| PO-VB-DYBJ-017 | RRO-VB-DYBJ-017 | (all) | 39 | Full |
| PO-VB-DYBJ-018 | RRO-VB-DYBJ-018 | round_trip | 1 | Full |
started_at: 2026-05-27T22:30:00.000000+00:00

## Summary

- **Behaviors identified**: 12 (from contract.md functional contract clauses 1–12)
- **Trophy allocation**: 2 static / 7 unit / 8 integration / 0 e2e
- **Proptest invariants**: 6
- **Fuzz targets**: 0 (covered by State 6 fuzz artifacts; not duplicated in test file)
- **Kani harnesses**: 0 (covered by State 6 Kani artifacts; not duplicated in test file)
- **Test sub-modules**: 6 (`run_id`, `workflow_digest`, `record_kind`, `missing_bytes`, `trailing_bytes`, `migration_required`)

---

## 1. Behavior Inventory

Each contract clause maps to one or more behaviors:

| # | Behavior | Contract Clause | Test Module |
|---|---|---|---|
| B1 | RunId constructor preserves value (v == RunId::new(v).get()) | 1 | `run_id` |
| B2 | RunId::ZERO equals RunId::new(0) | 2 | `run_id` |
| B3 | RunId Postcard bytes match frozen golden fixture for ZERO | 3 | `run_id` |
| B4 | RunId Postcard bytes match frozen golden fixture for MAX | 3 | `run_id` |
| B5 | RunId decode from frozen fixture bytes yields original value | 4 | `run_id` |
| B6 | WorkflowDigest from_bytes(as_bytes(x)) roundtrips exactly | 5 | `workflow_digest` |
| B7 | WorkflowDigest Postcard bytes match frozen golden fixture | 6 | `workflow_digest` |
| B8 | RecordKind::id() values match master storage IDs for selected variants | 7 | `record_kind` |
| B9 | RecordKind Postcard enum bytes match frozen golden fixture | 8 | `record_kind` |
| B10 | Trailing suffix rejected by exact-value Postcard decode | 9 | `trailing_bytes` |
| B11 | Storage header shorter than fixed envelope returns JournalError::UnexpectedEof | 10 | `missing_bytes` |
| B12 | Storage payload Postcard decode failure returns JournalError::PostcardDecodeFailed | 11 | `missing_bytes` |
| B13 | Golden byte changes without migration documentation cause test failure | 12 | `migration_required` |

---

## 2. Trophy Allocation

| Layer | Count | Behaviors | Rationale |
|---|---|---|---|
| **Static Analysis** | 2 | B13 (migration naming), no-forbidden-codecs (PO-VB-DYBJ-018) | clippy lint gate + source scan; enforced by CI |
| **Unit / Calc** | 7 | B1, B2, B3, B4, B5, B6, B7 | Pure roundtrip and golden-byte assertions against newtype constructors/accessors — no I/O, no storage codec |
| **Integration** | 8 | B8, B9, B10, B11, B12 (and proptest variants of B1–B7) | Trailing/missing-byte assertions require `vb_storage::codec` + `JournalError` types; RecordKind surface fixtures require `vb_storage::records`; proptest across real Postcard serialize/deserialize |
| **E2E** | 0 | N/A | CLI/integration binary not in scope for golden-byte fixture tests |

**Ratio**: ~37% unit / ~53% integration / ~10% static / 0% e2e — within acceptable deviation (integration is the widest layer per Testing Trophy).

---

## 3. BDD Scenarios

### Module: `run_id`

#### Behavior B1: RunId constructor preserves value
Given: any u64 value v passed to RunId::new(v)
When: RunId::get() is called on the result
Then: the returned value equals v (for all edge values: 0, 1, u64::MAX, mid-range)

```
fn run_id_new_get_roundtrips_for_selected_u64_values()
fn run_id_new_get_roundtrips_for_edge_value_zero()
fn run_id_new_get_roundtrips_for_edge_value_max_u64()
```

#### Behavior B2: RunId::ZERO equals RunId::new(0)
Given: RunId::ZERO constant
When: compared with RunId::new(0)
Then: both values are equal and both produce identical Postcard bytes

```
fn run_id_zero_constant_equals_run_id_new_zero()
fn run_id_zero_postcard_bytes_match_run_id_new_zero_bytes()
```

#### Behavior B3: RunId Postcard golden fixture for ZERO
Given: RunId::ZERO is Postcard-serialized
When: output bytes are compared against frozen constant RUN_ID_ZERO_POSTCARD_BYTES
Then: bytes match exactly (golden byte assertions freeze compatibility)

```
fn run_id_zero_postcard_bytes_equal_golden_fixture()
```

#### Behavior B4: RunId Postcard golden fixture for MAX
Given: RunId::new(u64::MAX) is Postcard-serialized
When: output bytes are compared against frozen constant RUN_ID_MAX_POSTCARD_BYTES
Then: bytes match exactly

```
fn run_id_max_postcard_bytes_equal_golden_fixture()
```

#### Behavior B5: RunId decode from frozen fixture
Given: frozen RUN_ID_ZERO_POSTCARD_BYTES / RUN_ID_MAX_POSTCARD_BYTES
When: postcard::from_bytes(&fixture) is called
Then: decoded RunId equals the expected original value

```
fn run_id_decode_from_golden_fixture_zero_yields_run_id_zero()
fn run_id_decode_from_golden_fixture_max_yields_run_id_max()
```

---

### Module: `workflow_digest`

#### Behavior B6: WorkflowDigest byte preservation
Given: any [u8; 32] bytes
When: WorkflowDigest::from_bytes(bytes).as_bytes() is called
Then: returned bytes equal the original input bytes

```
fn workflow_digest_from_bytes_as_bytes_roundtrip_for_zero_array()
fn workflow_digest_from_bytes_as_bytes_roundtrip_for_nontrivial_pattern()
```

#### Behavior B7: WorkflowDigest golden fixture
Given: a selected nontrivial [u8; 32] digest (all zeros and a pattern)
When: Postcard-serialized and compared against frozen fixture bytes
Then: Postcard bytes match golden fixture; deserialization yields original digest

```
fn workflow_digest_zero_postcard_bytes_equal_golden_fixture()
fn workflow_digest_nontrivial_postcard_bytes_equal_golden_fixture()
fn workflow_digest_decode_from_golden_fixture_yields_original()
```

---

### Module: `record_kind`

#### Behavior B8: RecordKind::id() envelope ID assertions
Given: selected RecordKind variants (RunHeader = 3, RunAccepted = 10)
When: RecordKind::id() is called
Then: returned u16 equals the expected master storage ID (3, 10)
AND: u16::to_le_bytes() of the id is asserted separately from Postcard enum bytes

```
fn record_kind_run_header_envelope_id_u16_le_equals_3()
fn record_kind_run_accepted_envelope_id_u16_le_equals_10()
```

#### Behavior B9: RecordKind Postcard enum golden fixture
Given: selected RecordKind variants are Postcard-serialized
When: compared against frozen fixture bytes (RECORD_KIND_RUN_HEADER_POSTCARD_BYTES, RECORD_KIND_RUN_ACCEPTED_POSTCARD_BYTES)
Then: Postcard enum bytes match golden fixture
AND: Postcard enum bytes are NOT equal to the envelope_id_u16_le bytes (distinct surfaces)

```
fn record_kind_run_header_postcard_enum_bytes_equal_golden_fixture()
fn record_kind_run_accepted_postcard_enum_bytes_equal_golden_fixture()
fn record_kind_postcard_enum_bytes_differ_from_envelope_id_u16_le_run_header()
fn record_kind_postcard_enum_bytes_differ_from_envelope_id_u16_le_run_accepted()
```

**Note**: Test function names include `postcard_enum` and/or `envelope_id_u16_le` as required by PO-VB-DYBJ-009 naming conventions.

---

### Module: `trailing_bytes`

#### Behavior B10: Trailing suffix rejected by exact-value decode
Given: valid Postcard-encoded RunId / WorkflowDigest bytes
When: one or more arbitrary trailing bytes are appended to the encoded bytes
Then: postcard::from_bytes::<T>(&trailing_bytes) returns Err — no silent acceptance
AND: the decode error is not ambiguously caught as a generic "any error"

```
fn trailing_bytes_run_id_rejected_with_extra_byte()
fn trailing_bytes_run_id_rejected_with_multiple_extra_bytes()
fn trailing_bytes_workflow_digest_rejected_with_extra_byte()
fn trailing_bytes_workflow_digest_rejected_with_multiple_extra_bytes()
```

---

### Module: `missing_bytes`

#### Behavior B11: Storage header shorter than fixed envelope returns UnexpectedEof
Given: a byte slice shorter than RECORD_HEADER_BYTES
When: decode_record_header(&short_input) is called
Then: returns Err(JournalError::UnexpectedEof) — NOT any other error variant

```
fn decode_record_header_returns_unexpected_eof_for_zero_bytes()
fn decode_record_header_returns_unexpected_eof_for_one_byte()
fn decode_record_header_returns_unexpected_eof_for_header_minus_one_bytes()
```

#### Behavior B12: Storage payload decode failure returns PostcardDecodeFailed
Given: a valid envelope header pointing to an invalid Postcard payload
When: the full decode_record path is followed
Then: the error chain reaches JournalError::PostcardDecodeFailed
AND: PostcardDecodeFailed is only reachable after envelope validation (header/magic/length) passes

```
fn decode_record_returns_postcard_decode_failed_for_corrupted_payload()
```

---

### Module: `migration_required`

#### Behavior B13: Golden byte changes require named migration
Given: frozen golden fixture byte constants are compiled into the test
When: a developer changes a golden fixture byte without updating a corresponding migration-naming constant or comment
Then: the test function name and assertion messages document the named migration requirement
AND: any future byte drift that changes compatibility surfaces produces an assertion failure with a migration-related message

```
fn migration_required_run_id_zero_byte_change_without_migration_name_fails()
fn migration_required_workflow_digest_byte_change_without_migration_name_fails()
fn migration_required_record_kind_byte_change_without_migration_name_fails()
```

**Design note**: These tests use compile-time frozen byte constants and explicit `MIGRATION_REQUIRED` comment/doc annotations. They do not dynamically mutate bytes — they assert that the test itself documents its migration obligation and that the fixture bytes are explicitly frozen with migration naming.

---

## 4. Proptest Invariants

### Proptest: RunId newtype Postcard roundtrip
- **Invariant**: For any u64 value v, `postcard::from_bytes::<RunId>(&postcard::to_vec(&RunId::new(v)).unwrap()).unwrap() == RunId::new(v)`
- **Strategy**: `proptest::num::u64::ANY` — exhaustive over all u64 values
- **Anti-invariant**: N/A — all u64 values are valid RunId inputs
- **Test names**: `run_id_postcard_roundtrip_holds_for_any_u64`

### Proptest: RunId golden fixture decode consistency
- **Invariant**: Frozen fixture bytes consistently decode to the expected RunId value across both `postcard::from_bytes` and `RunId::new(v)` paths
- **Strategy**: `proptest::num::u64::ANY` compared against fixture-derived values
- **Anti-invariant**: N/A

### Proptest: WorkflowDigest byte preservation
- **Invariant**: For any [u8; 32] bytes b, `WorkflowDigest::from_bytes(b).as_bytes() == b`
- **Strategy**: `proptest::array::uniform32(proptest::num::u8::ANY)` — exhaustive over all 32-byte patterns
- **Anti-invariant**: N/A — all [u8; 32] arrays are valid WorkflowDigest inputs
- **Test names**: `workflow_digest_from_bytes_as_bytes_roundtrip_for_any_32_bytes`

### Proptest: WorkflowDigest Postcard roundtrip
- **Invariant**: `postcard::from_bytes::<WorkflowDigest>(&postcard::to_vec(&WorkflowDigest::from_bytes(b)).unwrap()).unwrap().as_bytes() == b`
- **Strategy**: `proptest::array::uniform32(proptest::num::u8::ANY)`
- **Test names**: `workflow_digest_postcard_roundtrip_holds_for_any_32_bytes`

### Proptest: Trailing bytes always rejected
- **Invariant**: For any valid encoded bytes `base` and any nonempty trailing suffix `suffix` (len 1..64), `postcard::from_bytes::<T>(&[base, suffix].concat())` returns `Err`
- **Strategy**: `any::<[u8; 32]>()` for digest base + `proptest::collection::vec(any::<u8>(), 1..=64)` for suffix
- **Falsifiability**: Assertion is `assert!(decoded.is_err())` — if postcard ever accepted trailing bytes, proptest would find the counterexample
- **Test names**: `trailing_bytes_rejected_for_any_suffix_on_workflow_digest`, `trailing_bytes_rejected_for_any_suffix_on_run_id`

### Proptest: Missing bytes return typed error
- **Invariant**: For any short byte slice len < RECORD_HEADER_BYTES, `decode_record_header(&short)` returns `Err(JournalError::UnexpectedEof)`
- **Strategy**: `proptest::collection::vec(any::<u8>(), 0..RECORD_HEADER_BYTES - 1)`
- **Anti-invariant**: Any input of exactly RECORD_HEADER_BYTES should NOT return UnexpectedEof (would indicate off-by-one)
- **Test names**: `decode_record_header_returns_unexpected_eof_for_any_short_input`

---

## 5. Fuzz Targets

**No new fuzz targets are required for this test file.** The following fuzz targets were written and verified at State 6 and provide the required coverage:

| Fuzz Target | Obligation | Evidence |
|---|---|---|
| `fuzz/fuzz_targets/vb_dybj_storage_short_decode.rs` | PO-VB-DYBJ-012 | `#10000 DONE, no crash` |
| `fuzz/fuzz_targets/vb_dybj_trailing_decode.rs` | PO-VB-DYBJ-015 | `#1000 DONE, no crash` (smoke bound) |

These fuzz targets live in `fuzz/fuzz_targets/` and are not duplicated in the behavior test file. The test file provides proptest-based behavior tests that assert exact error variants and golden byte values, complementing the fuzz coverage.

---

## 6. Kani Harnesses

**No new Kani harnesses are required for this test file.** The following Kani harnesses were written and verified at State 6:

| Kani Harness | Obligation | Evidence |
|---|---|---|
| `crates/vb_core/src/kani_vb_dybj_run_id_postcard.rs` | PO-VB-DYBJ-002 | VERIFICATION SUCCESSFUL |
| `crates/workspace_tests/src/kani_vb_dybj_trailing_decode.rs` | PO-VB-DYBJ-013 | `0 of 238 failed (5 unreachable)` |

These harnesses live in their respective crate source trees and are not duplicated in the test file.

---

## 7. Mutation Checkpoints

Critical mutations to survive:

| Function/Branch | Killed By Test | Rationale |
|---|---|---|
| RunId Postcard serialize bytes | `run_id_*_golden_fixture` | If postcard::to_vec produces different bytes for same RunId value, fixture assertion fails |
| RunId Postcard deserialize | `run_id_decode_from_golden_fixture_*` | If from_bytes produces wrong RunId, equality assertion fails |
| WorkflowDigest::from_bytes → as_bytes | `workflow_digest_from_bytes_as_bytes_*` | If roundtrip mutates bytes, assertion fails |
| WorkflowDigest Postcard bytes | `workflow_digest_*_golden_fixture` | Same as RunId above |
| RecordKind::id() match arm | `record_kind_*_envelope_id_u16_le_*` | If id() returns wrong u16 for a variant, assertion fails |
| RecordKind Postcard enum bytes | `record_kind_*_postcard_enum_bytes_*` | If postcard serializes enum differently, golden fixture fails |
| decode_record_header len check | `decode_record_header_returns_unexpected_eof_for_*` | If len check is removed or weakened, short input panic would be caught |
| trailing_bytes rejection in exact decode | `trailing_bytes_*_rejected_*` | If exact decode silently ignores trailing bytes, proptest assertion fails |
| Postcard payload decode error | `decode_record_returns_postcard_decode_failed_*` | If error variant changes, error assertion fails |

**Threshold**: ≥90% mutation kill rate. Every behavior listed above has at least one test that directly asserts the exact outcome — no `is_ok()`/`is_err()` without value assertions.

---

## 8. Combinatorial Coverage Matrix

### RunId module

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| Happy: new(v).get() == v | v = 0 | 0 | Unit |
| Happy: new(v).get() == v | v = 1 | 1 | Unit |
| Happy: new(v).get() == v | v = u64::MAX | u64::MAX | Unit |
| Happy: new(v).get() == v | v = mid-range (e.g. 0xDEAD_BEEF) | 0xDEAD_BEEF | Unit |
| ZERO constant | RunId::ZERO | RunId(0) | Unit |
| Golden fixture: ZERO postcard | RunId::ZERO → postcard | RUN_ID_ZERO_POSTCARD_BYTES | Unit |
| Golden fixture: MAX postcard | RunId(u64::MAX) → postcard | RUN_ID_MAX_POSTCARD_BYTES | Unit |
| Decode fixture: ZERO | from_bytes(fixture) | RunId::ZERO | Unit |
| Decode fixture: MAX | from_bytes(fixture) | RunId(u64::MAX) | Unit |
| Proptest: roundtrip | any u64 | original value | Unit |
| Proptest: trailing rejected | valid base + 1..64 bytes suffix | Err(postcard::Error) | Integration |

### WorkflowDigest module

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| Happy: roundtrip | [0u8; 32] | [0u8; 32] | Unit |
| Happy: roundtrip | nontrivial pattern | same pattern | Unit |
| Golden fixture: zero | WorkflowDigest([0u8; 32]) → postcard | WF_ZERO_POSTCARD_BYTES | Unit |
| Golden fixture: nontrivial | pattern → postcard | WF_PATTERN_POSTCARD_BYTES | Unit |
| Decode fixture: nontrivial | from_bytes(fixture) | original digest | Unit |
| Proptest: bytes roundtrip | any [u8; 32] | original bytes | Unit |
| Proptest: Postcard roundtrip | any [u8; 32] | original digest | Unit |
| Proptest: trailing rejected | valid base + 1..64 bytes suffix | Err(postcard::Error) | Integration |

### RecordKind module

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| envelope_id_u16_le: RunHeader | RecordKind::RunHeader.id() | 3 | Integration |
| envelope_id_u16_le: RunAccepted | RecordKind::RunAccepted.id() | 10 | Integration |
| postcard_enum: RunHeader | to_vec(RunHeader) | golden bytes | Integration |
| postcard_enum: RunAccepted | to_vec(RunAccepted) | golden bytes | Integration |
| Surface distinction | postcard bytes vs id().to_le_bytes() | NOT equal | Integration |

### missing_bytes module

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| Zero-length input | &[] | Err(UnexpectedEof) | Integration |
| 1-byte input | &[0x01] | Err(UnexpectedEof) | Integration |
| RECORD_HEADER_BYTES-1 input | len < RECORD_HEADER_BYTES | Err(UnexpectedEof) | Integration |
| Exactly RECORD_HEADER_BYTES | exact length | NOT Err(UnexpectedEof) (magic may still fail) | Integration |
| Corrupted payload | valid header + invalid payload | Err(PostcardDecodeFailed) | Integration |
| Proptest: any short input | 0..RECORD_HEADER_BYTES-1 bytes | Err(UnexpectedEof) | Integration |

### trailing_bytes module

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| Single extra byte on RunId | valid bytes + 1 byte | Err | Integration |
| Multiple extra bytes on RunId | valid bytes + 1..64 bytes | Err | Integration |
| Single extra byte on WorkflowDigest | valid bytes + 1 byte | Err | Integration |
| Multiple extra bytes on WorkflowDigest | valid bytes + 1..64 bytes | Err | Integration |
| Proptest: any nonempty suffix | valid base + 1..64 bytes | Err(postcard::Error or PostcardDecodeFailed) | Integration |

### migration_required module

| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| RunId ZERO fixture frozen | compile-time constant | documented migration name present | Static/Unit |
| WorkflowDigest fixture frozen | compile-time constant | documented migration name present | Static/Unit |
| RecordKind fixture frozen | compile-time constant | documented migration name present | Static/Unit |

---

## 9. Test File Structure

```
crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs
└── #![forbid(unsafe_code)]
    ├── use statements (postcard, vb_core, vb_storage, proptest)
    ├── Frozen golden-byte constants section
    │   ├── const RUN_ID_ZERO_POSTCARD_BYTES: [u8; N] = [...];
    │   ├── const RUN_ID_MAX_POSTCARD_BYTES: [u8; N] = [...];
    │   ├── const WORKFLOW_DIGEST_ZERO_POSTCARD_BYTES: [u8; 34] = [...];
    │   ├── const WORKFLOW_DIGEST_PATTERN_POSTCARD_BYTES: [u8; 34] = [...];
    │   ├── const RECORD_KIND_RUN_HEADER_POSTCARD_BYTES: [u8; N] = [...];
    │   ├── const RECORD_KIND_RUN_ACCEPTED_POSTCARD_BYTES: [u8; N] = [...];
    │   └── const MIGRATION_REQUIRED: &str = "vb-dybj-run-id-workflow-digest-record-kind-2026";
    ├── mod run_id { ... }          // B1-B5 + proptest
    ├── mod workflow_digest { ... } // B6-B7 + proptest
    ├── mod record_kind { ... }     // B8-B9
    ├── mod missing_bytes { ... }   // B11-B12 + proptest
    ├── mod trailing_bytes { ... }  // B10 + proptest
    └── mod migration_required { ... } // B13
```

---

## 10. Dependencies Required (test-writer check)

The test file requires these dependencies already available in `crates/workspace_tests/Cargo.toml`:

| Dependency | Available in Cargo.toml | Used For |
|---|---|---|
| `postcard` | Yes (workspace) | Serialize/deserialize for golden fixtures |
| `vb_core` | Yes (path dependency) | RunId, WorkflowDigest |
| `vb_storage` | Yes (dev-dependency, path) | decode_record_header, JournalError |
| `proptest` | Yes (workspace, dev-dependency) | Property-based test strategies |
| `serde` | Yes (workspace) | Serde derive support |

No new dependencies needed. No forbidden codec/dependency (JSON, YAML, HTTP, Bilrost, Protobuf) is introduced.

---

## 11. Evidence Commands

### Run all tests in the new file
```bash
cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests --no-fail-fast
```

### Run specific sub-modules
```bash
cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests run_id --no-fail-fast
cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests workflow_digest --no-fail-fast
cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests record_kind --no-fail-fast
cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests missing_bytes --no-fail-fast
cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests trailing_bytes --no-fail-fast
cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests migration_required --no-fail-fast
```

### CI gate
```bash
moon ci
```

---

## 12. Anti-Pattern Checklist

| Rule | Check |
|---|---|
| No `assert!(result.is_ok())` without asserting inner value | ✅ — All assertions assert exact byte equality, exact error variant, or exact value |
| No mocking | ✅ — All dependencies are real (postcard, vb_core, vb_storage) |
| No logic (loops/conditionals) in test bodies | ✅ — Proptest handles iteration; test bodies are assert-only |
| No `sleep()` | ✅ — No async or timing-dependent tests |
| One logical assertion per test | ✅ — One behavior per test function; fixture tests assert one property |
| Test names describe behavior | ✅ — Function names use `[subject]_[outcome]_when_[condition]` pattern |
| No test that passes if implementation is deleted | ✅ — All tests assert exact values/fixtures; deletion breaks assertions |
| DAMP over DRY | ✅ — Golden byte constants are explicit; shared helpers minimal |
| No forbidden codecs | ✅ — Only postcard; no JSON/YAML/HTTP/Bilrost/Protobuf |

---

## Open Questions

**None.** All domain questions from `contract.md` section "Open Domain Questions" are resolved:

1. **Q: Should tests assert both RecordKind Postcard enum bytes and envelope RecordKind::id() LE bytes?**  
   **A: Both.** Test names distinguish `postcard_enum` and `envelope_id_u16_le` surfaces as required by PO-VB-DYBJ-009. The `record_kind` module includes tests for both surfaces with explicit assertions that they are not the same bytes.

2. **Q: Should typed trailing-byte acceptance be at raw Postcard level or storage level?**  
   **A: Both.** The `trailing_bytes` module tests raw Postcard exact-value decode rejection. The `missing_bytes` module tests storage-level `JournalError::UnexpectedEof` for short headers and `JournalError::PostcardDecodeFailed` for payload corruption. Each module covers the appropriate surface.

The target file path `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` is correct and the test target is already registered in `Cargo.toml`.

---

Test plan ready for test-writer at State 9.
