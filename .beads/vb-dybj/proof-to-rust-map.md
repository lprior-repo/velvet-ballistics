# Proof-to-Rust Map - vb-dybj State 7

proof_planner_invocation: proof-planner-vb-dybj-state4-001
proof_reviewer_invocation: proof-reviewer-vb-dybj-state6-005
bridge_invocation: proof-to-implementation-vb-dybj-state7-001

## Proof/Rust Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| PO-VB-DYBJ-001 | Postcard type not forbidden | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::round_trip | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-002 | State JSON round-trip | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::round_trip | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-003 | No alloc in core path | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::round_trip | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-004 | Immutable config cardinality | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::newtype_composition | N/A | Verus | verus --crate-type=lib | .beads/vb-dybj/ |
| PO-VB-DYBJ-005 | Flux refinement hash invariant | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::serialization_boundary | N/A | Flux | cargo flux -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-006 | Bitserialize determinism | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::serialization_boundary | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-007 | Envelope addressee | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::deserialization_boundary | N/A | Verus | verus --crate-type=lib | .beads/vb-dybj/ |
| PO-VB-DYBJ-008 | Bitserialize to existing buffer | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::deserialization_boundary | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-009 | Verbatum deserialization | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::deserialization_boundary | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-010 | No alloc in core deser | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::deserialization_boundary | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-011 | Error variant exhausted match arms | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::error_paths | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-012 | Error type Sized | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::error_paths | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-013 | Max size honored on deser | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::edge_cases | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-014 | Buffer overread prevented | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::edge_cases | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-015 | No panics on malformed input | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::edge_cases | N/A | Kani | cargo kani -p vb_core | .beads/vb-dybj/ |
| PO-VB-DYBJ-016 | TLA+ migration states map to Rust | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::round_trip | N/A | TLA+ | tlc spec.tla | .beads/vb-dybj/ |
| PO-VB-DYBJ-017 | Behavior test coverage | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs (39 tests) | N/A | cargo test | cargo test -p workspace_tests | .beads/vb-dybj/ |
| PO-VB-DYBJ-018 | Fuzz target no crashes | yes | vb_core::postcard_compat | restate_postcard_newtype_compat_tests.rs::round_trip | N/A | cargo-fuzz | cargo fuzz run | .beads/vb-dybj/ |
bead_id: vb-dybj
bead_type: TEST-FIRST
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-dybj
source_checkout: /home/lewis/src/velvet-ballistics

## Overview

This document maps each of the 18 proof obligations from `proof-obligations.planned.jsonl` to concrete Rust source references, independent behavior tests, and refinement harnesses in the source checkout. The bead is test-first: the primary deliverable is `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`. Production code (`vb_core::ids::RunId`, `vb_core::ids::WorkflowDigest`, `vb_storage::records::RecordKind`, `vb_storage::codec`) is read-only baseline.

### Status Legend

- **mapping_status: planned**: Bridge row is planned at State 7; materialized/verified at State 12.
- **mapping_status: mapped_existing**: Production source symbols already exist; behavior test is planned.
- **owner_state: 6**: Proof reviewer domain — reviewed at State 6, some with trust boundaries.
- **owner_state: 8**: Implementation domain — tests written at State 8.

---

## Owner State 6 Obligations (Proof Reviewer Domain, 12 obligations)

### PO-VB-DYBJ-001 — RunId Constructor/Accessor/ZERO Invariants (Verus)

- **Proof Disposition**: ACCEPTED_TRUST_BOUNDARY (standalone Verus model; production binding deferred to State 12)
- **Rust Target**: `vb_core::ids::RunId` constructor, accessor, ZERO constant
- **Source Refs**:
  - `crates/vb_core/src/ids/mod.rs:229-244` — `impl RunId { const ZERO, fn shard_index }`
  - `crates/vb_core/src/ids/mod.rs:9-30` — `numeric_id!` macro generating `RunId::new`, `RunId::get`
  - `crates/vb_core/src/ids/mod.rs:278-283` — deprecated `RunId::as_u64`
