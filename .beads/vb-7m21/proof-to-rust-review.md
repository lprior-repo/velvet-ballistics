# Proof-to-Rust Review — vb-7m21 State 7

**reviewer_skill**: proof-reviewer
**reviewer_invocation_id**: proof-reviewer-vb-7m21-state7-bridge-001
**reviewed_writer_invocation_id**: proof-to-implementation-vb-7m21-state7-001
**bead_id**: vb-7m21
**state**: 7 (bridge review)
**sublane**: proof-to-rust-review
**reviewed_artifacts_existed_before_start**: true

## Executive Summary

Review of the bridge mapping produced by `proof-to-implementation-vb-7m21-state7-001` (ledger sequence 21). The bridge maps 14 approved proof claims (3 Kani + 8 proptest + 3 fuzz) from State 6 `proof-review.md` (STATUS: APPROVED) to concrete Rust source refs, independent behavior tests, and refinement harnesses. The mapping is honest, thorough, and correctly documents residual gaps. All 14 `rust-refinement-obligation/v1` rows are behavior-affecting and include source refs, behavior test refs, refinement harness refs, and evidence commands.

**Verdict: APPROVED with documented findings.**

## Provenance Verification

| Check | Result | Evidence |
|---|---|---|
| Self-approval | PASS | Writer is `proof-to-implementation`, reviewer is `proof-reviewer`. Different skills. |
| Artifacts pre-existed | PASS | `reviewed_artifacts_existed_before_start: true` in ledger entry 21. |
| Parent invocation | PASS | `parent_invocation_id`: `proof-reviewer-vb-7m21-state6-001` (APPROVED). |
| Bridge writer provenance | PASS | Ledger entry 21 records `proof-to-implementation-vb-7m21-state7-001` with correct input artifact hashes. |

## Obligation Mapping Verification

### Kani Claims (PO-vb-7m21-kani-001/002/003) → Source

| Proof ID | Claimed Rust Target | Verified? | Notes |
|---|---|---|---|
| PO-vb-7m21-kani-001 | `decode_record_header`, `decode_record_payload`, `decode_record` | YES | All three found in `kani_vb_7m21_codec_panic.rs` with `kani::any()` inputs ✅ |
| PO-vb-7m21-kani-002 | `validate_schema_version`, `validate_known_kind`, `validate_kind_family` | YES | Found in `kani_vb_7m21_header_validate.rs`. Functions exist in `crates/vb_storage/src/codec/validation.rs`. |
| PO-vb-7m21-kani-003 | `payload_len_u32`, `encode_record_payload`, `decode_record_payload` | YES | Found in `kani_vb_7m21_payload_bounds.rs`. Functions exist in `crates/vb_storage/src/codec/payload.rs`. |

**GOD RULE 1 audit**: Verified in `kani_vb_7m21_codec_panic.rs`. All three harnesses use `kani::any()` for inputs. No hardcoded `WorkflowParts`, `RunFrame`, or fixed dummy data. Lines 25-33 use `kani::any()` in `arbitrary_max_payload_len()` for discrete bound sampling. Lines 37-41 use `kani::any()` in `arbitrary_byte_len()` capped at 128. Lines 53, 57, 58 use `kani::any()` directly for byte values, magic, and max_payload_len. PASS. ✅

**Harness naming note**: The bridge maps to three consolidated harness files (`kani_vb_7m21_codec_panic.rs`, `kani_vb_7m21_header_validate.rs`, `kani_vb_7m21_payload_bounds.rs`) which supersede the original eight `kani_vb_7m21_001.rs` through `kani_vb_7m21_008.rs`. The `lib.rs` declares both sets under `#[cfg(kani)]`. This consolidation is a legitimate refactoring during State 5 repair cycles.

### Proptest Claims (PO-vb-7m21-prop-001/008) → Source

