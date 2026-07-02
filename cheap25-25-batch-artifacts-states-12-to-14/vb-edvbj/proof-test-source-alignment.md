# Proof / Test / Source Alignment — vb-edvbj

- **bead_id:** vb-edvbj
- **bead_title:** Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
- **phase:** 12 (formal-verifier)
- **workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-edvbj`
- **invocation_id:** formal-verifier-vb-edvbj-state12
- **date:** 2026-07-01
- **row count:** 10 (matches the 10 POs in `proof-obligations.planned.jsonl`)

This artifact pairs each proof obligation with its proof artifact, its behavior-test
evidence, and the source artifact it targets. The honest alignment for State 12 is
documented below; the raw JSONL is `proof-test-source-alignment.jsonl`.

## Alignment Summary

| Obligation | Proof artifact status | Test evidence | Source artifact | Alignment |
|------------|----------------------|---------------|-----------------|-----------|
| PO-EDVBJ-001-VERUS | untracked on disk (verifier error) | cargo test storage_event: 1 passed | chunk_002.rs (storage_event post-fix) | PARTIAL |
| PO-EDVBJ-002-KANI | absent (pre-existing kani build blocker) | cargo test storage_event: 1 passed | chunk_002.rs | GAP |
| PO-EDVBJ-003-PROPTEST | absent (Cargo feature not declared) | cargo test --lib: 1807 passed | chunk_001.rs + chunk_002.rs | GAP |
| PO-EDVBJ-004-PROPTEST | absent (Cargo feature not declared) | cargo test recovery: 13 passed | chunk_002.rs | GAP |
| PO-EDVBJ-005-VERUS | untracked on disk, missing companion (VACUUM) | cargo test --lib: 1807 passed | chunk_002.rs:342-346, chunk_003.rs:8-16 | GAP |
| PO-EDVBJ-006-KANI | absent (pre-existing kani build blocker) | cargo test --lib: 1807 passed | chunk_002.rs:342-346, chunk_003.rs:8-16 | GAP |
| PO-EDVBJ-007-VERUS | untracked on disk (verus verifies 2/0) | scripts/check-verus-production-binding.sh: 73 WEAK, 2 VACUUM | storage_kind_family.rs mirror + storage events.rs/records.rs | **ALIGNED** |
| PO-EDVBJ-008-FLUX | absent | cargo flux -p vb_runtime: package-level passes | error/diagnostics.rs:107-198 | GAP |
| PO-EDVBJ-009-VERUS | untracked on disk, missing companion (VACUUM) | cargo test --lib: 1807 passed | error/diagnostics.rs:107-198 | GAP |
| PO-EDVBJ-010-PROPTEST | absent (Cargo feature not declared) | cargo test --lib: 1807 passed | error/diagnostics.rs | GAP |

**Tally:** 1 ALIGNED (PO-007), 1 PARTIAL (PO-001), 8 GAP.

## 1. PO-EDVBJ-001-VERUS

- **Proof artifact:** `verification/verus/vb_edvbj_storage_event.rs` (untracked on disk)
- **Test artifact:** `cargo test -p vb_runtime --lib storage_event` → 1 passed
- **Source artifact:** `crates/vb_runtime/src/journal/chunk_002.rs::StorageRuntimeJournal::storage_event` (post-fix body in `mrpqqutq`)
- **Finding codes:** `verifier_error`, `spec_artifact_untracked`
- **Note:** The proof artifact exists on disk as an untracked file; the companion `extern_vb_edvbj_storage_event.rs` is tracked in `mrpqqutq` (added in `rzwmqlyw`); the production mirror at `production_inner/vb_edvbj_storage_event_production.rs` is tracked. The Verus invocation fails with a duplicate-specification error because the mirror's `mirror_storage_event` is not marked `#[verifier::external_body]`; the spec's `assume_specification` cannot attach to a non-`external` body. The behavior test passes.

## 2. PO-EDVBJ-002-KANI

- **Proof artifact:** `crates/vb_runtime/src/kani_vb_edvbj_storage_event_no_fabricate.rs` (absent)
- **Test artifact:** `cargo test -p vb_runtime --lib storage_event` → 1 passed
- **Source artifact:** `crates/vb_runtime/src/journal/chunk_002.rs` (post-fix)
- **Finding codes:** `missing_artifact`, `pre_existing_build_blocker`
- **Note:** The 6-harness Kani file is not on disk. The crate-level kani build path is also blocked by a pre-existing unclosed-delimiter compile error in `crates/vb_core/src/frame_kani_harnesses` (unrelated to this bead; see F-002 in `proof-findings.jsonl`).

## 3. PO-EDVBJ-003-PROPTEST

- **Proof artifact:** `crates/vb_runtime/src/journal/tests/proptest_vb_edvbj_all_21_variants.rs` (absent)
- **Test artifact:** `cargo test -p vb_runtime --lib` → 1807 passed
- **Source artifact:** `crates/vb_runtime/src/journal/chunk_001.rs` (added in `mrpqqutq`; `runtime_journal_event_kind` helper) and `chunk_002.rs`
- **Finding codes:** `missing_artifact`
- **Note:** The 21-variant exhaustive proptest file is not on disk. The `vb-edvbj-pending` Cargo feature is not declared in `crates/vb_runtime/Cargo.toml`.