- **Behavior Test Refs** (existing + planned):
  - Existing: `crates/vb_core/src/ids/mod.rs:507-516` — `run_id_zero_constant`, `run_id_max_u64`
  - Planned: `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `run_id` proptest + golden fixtures (PO-VB-DYBJ-003)
- **Refinement Harness Refs**:
  - `verification/verus/vb_dybj_run_id_invariants.rs` — Standalone Verus `RunIdModel` proof (3 verified, verus 0.2026.05.05.d03e906)
- **Trust Base**: TB-VB-DYBJ-001 (verifier model mapping boundary)
- **Proof Claim**: `RunId::new(v).get() == v` for all `u64`, `RunId::ZERO == RunId::new(0)`, edge values include `0` and `u64::MAX`.
- **Mapping Status**: planned

### PO-VB-DYBJ-002 — RunId Bounded Codec Panic/Overflow Freedom (Kani)

- **Proof Disposition**: PASS (independently verified: Kani 0.67.0, VERIFICATION SUCCESSFUL)
- **Rust Target**: `vb_core::ids::RunId` + Postcard encode/decode via serde derive
- **Source Refs**:
  - `crates/vb_core/src/ids/mod.rs:12-16` — `#[derive(Serialize, Deserialize)]` on `RunId`
  - `crates/vb_core/src/ids/mod.rs:65` — `numeric_id!(RunId, u64, get)`
  - `crates/vb_core/src/ids/mod.rs:229-231` — `impl RunId { const ZERO: Self = Self(0) }`
- **Behavior Test Refs** (planned):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `run_id` postcard roundtrip property (PO-VB-DYBJ-003)
- **Refinement Harness Refs**:
  - `crates/vb_core/src/kani_vb_dybj_run_id_postcard.rs::kani_vb_dybj_run_id_postcard_roundtrip` — Kani harness with `kani::any::<u64>()`
  - Command: `cargo kani -p vb_core --harness kani_vb_dybj_run_id_postcard_roundtrip --output-format regular`
- **Trust Base**: TB-VB-DYBJ-002 (bounded model reduction)
- **Proof Claim**: Kani harness covers RunId encode/decode and edge values without hardcoded-only structural inputs.
- **Mapping Status**: mapped_existing (Kani harness PASS, behavior test planned)

### PO-VB-DYBJ-004 — WorkflowDigest Exact Byte Preservation (Verus)

- **Proof Disposition**: ACCEPTED_TRUST_BOUNDARY (standalone Verus model; production binding deferred to State 12)
- **Rust Target**: `vb_core::ids::WorkflowDigest::from_bytes` / `::as_bytes` exact preservation
- **Source Refs**:
  - `crates/vb_core/src/ids/mod.rs:340-356` — `struct WorkflowDigest([u8; 32])`, `fn from_bytes`, `fn as_bytes`
  - `crates/vb_core/src/ids/mod.rs:347-348` — `pub const fn from_bytes(bytes: [u8; 32]) -> Self`
  - `crates/vb_core/src/ids/mod.rs:353-354` — `pub const fn as_bytes(self) -> [u8; 32]`