| Proof ID | Claimed Rust Target | Behavior Test | Verified? |
|---|---|---|---|
| PO-vb-7m21-prop-001 | `encode_record_header`, `payload_len_u32` | `oversized_declared_record_returns_payload_too_large` | YES — calls `vb_storage::encode_record_header` directly |
| PO-vb-7m21-prop-002 | `CURRENT_SCHEMA_VERSION` | `future_schema_is_unsupported` | YES — uses `vb_storage::CURRENT_SCHEMA_VERSION` |
| PO-vb-7m21-prop-003 | `decode_record_header` | `truncated_header_is_unexpected_eof` | YES — calls `vb_storage::decode_record_header` directly |
| PO-vb-7m21-prop-004 | `indexes.rs`, `JournalError` | `missing_side_index_is_typed` | **CLASSIFIER-ONLY** (see Finding B7-001) |
| PO-vb-7m21-prop-005 | `JournalError::SequenceGap`, `journal/replay.rs` | `sequence_gap_is_typed` | **CLASSIFIER-ONLY** (see Finding B7-001) |
| PO-vb-7m21-prop-006 | `JournalError::DuplicateEvent`, `journal/core.rs` | `divergent_duplicate_is_typed` | **CLASSIFIER-ONLY** (see Finding B7-001) |
| PO-vb-7m21-prop-007 | `snapshots.rs`, `recovery/types.rs` | `stale_snapshot_replays_tail` | **CLASSIFIER-ONLY** (see Finding B7-001) |
| PO-vb-7m21-prop-008 | `keys.rs`, `journal/internal.rs` | `missing_manifest_keyspace_is_typed` | **CLASSIFIER-ONLY** (see Finding B7-001) |

Properties prop-001 through prop-003 exercise actual `vb_storage` public APIs. Properties prop-004 through prop-008 use local `classify_*` functions defined in the test file (lines 16-62) with `CorpusOutcome` enum — they verify the classification contract but do not call storage public APIs for index parity, sequence checking, duplicate detection, snapshot recovery, or manifest validation.

### Fuzz Claims (PO-vb-7m21-fuzz-001/002/003) → Source

| Proof ID | Claimed Rust Target | Fuzz Target File | Verified? |
|---|---|---|---|
| PO-vb-7m21-fuzz-001 | `decode_record_header`, `decode_record`, `decode_journal_event` | `fuzz/fuzz_targets/vb_7m21_envelope_decode.rs` (1.3K) | YES — file exists |
| PO-vb-7m21-fuzz-002 | `decode_record_header` | `fuzz/fuzz_targets/vb_7m21_header_parse.rs` (1.6K) | YES — file exists |
| PO-vb-7m21-fuzz-003 | `decode_record_payload`, `verify_digest_match`, `encode_record`, `decode_record` | `fuzz/fuzz_targets/vb_7m21_payload_decode.rs` (2.3K) | YES — file exists |

### Full Mapping Table Audit

The 14-row mapping table at `proof-to-rust-map.md:93-109` correctly enumerates:
- Proof ID, claim description, behavior-affecting flag ✅
- Rust source refs pointing to production code ✅
- Behavior test refs pointing to the test file ✅
- Refinement harness refs (Kani files, fuzz targets) ✅
- Verifier tool ✅
- Evidence command with exact flags ✅
- Rerun-from state ✅

## Residual Gap Assessment

### 1. Kani Verification BLOCKED_TOOLING (ACCEPTED_TRUST_BOUNDARY)

All 12 Kani harnesses (3 files, 547 total lines) compile under `cargo kani 0.67.0` but verification is blocked by `std::ptr::drop_in_place::<error::JournalError>` recursive unwinding. The bridge honestly marks all Kani rows as `mapping_status: planned` with `rerun_from: 11`. GOD RULE 1 compliance confirmed. Remediation path (`Kani 0.68+` or `--enable-unstable --concrete-drop`) is documented. **ACCEPTED** — genuine tooling limitation, not a proof or mapping failure.

### 2. Fuzz Deep Campaign DEFERRED (ACCEPTED_TRUST_BOUNDARY)

All 3 fuzz targets compile. Deep libFuzzer campaigns (`-max_total_time=3600 -runs=500000`) deferred to State 11. Bridge marks all fuzz rows as `mapping_status: planned` with `rerun_from: 11`. **ACCEPTED** — explicit deferral to formal-verifier state.

### 3. Proptest Classifier Gap (L_PROPTEST_CLASSIFIER_ONLY)

5 of 8 proptest properties use local classifier functions rather than calling `vb_storage` public APIs directly. Bridge marks these as `mapping_status: planned` with explicit documentation: "Public API integration deferred." The classifiers verify classification logic correctness for the input space, but do not exercise Fjall journal setup, index persistence, or snapshot recovery. **ACCEPTED** — gap is honestly documented and deferred to future beads. The `rust-refinement-obligations.jsonl` rows for these obligations include `refinement_claim` fields that explicitly note `(L_PROPTEST_CLASSIFIER_ONLY)` and the need for future API integration.

### 4. Contract Requirement Coverage Gaps