## 4. PO-EDVBJ-004-PROPTEST

- **Proof artifact:** `crates/vb_runtime/src/journal/tests/proptest_vb_edvbj_resumed_replay.rs` (absent)
- **Test artifact:** `cargo test -p vb_runtime --lib recovery` → 13 passed
- **Source artifact:** `crates/vb_runtime/src/journal/chunk_002.rs` (post-fix)
- **Finding codes:** `missing_artifact`
- **Note:** The Fjall-backed Resumed replay proptest is not on disk; the `vb-edvbj-pending` feature is not declared.

## 5. PO-EDVBJ-005-VERUS

- **Proof artifact:** `verification/verus/vb_edvbj_propagation.rs` (untracked on disk; companion `extern_vb_edvbj_propagation.rs` is ABSENT)
- **Test artifact:** `cargo test -p vb_runtime --lib` → 1807 passed
- **Source artifact:** `crates/vb_runtime/src/journal/chunk_002.rs:342-346` (`append_sequenced`) and `chunk_003.rs:8-16` (`QueuedStorageRuntimeJournal::append_sequenced` Strict-profile guard); both unchanged in `mrpqqutq`
- **Finding codes:** `vacuum_proof`, `missing_companion`
- **Note:** The `scripts/check-verus-production-binding.sh` script flags `vb_edvbj_propagation.rs` as one of 2 VACUUM files in `verification/verus/`. The behavior test exercises the runtime surface that the propagation claim depends on, but the Verus lane cannot close until the spec is production-bound.

## 6. PO-EDVBJ-006-KANI

- **Proof artifact:** `crates/vb_runtime/src/kani_vb_edvbj_propagation_strict_gate.rs` (absent)
- **Test artifact:** `cargo test -p vb_runtime --lib` → 1807 passed
- **Source artifact:** `chunk_002.rs:342-346`, `chunk_003.rs:8-16`
- **Finding codes:** `missing_artifact`, `pre_existing_build_blocker`
- **Note:** The 2-harness Kani file is not on disk; same kani build blocker as PO-002.

## 7. PO-EDVBJ-007-VERUS — **ALIGNED**

- **Proof artifact:** `verification/verus/vb_edvbj_mirror_bind.rs` (untracked on disk; verus verifies cleanly: 2 verified, 0 errors)
- **Test artifact:** `bash scripts/check-verus-production-binding.sh` → 73 WEAK, 2 VACUUM (the 2 VACUUMs are `vb_edvbj_propagation.rs` and `vb_edvbj_symbolic_code.rs`; neither is the existing storage_kind_family mirror that PO-007 anchors)
- **Source artifact:** `verification/verus/extern_storage_kind_family.rs` (existing WEAK_MIRROR mirror, unchanged in `mrpqqutq`); `crates/vb_storage/src/events.rs` (JournalEvent shape, unchanged); `crates/vb_storage/src/records.rs` (RecordKind discriminant set, unchanged)
- **Finding codes:** none
- **Note:** PO-007 is the mandatory mirror-drift gate (H-3 mitigation). The script's WEAK classification for the existing storage_kind_family mirror is unaffected by this bead's fix; the companion `vb_edvbj_mirror_bind.rs` spec verifies (2/0). This is the only obligation that can close at State 12.

## 8. PO-EDVBJ-008-FLUX

- **Proof artifact:** `crates/vb_runtime/src/verification/flux/vb_edvbj_diagnostic_code_refinement.rs` (absent)
- **Test artifact:** `cargo flux -p vb_runtime` → Finished, 0 errors (package-level; not a per-refinement check)
- **Source artifact:** `crates/vb_runtime/src/error/diagnostics.rs:107-198` (modified in `mrpqqutq`; UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE = 0x2020 added; new arms in `diagnostic_code` and `runtime_code`)
- **Finding codes:** `missing_artifact`
- **Note:** The Flux refinement file for `UnmappedRuntimeJournalEvent` is not on disk. Package-level flux compiles, but the per-obligation refinement cannot close.

## 9. PO-EDVBJ-009-VERUS

- **Proof artifact:** `verification/verus/vb_edvbj_symbolic_code.rs` (untracked on disk; companion `extern_vb_edvbj_symbolic_code.rs` is ABSENT)
- **Test artifact:** `cargo test -p vb_runtime --lib` → 1807 passed
- **Source artifact:** `crates/vb_runtime/src/error/diagnostics.rs:107-198` (modified in `mrpqqutq`)
- **Finding codes:** `vacuum_proof`, `missing_companion`
- **Note:** The `scripts/check-verus-production-binding.sh` script flags `vb_edvbj_symbolic_code.rs` as the second of 2 VACUUM files.

## 10. PO-EDVBJ-010-PROPTEST

- **Proof artifact:** `crates/vb_runtime/src/error/tests_diagnostics/proptest_vb_edvbj_diagnostic_code.rs` (absent)
- **Test artifact:** `cargo test -p vb_runtime --lib` → 1807 passed
- **Source artifact:** `crates/vb_runtime/src/error/diagnostics.rs` (modified in `mrpqqutq`)
- **Finding codes:** `missing_artifact`
- **Note:** The diagnostic-code proptest is not on disk; the `vb-edvbj-pending` Cargo feature is not declared.