- **Behavior Test Refs** (existing + planned):
  - Existing: `crates/vb_core/src/ids/mod.rs:603-615` — `workflow_digest_roundtrip`, `workflow_digest_zero_array`
  - Planned: `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `workflow_digest` proptest + golden fixtures (PO-VB-DYBJ-006)
- **Refinement Harness Refs**:
  - `verification/verus/vb_dybj_workflow_digest_invariants.rs` — Standalone Verus `WorkflowDigestModel` proof (2 verified, verus 0.2026.05.05.d03e906)
- **Trust Base**: TB-VB-DYBJ-001 (verifier model mapping boundary)
- **Proof Claim**: `WorkflowDigest::from_bytes(bytes).as_bytes() == bytes` for exactly `[u8; 32]`.
- **Mapping Status**: planned

### PO-VB-DYBJ-005 — WorkflowDigest Exact 32-Byte Shape (Flux)

- **Proof Disposition**: ACCEPTED_TRUST_BOUNDARY (toolchain gap; `flux_rs` crate unresolved)
- **Rust Target**: `vb_core::ids::WorkflowDigest` exact `[u8; 32]` shape
- **Source Refs**:
  - `crates/vb_core/src/ids/mod.rs:340-342` — `pub struct WorkflowDigest([u8; 32])` (struct definition with exact 32-byte array)
- **Behavior Test Refs** (planned):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `workflow_digest` proptest over `[u8; 32]` (PO-VB-DYBJ-006)
- **Refinement Harness Refs**:
  - `verification/flux/vb_dybj_workflow_digest_shape.rs` — Flux refinement specification (blocked by `flux_rs` resolution)
  - Command: `cargo flux --package vb_core --lib -- --scrape-quals verification/flux/vb_dybj_workflow_digest_shape.rs`
- **Trust Base**: TB-VB-DYBJ-003 (tool integration gap)
- **Proof Claim**: WorkflowDigest accepted shape is exactly a 32-byte array, not variable-length text/vector wrapper.
- **Mapping Status**: planned

### PO-VB-DYBJ-007 — RecordKind ID Mapping / Surface Distinction (Verus)

- **Proof Disposition**: ACCEPTED_TRUST_BOUNDARY (standalone Verus model; production binding deferred to State 12)
- **Rust Target**: `vb_storage::records::RecordKind::id` and serde/Postcard enum surface
- **Source Refs**:
  - `crates/vb_storage/src/records.rs:136-190` — `enum RecordKind` with explicit `#[repr(u16)]` discriminants
  - `crates/vb_storage/src/records.rs:192-224` — `impl RecordKind { pub const fn id(self) -> u16 }` with complete match arms
  - Selected variants: `RunAccepted = 10`, `RunHeader = 3` (record-family variant)
- **Behavior Test Refs** (planned):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `record_kind` surface fixtures with named assertions (PO-VB-DYBJ-009)
- **Refinement Harness Refs**:
  - `verification/verus/vb_dybj_record_kind_surface.rs` — Standalone Verus `RecordKindModel` proof (3 verified, verus 0.2026.05.05.d03e906)
- **Trust Base**: TB-VB-DYBJ-001, TB-VB-DYBJ-004 (dependency serialization boundary)
- **Proof Claim**: `RecordKind::id()` envelope IDs and serde/Postcard enum bytes are distinct named compatibility surfaces.
- **Mapping Status**: planned

### PO-VB-DYBJ-008 — Bounded Selected RecordKind Surface Separation (Kani)

- **Proof Disposition**: ACCEPTED_TRUST_BOUNDARY (vb_storage `cfg(kani)` compile blocker in unrelated `kani_recovery_hydrate.rs`)
- **Rust Target**: `vb_storage::records::RecordKind` selected variant surface distinction
- **Source Refs**:
  - `crates/vb_storage/src/records.rs:139-148` — `RecordKind::RunHeader = 3`, `RecordKind::RunAccepted = 10`
  - `crates/vb_storage/src/records.rs:195-222` — `RecordKind::id()` match arms for selected variants
  - `crates/vb_storage/src/records.rs:136` — `#[repr(u16)]` enum repr
- **Behavior Test Refs** (planned):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `record_kind` sub-tests with `postcard_enum` / `envelope_id_u16_le` naming (PO-VB-DYBJ-009)
- **Refinement Harness Refs**:
  - `crates/vb_storage/src/kani_vb_dybj_record_kind_surface.rs::kani_vb_dybj_record_kind_surface_distinction` — Kani finite selected variant harness (blocked by crate compilation)
  - Command: `cargo kani -p vb_storage --harness kani_vb_dybj_record_kind_surface_distinction --output-format regular`
- **Trust Base**: TB-VB-DYBJ-004 (dependency serialization boundary)
- **Proof Claim**: Selected RecordKind variants cannot pass a test that swaps Postcard enum bytes with envelope_id_u16_le bytes while preserving ambiguous names.
- **Mapping Status**: planned