| REQ | Description | Mapped? | Status |
|---|---|---|---|
| REQ-1 | Known-good journal event fixture succeeds | ❌ NOT MAPPED | **GAP** — no proof obligation or behavior test covers happy-path journal event acceptance |
| REQ-2 | Known-good snapshot envelope fixture succeeds | ❌ NOT MAPPED | **GAP** — no proof obligation or behavior test covers happy-path snapshot acceptance |
| REQ-3 | Unknown/future schema → UnsupportedSchemaVersion | ✅ prop-002, kani-002, fuzz-002 | Mapped |
| REQ-4 | Missing side-index → IndexParityMismatch | ✅ prop-004 (classifier) | Mapped but classifier-only |
| REQ-5 | Oversized record → PayloadTooLarge | ✅ prop-001, kani-001, fuzz-001 | Mapped |
| REQ-6 | Truncated header → UnexpectedEof | ✅ prop-003, kani-003, fuzz-003 | Mapped |
| REQ-7 | Corrupt envelope/payload → exact errors | ⚠️ PARTIAL | Fuzz targets cover decode paths but no dedicated proptest for corrupt payload digest/crc |
| REQ-8 | Journal gap → SequenceGap | ✅ prop-005 (classifier) | Mapped but classifier-only |
| REQ-9 | Duplicate event → DuplicateEvent | ✅ prop-006 (classifier) | Mapped but classifier-only |
| REQ-10 | Stale snapshot → typed recovery error | ✅ prop-007 (classifier) | Mapped but classifier-only |
| REQ-11 | Missing manifest → typed outcome | ✅ prop-008 (classifier) | Mapped but classifier-only |
| REQ-12 | Each fixture → one typed outcome | ⚠️ IMPLICIT | Not separately mapped; outcome matrix is implicit in test design |
| REQ-13 | Covers every storage error family | ⚠️ PARTIAL | See Finding B7-002 |
| REQ-14 | No random bytes without seed | ⚠️ IMPLICIT | Proptest seeds present; Kani uses `kani::any()` which is deterministic within bounds |
| REQ-15 | Corruption operates on isolated temp storage | ⚠️ IMPLICIT | Test-only bead; no persistent storage mutation |
| REQ-16 | Uses VB public APIs, no Restate copy | ✅ | `proof-to-rust-map.md:90` confirms provenance review |

**Finding B7-002**: REQ-1 and REQ-2 (happy-path acceptance) are not covered by any of the 14 proof obligations. The test file `restate_storage_blackhat_fixture_corpus.rs` contains only error-path properties. This gap exists in the approved proof plan (State 4 replan reduced scope) and propagates through the bridge. While not strictly a bridge mapping defect (the bridge maps what was planned), it is a contract coverage gap that must be addressed in test planning (State 8) or future bead scope.

**Finding B7-003**: REQ-7 (corrupt envelope/payload → exact typed errors) has partial coverage through fuzz targets (envelope decode, payload decode, header parse) but lacks a dedicated proptest property with deterministic corruption mutation and exact error variant assertion for CRC mismatch and payload digest mismatch scenarios. The Kani `kani_vb_7m21_codec_panic.rs` harness covers `HeaderChecksumMismatch` and `PayloadDigestMismatch` as `kani::cover!()` branches but not as explicit assertions.

### 5. Downstream Dependency Resolution

The bridge resolves the four downstream dependencies from `proof-to-implementation-input.md`:

1. **REQ-4/PS-004** (`IndexParityMismatch` not a public `JournalError` variant): Resolved via `CorpusOutcome::IndexParityMismatch` in test file. Future bead must add variant. ✅
2. **REQ-9/PS-006** (`duplicate idempotency key` not a located storage concept): Resolved via `CorpusOutcome::DuplicateEvent` classification. Future bead must decide storage vs runtime surface. ✅
3. **REQ-11/PS-008** (`missing manifest` bound to Fjall keyspace): Resolved via `classify_manifest` mask logic. Full Fjall integration deferred. ✅
4. **REQ-16/PS-009** (`no-copy fence`): Confirmed — all fixtures use VB public APIs/constants. ✅

## Trust Marker Audit

The `trusted-base-ledger.jsonl` contains 26 rows. Relevant to bridge scope:

| Trust ID | Artifact | Status | Bridge Relevance |
|---|---|---|---|
| TB-vb-7m21-STATE5-005-PROPTEST-CLASSIFIER-RESIDUAL | Test file | ACTIVE | Confirms classifier-only limitation is already ledged. Bridge reinforces. |
| TB-vb-7m21-STATE5-005-KANI-001-ASSUME | kani_vb_7m21_001.rs | ACTIVE | Old harness; superseded by consolidated files but still active. |
| TB-vb-7m21-STATE5-005-KANI-002-ASSUME | kani_vb_7m21_002.rs | ACTIVE | Same as above. |
| TB-vb-7m21-STATE5-005-KANI-LEGACY-DISABLE | kani_recovery_hydrate.rs | ACTIVE | Legacy harness disabled via `cfg(any())`. Not in bridge scope. |
| TB-vb-7m21-STATE5-005-KANI-004-ABSTRACTION | kani_vb_7m21_004.rs | ACTIVE | Old harness superseded; same bounded-model-abstraction limitation. |