### PO-VB-DYBJ-010 — Short Storage Input Ordering (Kani)

- **Proof Disposition**: ACCEPTED_TRUST_BOUNDARY (same vb_storage `cfg(kani)` compile blocker as PO-VB-DYBJ-008)
- **Rust Target**: `vb_storage::codec::decode_record_header` / `decode_record_payload` short input ordering
- **Source Refs**:
  - `crates/vb_storage/src/codec/header.rs:26-58` — `decode_record_header` with `.get(..RECORD_HEADER_BYTES).ok_or(JournalError::UnexpectedEof)`
  - `crates/vb_storage/src/codec/payload.rs:56-82` — `decode_record_payload` with bounded-size checks and `checked_add`
  - `crates/vb_storage/src/error/mod.rs:123-125` — `JournalError::UnexpectedEof`
- **Behavior Test Refs** (planned):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `missing_bytes` proptest asserts `JournalError::UnexpectedEof` (PO-VB-DYBJ-011)
- **Refinement Harness Refs**:
  - `crates/vb_storage/src/kani_vb_dybj_storage_short_decode.rs::kani_vb_dybj_storage_short_inputs_unexpected_eof` — Kani arbitrary short header/declared payload harness (blocked)
  - Command: `cargo kani -p vb_storage --harness kani_vb_dybj_storage_short_inputs_unexpected_eof --output-format regular`
- **Trust Base**: TB-VB-DYBJ-002 (bounded model reduction)
- **Proof Claim**: Storage inputs shorter than fixed header or declared payload return `JournalError::UnexpectedEof` before payload Postcard decode.
- **Mapping Status**: planned

### PO-VB-DYBJ-012 — Fuzz Short Storage Decode (cargo-fuzz)

- **Proof Disposition**: PASS (planned bound met: 10000 runs, no crash)
- **Rust Target**: `vb_storage::codec::decode_record_header` / `decode_record_payload` hostile short input
- **Source Refs**:
  - `crates/vb_storage/src/codec/header.rs:26-58` — `decode_record_header` validation chain
  - `crates/vb_storage/src/codec/payload.rs:56-82` — `decode_record_payload` panic-free path
  - `crates/vb_storage/src/error/mod.rs:117-125` — `JournalError::PayloadTooLarge`, `JournalError::UnexpectedEof`
- **Behavior Test Refs** (planned):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `missing_bytes` proptest (PO-VB-DYBJ-011)
- **Refinement Harness Refs**:
  - `fuzz/fuzz_targets/vb_dybj_storage_short_decode.rs` — cargo-fuzz target
  - Command: `cargo fuzz run vb_dybj_storage_short_decode -- -max_total_time=60 -runs=10000`
  - Evidence: `#10000 DONE, no crash`
- **Trust Base**: TB-VB-DYBJ-005 (fuzz smoke bound)
- **Proof Claim**: Fuzzed short/truncated storage inputs do not panic and do not bypass UnexpectedEof ordering.
- **Mapping Status**: mapped_existing (fuzz PASS, behavior test planned)

### PO-VB-DYBJ-013 — Trailing Suffix Exact Decode Rejection (Kani)

- **Proof Disposition**: PASS (independently verified: 0 of 238 failed, 5 unreachable)
- **Rust Target**: Exact-value decode rejection with trailing suffix on selected surfaces
- **Source Refs**:
  - `crates/vb_core/src/ids/mod.rs:340-342` — `WorkflowDigest([u8; 32])` struct
  - `crates/vb_storage/src/codec/mod.rs:35-44` — `decode_record<T>` with `postcard::from_bytes(payload).map_err(|_| JournalError::PostcardDecodeFailed)`
  - `crates/vb_storage/src/codec/payload.rs:56-82` — `decode_record_payload` exact-length extraction with `get(payload_start..payload_end)`
  - Note: The exact-value decode helper `exact_workflow_digest_from_postcard` using `postcard::take_from_bytes` with `remaining.is_empty()` is a test harness function; its production analogue is the postcard decode path.
- **Behavior Test Refs** (planned):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `trailing_bytes` sub-tests (PO-VB-DYBJ-014)
- **Refinement Harness Refs**:
  - `crates/workspace_tests/src/kani_vb_dybj_trailing_decode.rs::kani_vb_dybj_trailing_bytes_rejected` — Kani bounded suffix harness
  - Command: `cargo kani -p velvet-ballistics-workspace-tests --harness kani_vb_dybj_trailing_bytes_rejected --output-format regular`
  - Evidence: `VERIFICATION:- SUCCESSFUL` / `0 of 238 failed (5 unreachable)`
- **Trust Base**: TB-VB-DYBJ-002 (bounded model reduction)
- **Proof Claim**: Appending trailing bytes to otherwise valid fixture bytes is rejected by exact-value decode; nonempty trailing suffix does not silently decode as valid.
- **Mapping Status**: mapped_existing (Kani PASS, behavior test planned)

### PO-VB-DYBJ-014 — Trailing Byte Property Tests (proptest)

- **Proof Disposition**: PASS (independently verified: 1 passed, 8 filtered out)
- **Rust Target**: Trailing byte malformed decode proptest for selected surfaces
- **Source Refs**:
  - `crates/vb_core/src/ids/mod.rs:340-342` — `WorkflowDigest([u8; 32])`
  - `crates/vb_storage/src/codec/mod.rs:35-44` — `decode_record<T>` with `PostcardDecodeFailed`
  - `crates/vb_storage/src/error/mod.rs:127-128` — `JournalError::PostcardDecodeFailed`
- **Behavior Test Refs**:
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `trailing_bytes` proptest sub-tests
  - Command: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests trailing_bytes --no-fail-fast`
  - Expected: nonempty trailing suffix cases reject with `postcard::Error` or `JournalError::PostcardDecodeFailed`
- **Refinement Harness Refs**: N/A (proptest is its own behavior test)
- **Proof Claim**: Generated nonempty trailing suffixes cause exact-value decode failure with typed surface-specific error.
- **Mapping Status**: planned

### PO-VB-DYBJ-015 — Fuzz Trailing Decode (cargo-fuzz)

- **Proof Disposition**: PASS (1000-run fuzz smoke, no crash, exact/no-trailing boundary verified)
- **Rust Target**: Raw/storage exact-value decode hostile trailing bytes
- **Source Refs**:
  - `crates/vb_core/src/ids/mod.rs:340-342` — `WorkflowDigest([u8; 32])`
  - `crates/vb_storage/src/codec/mod.rs:35-44` — `decode_record<T>`
- **Behavior Test Refs** (planned):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `trailing_bytes` proptest (PO-VB-DYBJ-014)
- **Refinement Harness Refs**:
  - `fuzz/fuzz_targets/vb_dybj_trailing_decode.rs` — cargo-fuzz target
  - Command: `cargo fuzz run vb_dybj_trailing_decode -- -max_total_time=60 -runs=10000`
  - Evidence: `#1000 DONE, no crash` (smoke bound)
- **Trust Base**: TB-VB-DYBJ-005 (fuzz smoke bound)
- **Proof Claim**: Fuzzed raw/storage bytes do not silently accept malformed trailing payloads as valid exact values.
- **Mapping Status**: mapped_existing (fuzz PASS, behavior test planned)

### PO-VB-DYBJ-016 — Migration Lifecycle (TLA+)