**Note**: The consolidated harness files (`kani_vb_7m21_codec_panic.rs` etc.) are not separately ledged in trusted-base-ledger.jsonl. The bridge review confirms the `ACCEPTED_TRUST_BOUNDARY` disposition for all 12 Kani harnesses from State 6 proof review continues to apply to the consolidated files.

## Raw Evidence Verification

| Evidence Claim | Verified? | Notes |
|---|---|---|
| 8 proptest properties PASS (nextest 0.004s) | ✅ | Confirmed in proof-review.md:43-49 with raw nextest output |
| Kani compilation PASS (1.30s) | ✅ | Confirmed in proof-review.md:68-73 with `--only-codegen` output |
| Fuzz compilation PASS (0.05s) | ✅ | Confirmed in proof-review.md:111-114 with `cargo check` output |
| Kani harnesses use `kani::any()` | ✅ | Verified in `kani_vb_7m21_codec_panic.rs` lines 25-58 |
| Fuzz targets exist | ✅ | All 3 files confirmed on disk |
| Behavior test file exists | ✅ | `restate_storage_blackhat_fixture_corpus.rs` (114 lines) |

## Findings Summary

| # | ID | Severity | Code | Summary |
|---|---|---|---|---|
| 1 | PF-vb-7m21-B7-001 | low | L_BRIDGE_CLASSIFIER_ONLY_PROPAGATED | 5 proptest properties mapped to classifier functions rather than public storage APIs. Bridge honestly documents. Deferred to future bead for API integration. |
| 2 | PF-vb-7m21-B7-002 | medium | M_CONTRACT_COVERAGE_GAP_REQ1_REQ2 | REQ-1 (happy-path journal event) and REQ-2 (happy-path snapshot) have no proof obligations or behavior tests mapped. Gap from reduced-scope replan propagates through bridge. Must be addressed in State 8 test planning or future bead scope. |
| 3 | PF-vb-7m21-B7-003 | low | L_REQ7_CORRUPT_ENVELOPE_PARTIAL | REQ-7 (corrupt envelope/payload → exact typed errors) has fuzz coverage but lacks dedicated proptest with deterministic corruption mutation and exact error variant assertions. |
| 4 | PF-vb-7m21-B7-004 | info | I_TRUSTED_BASE_NAMING_DISCREPANCY | `trusted-base-ledger.jsonl` references old `kani_vb_7m21_00[1-8].rs` files; bridge maps to consolidated `kani_vb_7m21_{codec_panic,header_validate,payload_bounds}.rs`. Both sets exist in `lib.rs`. Consolidated files not separately ledged. |
| 5 | PF-vb-7m21-B7-005 | info | I_KANI_BLOCKED_TOOLING_PROPAGATED | Kani verification blocked by Kani 0.67 tooling limitation. Bridge honestly marks all 3 Kani obligation groups as `mapping_status: planned`, `rerun_from: 11`. |
| 6 | PF-vb-7m21-B7-006 | info | I_FUZZ_DEEP_CAMPAIGN_PROPAGATED | Fuzz deep campaigns deferred to State 11. Bridge marks all 3 fuzz obligations as `mapping_status: planned`, `rerun_from: 11`. |

## Review Decision

The bridge maps all 14 planned proof obligations to concrete Rust source refs, behavior tests, and refinement harnesses with exact evidence commands. Gaps are honestly documented: Kani verification blocked by tooling, fuzz deep campaigns deferred, 5 proptest properties are classifier-only with public API integration deferred. 

Two contract coverage gaps propagate from the reduced-scope replan:
1. **REQ-1 and REQ-2** (happy-path acceptance fixtures) are not covered. This is a contract gap, not a bridge mapping failure — the bridge maps what was planned.
2. **REQ-7** (corrupt envelope/payload → exact errors) has partial fuzz coverage without dedicated proptest.

The bridge-audit override is to accept with the documented findings, noting that State 8 test planning must address the REQ-1/REQ-2 coverage gap as an explicit planning decision (accept, defer, or add to scope).

STATUS: APPROVED