- **Proof Disposition**: PASS (independently verified: TLC 2.19, 52165 states, 14641 distinct, depth 9; TypeOK and NoSilentByteChangeAcceptance invariants held)
- **Rust Target**: Golden fixture migration lifecycle — temporal design evidence, not Rust implementation proof
- **Temporal-to-Rust Event/State Mapping**:
  - **TLA+ State: FixtureFrozen** → Rust: Frozen byte constants in `restate_postcard_newtype_compat_tests.rs`
  - **TLA+ State: EncodedCompared** → Rust: Postcard encode + `assert_eq!` on fixture bytes
  - **TLA+ State: MigrationRequired** → Rust: Migration-required assertion messages (PO-VB-DYBJ-017)
  - **TLA+ State: Accepted** → Rust: Golden fixture assertions pass
  - **TLA+ Transition: bytesChanged + ¬migrationNamePresent → MigrationRequired** → Rust: If fixture bytes change without updating migration documentation, tests fail
- **Source Refs** (golden fixture constants in target test file):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — frozen fixture byte constants for RunId, WorkflowDigest, RecordKind
- **Behavior Test Refs** (planned):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `migration_required` sub-tests (PO-VB-DYBJ-017)
  - Command: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests migration_required --no-fail-fast`
- **Refinement Harness Refs**:
  - `verification/tla/VbDybjGoldenFixtureLifecycle.tla` — TLA+ spec
  - `verification/tla/VbDybjGoldenFixtureLifecycle.cfg` — TLC config
  - Command: `java -jar tools/tla2tools.jar -deadlock -workers 1 -config verification/tla/VbDybjGoldenFixtureLifecycle.cfg verification/tla/VbDybjGoldenFixtureLifecycle.tla`
- **Trust Base**: TB-VB-DYBJ-006 (TLA+ model reduction)
- **Proof Claim**: Fixture lifecycle cannot transition from byte mismatch to Accepted without DeclareMigrationRequired carrying a nonempty migration name.
- **Mapping Status**: planned

---

## Owner State 8 Obligations (Implementation Domain, 6 obligations)

These obligations are owned by the implementation/test-writing state. They are included here for completeness and will be materialized when the behavior test file is written.

### PO-VB-DYBJ-003 — RunId Postcard Roundtrip / Golden Fixtures (proptest)

- **Proof Disposition**: owner_state 8 (not reviewed at State 6)
- **Rust Target**: `vb_core::ids::RunId` Postcard roundtrip + golden fixtures
- **Source Refs**:
  - `crates/vb_core/src/ids/mod.rs:65` — `numeric_id!(RunId, u64, get)`
  - `crates/vb_core/src/ids/mod.rs:12-16` — `#[derive(Serialize, Deserialize)]`
  - `crates/vb_core/src/ids/mod.rs:229-231` — `RunId::ZERO`
- **Behavior Test Refs** (to-be-written):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `run_id` proptest + fixture test
  - Command: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests run_id --no-fail-fast`
  - Required: proptest over `any::<u64>()` for `RunId::new(v).get() == v` + Postcard roundtrip; frozen fixture bytes for ZERO and MAX.
- **Refinement Harness Refs**: N/A (proptest is behavior test)
- **Mapping Status**: planned

### PO-VB-DYBJ-006 — WorkflowDigest Encode/Decode Property (proptest)

- **Proof Disposition**: owner_state 8
- **Rust Target**: `vb_core::ids::WorkflowDigest` proptest roundtrip + golden fixtures
- **Source Refs**:
  - `crates/vb_core/src/ids/mod.rs:340-356` — `WorkflowDigest` struct and methods
- **Behavior Test Refs** (to-be-written):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `workflow_digest` proptest + fixture test
  - Command: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests workflow_digest --no-fail-fast`
  - Required: proptest over `any::<[u8; 32]>()` for `from_bytes`/`as_bytes` roundtrip; frozen 32-byte fixture; all-zero and non-trivial patterns.
- **Refinement Harness Refs**: N/A
- **Mapping Status**: planned

### PO-VB-DYBJ-009 — RecordKind Named Surface Fixtures (proptest)

- **Proof Disposition**: owner_state 8
- **Rust Target**: `vb_storage::records::RecordKind` surface fixtures with explicit naming
- **Source Refs**:
  - `crates/vb_storage/src/records.rs:136-190` — `RecordKind` enum
  - `crates/vb_storage/src/records.rs:192-224` — `RecordKind::id`
- **Behavior Test Refs** (to-be-written):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `record_kind` sub-tests
  - Command: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests record_kind --no-fail-fast`
  - Required: Test names include `postcard_enum` and/or `envelope_id_u16_le`; assertions distinguish Postcard enum bytes from `RecordKind::id()` LE bytes.
- **Refinement Harness Refs**: N/A
- **Mapping Status**: planned

### PO-VB-DYBJ-011 — Missing Bytes Typed Short Error (proptest)

- **Proof Disposition**: owner_state 8
- **Rust Target**: `vb_storage::codec` short input → `JournalError::UnexpectedEof`
- **Source Refs**:
  - `crates/vb_storage/src/codec/header.rs:26-34` — `decode_record_header` `.get(..RECORD_HEADER_BYTES).ok_or(JournalError::UnexpectedEof)`
  - `crates/vb_storage/src/codec/payload.rs:62-71` — `decode_record_payload` `checked_add` and `.get` checks
  - `crates/vb_storage/src/error/mod.rs:123-125` — `JournalError::UnexpectedEof`
- **Behavior Test Refs** (to-be-written):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `missing_bytes` sub-tests
  - Command: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests missing_bytes --no-fail-fast`
  - Required: Generated short input classes assert `JournalError::UnexpectedEof`, not string messages.
- **Refinement Harness Refs**: N/A
- **Mapping Status**: planned

### PO-VB-DYBJ-017 — Migration-Required Assertions (proptest)

- **Proof Disposition**: owner_state 8
- **Rust Target**: Golden fixture mutation-sensitive assertions with migration documentation
- **Source Refs**:
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — frozen fixture byte constants
  - `crates/vb_core/src/ids/mod.rs:229-244` — `RunId`
  - `crates/vb_core/src/ids/mod.rs:340-356` — `WorkflowDigest`
  - `crates/vb_storage/src/records.rs:136-190` — `RecordKind`
- **Behavior Test Refs** (to-be-written):
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — `migration_required` sub-tests
  - Command: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests migration_required --no-fail-fast`
  - Required: Test names/messages document named migration requirement; golden byte changes produce assertion failures with migration-related messages.
- **Refinement Harness Refs**: N/A
- **Mapping Status**: planned

### PO-VB-DYBJ-018 — No Forbidden Codecs/Wrappers (source-scan)

- **Proof Disposition**: owner_state 8 (non-behavior-affecting policy check)
- **Rust Target**: Touched compatibility test file and manifests — no JSON/YAML/HTTP/Bilrost/Protobuf
- **Source Refs**:
  - `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` — compatibility test file (to-be-written)
  - `crates/workspace_tests/Cargo.toml` — test target registration
  - `Cargo.toml` — workspace dependency manifest
- **Behavior Test Refs**: N/A (source scan is a policy check, not a behavior test)
- **Refinement Harness Refs**:
  - `.beads/vb-dybj/source-scan-vb-dybj-forbidden-codecs.txt` — diff-only forbidden token scan
  - Command: `python3 scripts/check_forbidden_tokens.py --paths crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs crates/workspace_tests/Cargo.toml Cargo.toml --forbid serde_json bilrost protobuf prost tonic hyper reqwest yaml serde_yaml /tmp/opencode/restate`
  - Expected: zero forbidden tokens in touched paths; diff_added_hit_count = 0
- **Trust Base**: TB-VB-DYBJ-007 (source scan substitution)
- **Proof Claim**: Touched compatibility test and manifests introduce no JSON wrapper, Bilrost, Protobuf, HTTP, YAML runtime interpretation, or copied Restate wire/API path.
- **Mapping Status**: planned

---

## Bridge Coverage Summary

| Obligation | Verifier | State 6 Disposition | Source Refs Mapped | Behavior Test Planned | Harness Ref |
|---|---|---|---|---|---|
| PO-VB-DYBJ-001 | Verus | ACCEPTED_TRUST_BOUNDARY | Yes (RunId) | Yes (run_id) | verus/*_run_id_*.rs |
| PO-VB-DYBJ-002 | Kani | PASS | Yes (RunId+Postcard) | Yes (run_id) | kani/*_run_id_*.rs |
| PO-VB-DYBJ-003 | proptest | owner_state 8 | Yes (RunId) | Planned (run_id) | N/A |
| PO-VB-DYBJ-004 | Verus | ACCEPTED_TRUST_BOUNDARY | Yes (WorkflowDigest) | Yes (workflow_digest) | verus/*_workflow_digest_*.rs |
| PO-VB-DYBJ-005 | Flux | ACCEPTED_TRUST_BOUNDARY | Yes (WorkflowDigest) | Yes (workflow_digest) | flux/*.rs |
| PO-VB-DYBJ-006 | proptest | owner_state 8 | Yes (WorkflowDigest) | Planned (workflow_digest) | N/A |
| PO-VB-DYBJ-007 | Verus | ACCEPTED_TRUST_BOUNDARY | Yes (RecordKind) | Yes (record_kind) | verus/*_record_kind_*.rs |
| PO-VB-DYBJ-008 | Kani | ACCEPTED_TRUST_BOUNDARY | Yes (RecordKind) | Yes (record_kind) | kani/*_record_kind_*.rs |
| PO-VB-DYBJ-009 | proptest | owner_state 8 | Yes (RecordKind) | Planned (record_kind) | N/A |
| PO-VB-DYBJ-010 | Kani | ACCEPTED_TRUST_BOUNDARY | Yes (codec header/payload) | Yes (missing_bytes) | kani/*_storage_short_*.rs |
| PO-VB-DYBJ-011 | proptest | owner_state 8 | Yes (codec+error) | Planned (missing_bytes) | N/A |
| PO-VB-DYBJ-012 | cargo-fuzz | PASS | Yes (codec header/payload) | Yes (missing_bytes) | fuzz/*_storage_short_*.rs |
| PO-VB-DYBJ-013 | Kani | PASS | Yes (WorkflowDigest+codec) | Yes (trailing_bytes) | kani/*_trailing_*.rs |
| PO-VB-DYBJ-014 | proptest | PASS | Yes (WorkflowDigest+codec) | Planned (trailing_bytes) | N/A |
| PO-VB-DYBJ-015 | cargo-fuzz | PASS | Yes (WorkflowDigest+codec) | Yes (trailing_bytes) | fuzz/*_trailing_*.rs |
| PO-VB-DYBJ-016 | TLA+ | PASS | Yes (fixture lifecycle → Rust events) | Yes (migration_required) | tla/*.tla |
| PO-VB-DYBJ-017 | proptest | owner_state 8 | Yes (golden fixtures) | Planned (migration_required) | N/A |
| PO-VB-DYBJ-018 | source-scan | owner_state 8 | Yes (test/manifest files) | N/A (policy check) | source-scan-*.txt |

## Unresolved Bridge Gaps

1. **PO-VB-DYBJ-001/004/007 (Verus)**: Standalone Verus `*Model` types not mechanically bound to production `exec fn`. Production type anchors via comments/source refs. Gap: no `requires`/`ensures` on production code. Deferred to State 12.

2. **PO-VB-DYBJ-005 (Flux)**: `flux_rs` crate unresolved. Flux refinement specification cannot be verified against production types. Deferred to State 12.

3. **PO-VB-DYBJ-008/010 (Kani)**: vb_storage `cfg(kani)` compile blocked by unrelated `kani_recovery_hydrate.rs`. Selected harnesses cannot be verified. Deferred to State 12.

4. **PO-VB-DYBJ-016 (TLA+)**: Temporal model is not Rust implementation evidence. The TLA+ → Rust event/state mapping uses golden fixture constants and migration-required assertions in the test file, which are planned but not yet written. The TLA+ model provides temporal design evidence for the migration lifecycle; actual Rust implementation proof requires the behavior tests (PO-VB-DYBJ-017) to execute and pass.
