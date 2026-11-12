# Test Plan: vb-y1zq — Inventory unsafe-adjacent and C ABI boundaries

## Summary
- Contract status prerequisite: `contract-verification-review.md` has `STATUS: APPROVED`.
- Behaviors identified: 34.
- Trophy allocation target: 33 deletion-resistant named unit tests / 18 integration behavior groups / 2 e2e gates / 4 static/proof-focused behavior groups.
- Proptest invariants: 9.
- Fuzz targets: 3.
- Kani/proof harnesses: 7.
- Mutation threshold: `cargo-mutants` kill rate must be **>= 90%** for boundary-inventory targets; any surviving mutant in required-field, evidence, schema, unsafe-ban, or completion logic is release-blocking.
- Assertion rule: no test may assert only `is_ok()` or `is_err()`; every scenario below requires an exact accepted value, completed report field set, status value, or `BoundaryInventoryError` variant.

## Contract-Parity Constants for This Plan

These constants remove contract-parity escape hatches. If implementation wants different values or variants, it must amend `contract.md`, `proof-obligations.jsonl`, and `traceability-matrix.jsonl` before tests are written.

- Supported inventory schema versions for tests: `1` is the minimum supported version and `1` is the maximum supported version for this bead. Therefore version `1` is accepted; missing version, `0`, `2`, malformed version strings, and unknown future versions return `Err(BoundaryInventoryError::SchemaVersionUnsupported)`.
- Review status serialized enum values:
  - `approved`: valid review complete; eligible for `UnsafeIsolationStatus::Complete` when all other requirements pass.
  - `waived`: valid only when an explicit waiver artifact/reference is present; eligible for `UnsafeIsolationStatus::Complete` for known classes when all other requirements pass.
- Invalid review status values: missing/null, empty string, `pending`, `blocked`, `blocked_follow_up`, `APPROVED`, `Approved`, `reviewed`, and any unknown string. Each returns `Err(BoundaryInventoryError::ReviewStatusInvalid)` from `validate_inventory`.
- Missing source behavior: a missing, empty, or undecodable `source_path` field is malformed inventory and returns `Err(BoundaryInventoryError::InventoryParseFailure)`. A source path whose required workspace surface cannot be read returns `Err(BoundaryInventoryError::WorkspaceNotDiscoverable)`. No additional missing-source error is permitted without amending the contract artifacts.
- Unknown boundary class behavior: `unknown` or unclassified candidates always return exactly `Err(BoundaryInventoryError::UnknownBoundaryClass)` from classification/validation/completion. Unknown class never creates an inventory record and has no alternate path.
- Non-risk evidence behavior: this plan does not assert a contract-unspecified non-risk `EvidenceRequirement` variant. `required_evidence` scenarios are limited to boundaries that ingest external bytes or cross process/language/tool limits, because that is the contract guarantee.

## 1. Behavior Inventory

| ID | Behavior |
|---|---|
| B01 | Workspace discovery returns all required surfaces when `crates`, `fuzz`, `scripts`, and `Cargo.toml` are discoverable. |
| B02 | Workspace discovery returns `Error::WorkspaceNotDiscoverable` when a required workspace surface cannot be read. |
| B03 | Discovery input validation accepts a complete discovery surface set when crates, fuzz targets, scripts, Cargo.toml, decoders, process spawning, C ABI, FFI, IPC, and external binary surfaces are included. |
| B04 | Discovery input validation returns `Error::IncompleteDiscoveryInput` when any required surface class is omitted. |
| B05 | Boundary classification assigns exactly one primary class when a candidate matches `c_abi`, `ffi`, `ipc`, `external_binary`, `decoder`, `generated_code`, or `unsafe_adjacent_dependency`. |
| B06 | Boundary classification returns `Error::UnknownBoundaryClass` when a candidate cannot be mapped to an allowed primary class. |
| B07 | Fallible inventory operations return `Result<T, BoundaryInventoryError>` when validation/discovery/classification fails. |
| B08 | First-party production unsafe scanning returns `Error::UnsafeForbiddenViolation` when first-party production Rust contains `unsafe`. |
| B09 | Inventory validation accepts a fully populated IPC frame boundary when fuzz evidence is repo-local or explicitly provenanced. |
| B10 | Inventory validation accepts a fully populated external binary boundary when isolation or manual QA evidence is present. |
| B11 | Inventory validation accepts a fully populated decoder boundary when fuzz or Bolero evidence is present. |
| B12 | Inventory validation returns `Error::MissingOwner` when owner is absent. |
| B13 | Inventory validation returns `Error::MissingThreat` when threat statement is absent. |
| B14 | Inventory validation returns `Error::MissingEvidencePath` when a risky boundary lacks fuzz/isolation/manual-QA evidence. |
| B15 | Evidence validation returns `Error::InvalidEvidencePath` when evidence is free text, broken, malformed bead reference, or non-provenance external reference. |
| B16 | Evidence freshness validation returns `Error::StaleEvidence` when evidence predates the boundary source change or schema version requirement. |
| B17 | Boundary-id validation returns `Error::DuplicateBoundaryId` when distinct normalized sources produce the same id. |
| B18 | Inventory parser returns `Error::InventoryParseFailure` when inventory bytes are malformed, truncated, or undecodable. |
| B19 | Schema validation returns `Error::SchemaVersionUnsupported` when schema version is missing, unknown, or incompatible. |
| B20 | Review status validation returns `Error::ReviewStatusInvalid` when review status is absent or outside the allowed set. |
| B21 | Required evidence rules require fuzz, isolation, or manual-QA evidence when a boundary ingests external bytes or crosses process/language limits. |
| B22 | Completion status returns `UnsafeIsolationStatus::Complete` only when every discovered boundary is valid, evidenced, fresh, reviewed, and traceable. |
| B23 | Completion status returns exactly `Err(BoundaryInventoryError::UnknownBoundaryClass)` when any candidate or inventory input has class `unknown`; no inventory record, alternate path, or separate blocked status is allowed. |
| B24 | Completion status fails closed when inventory is absent, parser fails, schema is stale, source is missing, evidence is invalid, or class is unknown. |
| B25 | Inventory report distinguishes first-party unsafe-forbidden code from third-party, generated, or external unsafe-adjacent risk. |
| B26 | Traceability report links every boundary id to evidence artifact, proof obligation, and review status without prose-only claims. |
| B27 | Boundary ids remain stable and deterministic when the same boundary set is discovered in different orders. |
| B28 | Empty inventory is rejected when workspace discovery finds boundaries. |
| B29 | Empty inventory is accepted only when discovery evidence proves that no boundaries exist. |
| B30 | Free-text evidence promises never satisfy evidence requirements. |
| B31 | Release provenance records dependency and generated-artifact evidence before unsafe-isolation completion is accepted. |
| B32 | Full verification gauntlet requires `moon run :verify-proof` for Lean/Kani obligations. |
| B33 | Full release gate requires `moon run :verify-all` before release-critical unsafe-boundary inventory is accepted. |
| B34 | Manual QA transcript records every exact error variant scenario and its observed command/API outcome. |

## 2. Trophy Allocation

| Behaviors | Layer | Files/Tools | Rationale |
|---|---|---|---|
| B05, B21, B23, B24, B27 | Unit + proof kernel | `#[cfg(test)]` near pure predicates; Kani; Lean | Classification, evidence predicates, completion lattice, and deterministic id generation are pure and must be exhaustively checked. |
| B06, B12-B20, B28-B30 | Unit + integration | Unit predicate tests plus `tests/boundary_inventory_validation.rs` | Exact typed errors can be triggered through public validation APIs with real inventory records. |
| B01-B04, B08-B11, B22, B25-B26, B31, B34 | Integration | `tests/boundary_inventory_integration.rs`, fixture workspaces, real filesystem tempdirs | These behaviors cross discovery, filesystem, inventory parsing, evidence path validation, reports, and manual evidence artifacts. Use real temp workspaces, not mocks. |
| B32-B33 | E2E/acceptance | `moon run :verify-proof`, `moon run :verify-all` | User-visible release gate behavior must be tested from the outside through canonical Moon commands. |
| B07, unsafe ban, panic ban, schema compatibility, JSONL validity | Static | `moon run :verify-standard`, source scans, `jq -c`, cargo-deny where configured | Compile-time and static scans catch forbidden constructs and malformed machine-readable artifacts cheaply. |

Planned ratio by test count: 33 named unit tests, 18 integration behavior groups, 2 e2e gates, and 4 static/proof behavior groups. This intentionally exceeds the Mode 1 minimum of 25 unit tests for the 5 public contract functions so branch deletion in parser/schema/evidence/review/classification logic is mutation-visible. Static/proof share is intentionally higher than the default 5% because this bead is a release-safety inventory and has explicit Lean/Kani/static obligations.

### 2.1 Deletion-Resistant Unit Test Inventory

These mandatory unit tests are named, branch-specific, and deletion-resistant. They exercise the five public contract functions or pure predicates immediately behind those public functions, while asserting only public return values and public output records. **Count: 33 unit tests**, exceeding the required minimum of 25.

| # | Public function | Unit test name | Concrete assertion |
|---|---|---|---|
| U01 | `discover_boundaries` | `discover_boundaries_returns_workspace_not_discoverable_when_crates_surface_missing()` | returns `Err(BoundaryInventoryError::WorkspaceNotDiscoverable)` |
| U02 | `discover_boundaries` | `discover_boundaries_returns_workspace_not_discoverable_when_fuzz_surface_missing()` | returns `Err(BoundaryInventoryError::WorkspaceNotDiscoverable)` |
| U03 | `discover_boundaries` | `discover_boundaries_returns_workspace_not_discoverable_when_scripts_surface_missing()` | returns `Err(BoundaryInventoryError::WorkspaceNotDiscoverable)` |
| U04 | `discover_boundaries` | `discover_boundaries_returns_workspace_not_discoverable_when_cargo_toml_missing()` | returns `Err(BoundaryInventoryError::WorkspaceNotDiscoverable)` |
| U05 | `discover_boundaries` | `discover_boundaries_returns_incomplete_discovery_input_when_decoder_surface_omitted_from_config()` | returns `Err(BoundaryInventoryError::IncompleteDiscoveryInput)` |
| U06 | `classify_boundary` | `classify_boundary_returns_c_abi_when_candidate_declares_extern_c_boundary()` | returns `ClassifiedBoundary { class: BoundaryClass::CAbi, source_path: expected_path, .. }` |
| U07 | `classify_boundary` | `classify_boundary_returns_ffi_when_candidate_declares_foreign_function_boundary()` | returns `ClassifiedBoundary { class: BoundaryClass::Ffi, source_path: expected_path, .. }` |
| U08 | `classify_boundary` | `classify_boundary_returns_ipc_when_candidate_declares_ipc_frame_boundary()` | returns `ClassifiedBoundary { class: BoundaryClass::Ipc, source_path: expected_path, .. }` |
| U09 | `classify_boundary` | `classify_boundary_returns_external_binary_when_candidate_invokes_process_boundary()` | returns `ClassifiedBoundary { class: BoundaryClass::ExternalBinary, source_path: expected_path, .. }` |
| U10 | `classify_boundary` | `classify_boundary_returns_decoder_when_candidate_ingests_external_bytes()` | returns `ClassifiedBoundary { class: BoundaryClass::Decoder, source_path: expected_path, .. }` |
| U11 | `classify_boundary` | `classify_boundary_returns_generated_code_when_candidate_is_generated_interface()` | returns `ClassifiedBoundary { class: BoundaryClass::GeneratedCode, source_path: expected_path, .. }` |
| U12 | `classify_boundary` | `classify_boundary_returns_unsafe_adjacent_dependency_when_candidate_is_dependency_boundary()` | returns `ClassifiedBoundary { class: BoundaryClass::UnsafeAdjacentDependency, source_path: expected_path, .. }` |
| U13 | `classify_boundary` | `classify_boundary_returns_unknown_boundary_class_when_candidate_has_no_allowed_marker()` | returns `Err(BoundaryInventoryError::UnknownBoundaryClass)` |
| U14 | `required_evidence` | `required_evidence_returns_fuzz_isolation_or_manual_qa_for_c_abi_crossing_boundary()` | returns `Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa)` |
| U15 | `required_evidence` | `required_evidence_returns_fuzz_isolation_or_manual_qa_for_ffi_crossing_boundary()` | returns `Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa)` |
| U16 | `required_evidence` | `required_evidence_returns_fuzz_isolation_or_manual_qa_for_ipc_byte_boundary()` | returns `Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa)` |
| U17 | `required_evidence` | `required_evidence_returns_fuzz_isolation_or_manual_qa_for_external_binary_process_boundary()` | returns `Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa)` |
| U18 | `required_evidence` | `required_evidence_returns_fuzz_isolation_or_manual_qa_for_decoder_byte_boundary()` | returns `Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa)` |
| U19 | `validate_inventory` | `validate_inventory_returns_missing_owner_when_owner_absent()` | returns `Err(BoundaryInventoryError::MissingOwner)` |
| U20 | `validate_inventory` | `validate_inventory_returns_missing_threat_when_threat_absent()` | returns `Err(BoundaryInventoryError::MissingThreat)` |
| U21 | `validate_inventory` | `validate_inventory_returns_missing_evidence_path_when_risky_boundary_lacks_evidence()` | returns `Err(BoundaryInventoryError::MissingEvidencePath)` |
| U22 | `validate_inventory` | `validate_inventory_returns_invalid_evidence_path_when_evidence_is_free_text()` | returns `Err(BoundaryInventoryError::InvalidEvidencePath)` |
| U23 | `validate_inventory` | `validate_inventory_returns_invalid_evidence_path_when_evidence_is_absolute_outside_repo()` | returns `Err(BoundaryInventoryError::InvalidEvidencePath)` |
| U24 | `validate_inventory` | `validate_inventory_returns_stale_evidence_when_evidence_version_precedes_boundary_version()` | returns `Err(BoundaryInventoryError::StaleEvidence)` |
| U25 | `validate_inventory` | `validate_inventory_returns_duplicate_boundary_id_when_distinct_sources_share_id()` | returns `Err(BoundaryInventoryError::DuplicateBoundaryId)` |
| U26 | `validate_inventory` | `validate_inventory_returns_schema_version_unsupported_when_schema_version_missing()` | returns `Err(BoundaryInventoryError::SchemaVersionUnsupported)` |
| U27 | `validate_inventory` | `validate_inventory_accepts_schema_version_one_when_other_fields_valid()` | returns validated inventory with `schema_version == 1` |
| U28 | `validate_inventory` | `validate_inventory_returns_review_status_invalid_when_review_status_missing()` | returns `Err(BoundaryInventoryError::ReviewStatusInvalid)` |
| U29 | `validate_inventory` | `validate_inventory_accepts_review_status_approved_when_other_fields_valid()` | returns validated boundary with serialized review status `approved` |
| U30 | `validate_inventory` | `validate_inventory_accepts_review_status_waived_when_waiver_reference_exists()` | returns validated boundary with serialized review status `waived` and waiver reference preserved |
| U31 | `inventory_completion_status` | `inventory_completion_status_returns_unknown_boundary_class_when_unknown_class_present()` | returns `Err(BoundaryInventoryError::UnknownBoundaryClass)` |
| U32 | `inventory_completion_status` | `inventory_completion_status_returns_incomplete_discovery_input_when_inventory_empty_but_boundaries_discovered()` | returns `Err(BoundaryInventoryError::IncompleteDiscoveryInput)` |
| U33 | `inventory_completion_status` | `inventory_completion_status_returns_complete_when_all_boundaries_valid_fresh_reviewed_and_traceable()` | returns `Ok(UnsafeIsolationStatus::Complete)` and complete report lists every boundary id exactly once |

## 3. BDD Scenarios

### B01 — Workspace discovery returns required surfaces
- Test function: `fn discover_boundaries_returns_required_surfaces_when_workspace_is_complete()`
- Given: a temp workspace containing `crates/`, `fuzz/`, `scripts/`, and `Cargo.toml` plus fixture files for one decoder, one IPC frame, and one script external-binary call.
- When: `discover_boundaries(workspace)` runs.
- Then: returns a vector containing candidates with source paths for `crates`, `fuzz`, `scripts`, and `Cargo.toml`; expected candidate count equals the fixture count; no required surface is absent.

### B02 — Workspace discovery fails when required surface missing
- Test function: `fn discover_boundaries_returns_workspace_not_discoverable_when_required_surface_missing()`
- Given: a temp workspace with `Cargo.toml` but no `crates/` directory.
- When: `discover_boundaries(workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::WorkspaceNotDiscoverable)` and no completion status is produced.

### B03 — Discovery input validation accepts complete surface set
- Test function: `fn discovery_input_is_accepted_when_all_required_surfaces_are_present()`
- Given: discovery input enumerates crates, fuzz targets, scripts, Cargo.toml, decoders, process-spawning code, C ABI declarations, FFI declarations, IPC frame surfaces, and external binary invocations.
- When: discovery input is validated.
- Then: returns the same complete surface set with every required class marked present.

### B04 — Discovery input validation rejects omitted surface
- Test function: `fn discovery_input_returns_incomplete_discovery_input_when_required_surface_omitted()`
- Given: discovery input omits `scripts` while other required surfaces are present.
- When: discovery input is validated.
- Then: returns `Err(BoundaryInventoryError::IncompleteDiscoveryInput)`.

### B05 — Boundary classification assigns exactly one primary class
- Test function: `fn classify_boundary_returns_exactly_one_primary_class_when_candidate_matches_known_surface()`
- Given: one candidate for each allowed class: `c_abi`, `ffi`, `ipc`, `external_binary`, `decoder`, `generated_code`, and `unsafe_adjacent_dependency`.
- When: `classify_boundary(candidate)` runs for each candidate.
- Then: each result is `ClassifiedBoundary { class: <expected_class>, source_path: <expected_path>, ... }` and no candidate has multiple primary classes.

### B06 — Boundary classification rejects unknown class
- Test function: `fn classify_boundary_returns_unknown_boundary_class_when_candidate_has_no_allowed_class()`
- Given: a candidate whose metadata and path do not match any allowed primary class.
- When: `classify_boundary(candidate)` runs.
- Then: returns `Err(BoundaryInventoryError::UnknownBoundaryClass)`.

### B07 — Fallible operations return typed errors without panic
- Test function: `fn fallible_inventory_operation_returns_typed_error_when_failure_occurs()`
- Given: invalid inventory input with malformed bytes and unsupported schema marker.
- When: public fallible APIs parse and validate the input.
- Then: returns the exact first applicable `BoundaryInventoryError` variant by precedence, and the process exits normally with no panic output.

### B08 — First-party unsafe is forbidden
- Test function: `fn inventory_completion_returns_unsafe_forbidden_violation_when_first_party_production_unsafe_exists()`
- Given: a fixture first-party production Rust file under `crates/` contains an `unsafe` block, and the inventory also contains an entry for that path.
- When: `inventory_completion_status(inventory)` runs after unsafe scanning.
- Then: returns `Err(BoundaryInventoryError::UnsafeForbiddenViolation)`; the inventory entry does not convert the unsafe usage into an accepted boundary.

### B09 — IPC frame boundary accepts fuzz evidence
- Test function: `fn validate_inventory_accepts_ipc_frame_boundary_when_fuzz_evidence_is_recorded()`
- Given: an IPC frame boundary with stable id, class `ipc`, source path, owner, threat, review status, freshness marker, and repo-local fuzz evidence path.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `ValidatedBoundaryInventory` containing that boundary with class `ipc` and the same fuzz evidence path.

### B10 — External binary boundary accepts manual QA or isolation evidence
- Test function: `fn validate_inventory_accepts_external_binary_boundary_when_manual_qa_evidence_is_recorded()`
- Given: a script invokes an external executable and inventory records class `external_binary`, owner, threat, manual QA transcript path, freshness marker, and review status.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `ValidatedBoundaryInventory` containing that boundary and marks risk type as unsafe-adjacent external binary, not first-party unsafe.

### B11 — Decoder boundary accepts hostile-input evidence
- Test function: `fn validate_inventory_accepts_decoder_boundary_when_fuzz_or_bolero_evidence_is_recorded()`
- Given: a decoder boundary that ingests external bytes and has fuzz/Bolero evidence plus all required fields.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `ValidatedBoundaryInventory` containing that boundary and evidence requirement `FuzzOrIsolationOrManualQa` satisfied by fuzz/Bolero evidence.

### B12 — Missing owner error
- Test function: `fn validate_inventory_returns_missing_owner_when_owner_absent()`
- Given: a boundary entry has class, source path, threat, evidence path, freshness marker, and review status but no owner.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::MissingOwner)`.

### B13 — Missing threat error
- Test function: `fn validate_inventory_returns_missing_threat_when_threat_absent()`
- Given: a boundary entry has owner, class, source path, evidence path, freshness marker, and review status but no threat statement.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::MissingThreat)`.

### B14 — Missing evidence error for risky boundary
- Test function: `fn validate_inventory_returns_missing_evidence_path_when_risky_boundary_lacks_evidence()`
- Given: a C ABI, FFI, IPC, external binary, or decoder boundary has owner and threat but no evidence path.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::MissingEvidencePath)`.

### B15a — Invalid evidence path error for free text
- Test function: `fn evidence_validator_returns_invalid_evidence_path_when_reference_is_free_text()`
- Given: a risky boundary evidence field is `"we should fuzz this later"`.
- When: evidence references are validated.
- Then: returns `Err(BoundaryInventoryError::InvalidEvidencePath)`.

### B15b — Invalid evidence path error for outside-repo absolute path
- Test function: `fn evidence_validator_returns_invalid_evidence_path_when_reference_is_absolute_path_outside_repo()`
- Given: a risky boundary evidence field is `/tmp/evidence/report.md` or `/home/other/project/report.md`.
- When: evidence references are validated.
- Then: returns `Err(BoundaryInventoryError::InvalidEvidencePath)`.

### B15c — Invalid evidence path error for broken repo-local path
- Test function: `fn evidence_validator_returns_invalid_evidence_path_when_repo_local_reference_is_missing()`
- Given: a risky boundary evidence field is `docs/evidence/does-not-exist.md` and that repo-local artifact is absent.
- When: evidence references are validated.
- Then: returns `Err(BoundaryInventoryError::InvalidEvidencePath)`.

### B15d — Invalid evidence path error for malformed bead id
- Test function: `fn evidence_validator_returns_invalid_evidence_path_when_bead_reference_is_malformed()`
- Given: a risky boundary evidence field is `vb--bad`, `not-a-bead`, or `vb_1234` instead of a valid bead id such as `vb-y1zq`.
- When: evidence references are validated.
- Then: returns `Err(BoundaryInventoryError::InvalidEvidencePath)`.

### B15e — Invalid evidence path error for unprovenanced external URL
- Test function: `fn evidence_validator_returns_invalid_evidence_path_when_external_url_has_no_provenance()`
- Given: a risky boundary evidence field is `https://example.com/report.md` without explicit provenance metadata such as immutable digest, owner, and retrieval date.
- When: evidence references are validated.
- Then: returns `Err(BoundaryInventoryError::InvalidEvidencePath)`.

### B16 — Stale evidence error
- Test function: `fn evidence_validator_returns_stale_evidence_when_evidence_predates_boundary_or_schema()`
- Given: evidence timestamp/version is older than the boundary source fingerprint or older than the inventory schema version requirement.
- When: freshness is validated.
- Then: returns `Err(BoundaryInventoryError::StaleEvidence)`.

### B17 — Duplicate boundary id error
- Test function: `fn validate_inventory_returns_duplicate_boundary_id_when_distinct_sources_collide()`
- Given: two distinct normalized source identities resolve to the same stable boundary id.
- When: id uniqueness is validated.
- Then: returns `Err(BoundaryInventoryError::DuplicateBoundaryId)` and neither boundary is marked complete.

### B18a — Inventory parser rejects truncated bytes
- Test function: `fn parse_inventory_returns_inventory_parse_failure_when_bytes_are_truncated()`
- Given: inventory bytes end in the middle of a required record.
- When: inventory parser runs.
- Then: returns `Err(BoundaryInventoryError::InventoryParseFailure)` and no partial inventory is accepted.

### B18b — Inventory parser rejects syntactically malformed bytes
- Test function: `fn parse_inventory_returns_inventory_parse_failure_when_syntax_is_malformed()`
- Given: inventory bytes contain invalid delimiters or invalid UTF-8 for the chosen schema.
- When: inventory parser runs.
- Then: returns `Err(BoundaryInventoryError::InventoryParseFailure)` and no partial inventory is accepted.

### B18c — Inventory parser rejects random bytes
- Test function: `fn parse_inventory_returns_inventory_parse_failure_when_bytes_are_random_noise()`
- Given: inventory bytes are arbitrary random data that do not decode as the inventory schema.
- When: inventory parser runs.
- Then: returns `Err(BoundaryInventoryError::InventoryParseFailure)` and no partial inventory is accepted.

### B18d — Inventory parser rejects wrong top-level schema format
- Test function: `fn parse_inventory_returns_inventory_parse_failure_when_top_level_schema_shape_is_wrong()`
- Given: inventory bytes decode successfully as a generic document but the top-level shape is not an inventory record with schema version and boundary entries.
- When: inventory parser runs.
- Then: returns `Err(BoundaryInventoryError::InventoryParseFailure)` and no partial inventory is accepted.

### B18e — Inventory parser rejects missing source path field
- Test function: `fn parse_inventory_returns_inventory_parse_failure_when_source_path_field_is_missing()`
- Given: inventory bytes decode to a boundary record that lacks required `source_path`.
- When: inventory parser runs.
- Then: returns `Err(BoundaryInventoryError::InventoryParseFailure)`.

### B18f — Inventory parser rejects empty source path field
- Test function: `fn parse_inventory_returns_inventory_parse_failure_when_source_path_field_is_empty()`
- Given: inventory bytes decode to a boundary record with `source_path = ""`.
- When: inventory parser runs.
- Then: returns `Err(BoundaryInventoryError::InventoryParseFailure)`.

### B18g — Inventory validation rejects source path on unreadable required surface
- Test function: `fn validate_inventory_returns_workspace_not_discoverable_when_source_path_surface_cannot_be_read()`
- Given: inventory contains a non-empty source path under a required workspace surface that cannot be read.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::WorkspaceNotDiscoverable)`.

### B19a — Schema version accepts supported minimum
- Test function: `fn validate_inventory_accepts_schema_version_one_as_supported_minimum()`
- Given: inventory schema version is `1` and all other fields are valid.
- When: schema compatibility is checked.
- Then: the schema check returns accepted version `1`.

### B19b — Schema version accepts supported maximum
- Test function: `fn validate_inventory_accepts_schema_version_one_as_supported_maximum()`
- Given: inventory schema version is `1` and all other fields are valid.
- When: schema compatibility is checked.
- Then: the schema check returns accepted version `1`.

### B19c — Missing schema version error
- Test function: `fn validate_inventory_returns_schema_version_unsupported_when_schema_version_is_missing()`
- Given: inventory schema version field is absent.
- When: schema compatibility is checked.
- Then: returns `Err(BoundaryInventoryError::SchemaVersionUnsupported)`.

### B19d — Unknown schema version error
- Test function: `fn validate_inventory_returns_schema_version_unsupported_when_schema_version_is_unknown_string()`
- Given: inventory schema version is `"future-experimental"` or another non-numeric unknown value.
- When: schema compatibility is checked.
- Then: returns `Err(BoundaryInventoryError::SchemaVersionUnsupported)`.

### B19e — Lower-than-minimum schema version error
- Test function: `fn validate_inventory_returns_schema_version_unsupported_when_schema_version_is_below_minimum()`
- Given: inventory schema version is `0`.
- When: schema compatibility is checked.
- Then: returns `Err(BoundaryInventoryError::SchemaVersionUnsupported)`.

### B19f — Higher-than-maximum schema version error
- Test function: `fn validate_inventory_returns_schema_version_unsupported_when_schema_version_is_above_maximum()`
- Given: inventory schema version is `2`.
- When: schema compatibility is checked.
- Then: returns `Err(BoundaryInventoryError::SchemaVersionUnsupported)`.

### B20a — Review status `approved` is valid
- Test function: `fn validate_inventory_accepts_review_status_approved_when_other_fields_are_valid()`
- Given: a known-class boundary has every required field and `review_status = "approved"`.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `ValidatedBoundaryInventory` containing that boundary with review status `approved`.

### B20b — Review status `waived` is valid with explicit waiver evidence
- Test function: `fn validate_inventory_accepts_review_status_waived_when_explicit_waiver_evidence_exists()`
- Given: a known-class boundary has every required field, `review_status = "waived"`, and an explicit waiver artifact/reference.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `ValidatedBoundaryInventory` containing that boundary with review status `waived` and the waiver evidence path/reference.

### B20c — Review status `blocked_follow_up` is invalid
- Test function: `fn validate_inventory_returns_review_status_invalid_when_review_status_is_blocked_follow_up()`
- Given: a boundary has every required field and `review_status = "blocked_follow_up"`.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::ReviewStatusInvalid)`.

### B20d — Missing review status error
- Test function: `fn validate_inventory_returns_review_status_invalid_when_review_status_is_missing()`
- Given: a boundary has every required field except review status.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::ReviewStatusInvalid)`.

### B20e — Empty review status error
- Test function: `fn validate_inventory_returns_review_status_invalid_when_review_status_is_empty()`
- Given: a boundary has every required field and `review_status = ""`.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::ReviewStatusInvalid)`.

### B20f — Unknown review status error
- Test function: `fn validate_inventory_returns_review_status_invalid_when_review_status_is_unknown()`
- Given: a boundary has every required field and `review_status = "reviewed"`.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::ReviewStatusInvalid)`.

### B20g — Uppercase approved review status error
- Test function: `fn validate_inventory_returns_review_status_invalid_when_review_status_is_uppercase_approved()`
- Given: a boundary has every required field and `review_status = "APPROVED"`.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::ReviewStatusInvalid)`.

### B20h — Titlecase approved review status error
- Test function: `fn validate_inventory_returns_review_status_invalid_when_review_status_is_titlecase_approved()`
- Given: a boundary has every required field and `review_status = "Approved"`.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::ReviewStatusInvalid)`.

### B20i — Pending review status error
- Test function: `fn validate_inventory_returns_review_status_invalid_when_review_status_is_pending()`
- Given: a boundary has every required field and `review_status = "pending"`.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::ReviewStatusInvalid)`.

### B20j — Blocked review status error
- Test function: `fn validate_inventory_returns_review_status_invalid_when_review_status_is_blocked()`
- Given: a boundary has every required field and `review_status = "blocked"`.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns `Err(BoundaryInventoryError::ReviewStatusInvalid)`.

### B21 — Risky boundaries require evidence
- Test function: `fn required_evidence_returns_fuzz_isolation_or_manual_qa_when_boundary_ingests_bytes_or_crosses_limits()`
- Given: classified boundaries for IPC, C ABI, FFI, external binary, decoder, generated interface, and unsafe-adjacent dependency, and each fixture has at least one risk flag: byte ingestion, process limit crossing, language limit crossing, or external tool limit crossing.
- When: `required_evidence(boundary)` runs.
- Then: returns exactly `EvidenceRequirement::FuzzOrIsolationOrManualQa` for every fixture. No non-risk return value is asserted in this bead because the contract only guarantees risky-boundary evidence requirements.

### B22 — Completion succeeds only with full evidence
- Test function: `fn inventory_completion_status_returns_complete_when_all_boundaries_are_valid_evidenced_fresh_reviewed_and_traceable()`
- Given: discovery has scanned all required surfaces, every discovered boundary has stable id, class, source path, owner, threat, evidence path, freshness marker, review status, and traceability rows.
- When: `inventory_completion_status(validated_inventory)` runs.
- Then: returns `Ok(UnsafeIsolationStatus::Complete)` and the completion report lists each boundary id exactly once.

### B23 — Unknown class returns exact error with no bypass path
- Test function: `fn inventory_completion_status_returns_unknown_boundary_class_when_unknown_class_present()`
- Given: classification or inventory input contains a candidate with class `unknown`.
- When: `inventory_completion_status(inventory)` runs.
- Then: returns exactly `Err(BoundaryInventoryError::UnknownBoundaryClass)`.
- And: no inventory record is created for the unknown candidate.
- And: no inventory record, alternate path, separate blocked status, or `UnsafeIsolationStatus::Complete` is produced.

### B24 — Invalid inventory fails closed
- Test function: `fn inventory_completion_status_never_returns_complete_when_inventory_is_invalid()`
- Given: separate inventories for absent input, parse failure, stale schema, missing source field, unreadable source surface, invalid evidence, and unknown class.
- When: completion status is computed for each inventory.
- Then: each case returns the exact corresponding contract error variant: absent or syntactically invalid inventory returns `InventoryParseFailure`; unsupported schema returns `SchemaVersionUnsupported`; missing or empty source field returns `InventoryParseFailure`; unreadable required source surface returns `WorkspaceNotDiscoverable`; invalid evidence returns `InvalidEvidencePath`; unknown class returns `UnknownBoundaryClass`. No case returns `UnsafeIsolationStatus::Complete` and no uncontracted error is permitted.

### B25 — Report separates first-party unsafe from external risk
- Test function: `fn inventory_report_marks_external_or_third_party_risk_without_counting_it_as_first_party_unsafe()`
- Given: inventory includes a third-party unsafe-adjacent dependency, generated code, and external binary boundary, with no first-party production unsafe.
- When: inventory report is generated.
- Then: report classifies those entries as third-party/generated/external risk and contains zero first-party unsafe violations.

### B26 — Traceability is machine readable and complete
- Test function: `fn traceability_report_links_each_boundary_to_evidence_proof_and_review_when_inventory_complete()`
- Given: a validated inventory with N boundaries and machine-readable traceability rows.
- When: traceability report is checked with the planned JSONL checker.
- Then: every boundary id has a row with evidence artifact, proof obligation id, checker/tool, and review status; row count equals N; no evidence cell contains prose-only promises.

### B27 — Boundary ids are stable under discovery order permutation
- Test function: `fn boundary_ids_are_stable_when_same_boundaries_are_discovered_in_different_orders()`
- Given: the same set of normalized class/source identities in two different orders.
- When: boundary ids are generated for both sets.
- Then: the sorted id sets are byte-for-byte equal and each id maps to the same normalized class/source identity.

### B28 — Empty inventory rejected when boundaries exist
- Test function: `fn validate_inventory_rejects_empty_inventory_when_workspace_discovery_finds_boundaries()`
- Given: workspace discovery finds at least one IPC or decoder boundary, but inventory contains zero entries.
- When: `validate_inventory(inventory, workspace)` runs.
- Then: returns exactly `Err(BoundaryInventoryError::IncompleteDiscoveryInput)`; it must not return `UnsafeIsolationStatus::Complete` or any uncontracted missing-boundary error.

### B29 — Empty inventory accepted only with no-boundary discovery evidence
- Test function: `fn inventory_completion_status_accepts_empty_inventory_only_when_discovery_evidence_proves_no_boundaries()`
- Given: a complete discovery scan over all required surfaces returns zero candidates and the inventory includes discovery evidence proving the scan scope.
- When: `inventory_completion_status(validated_inventory)` runs.
- Then: returns `Ok(UnsafeIsolationStatus::Complete)` with `boundary_count == 0` and a discovery-evidence artifact path in the report.

### B30 — Free-text evidence never satisfies requirements
- Test function: `fn validate_inventory_returns_invalid_evidence_path_when_evidence_is_free_text_promise()`
- Given: a risky boundary has evidence value `"fuzzing planned in future"`.
- When: evidence references are validated.
- Then: returns `Err(BoundaryInventoryError::InvalidEvidencePath)`.

### B31 — Release provenance must exist
- Test function: `fn release_inventory_contains_dependency_and_generated_artifact_provenance_when_completion_is_claimed()`
- Given: a completed inventory includes unsafe-adjacent dependency and generated-code boundaries.
- When: release provenance is checked.
- Then: `formal-verification-report.md` contains dependency provenance, generated-artifact provenance, inventory evidence report path, and `REL-001` result marked passed.

### B32 — Proof gauntlet runs Lean and Kani obligations
- Test function: `fn verify_proof_runs_boundary_inventory_lean_and_kani_obligations_when_invoked()`
- Given: implementation and proof harnesses are present.
- When: `moon run :verify-proof` runs.
- Then: command exits 0 and `formal-verification-report.md` lists passed obligations `THM-POST-002`, `THM-POST-003`, `THM-INV-002`, `THM-INV-003`, `THM-INV-004`, `THM-INV-005`, `PRE-003-KANI`, `POST-004-KANI`, `INV-002-KANI`, `INV-004-KANI`, and `INV-005-KANI`.

### B33 — Full release gate runs complete unsafe-boundary verification
- Test function: `fn verify_all_accepts_release_inventory_only_when_all_required_evidence_is_present()`
- Given: all inventory, proof, fuzz, mutation, static, manual QA, and release provenance artifacts are generated.
- When: `moon run :verify-all` runs.
- Then: command exits 0 and `formal-verification-report.md` marks `GATE-002` and `REL-001` passed.

### B34 — Manual QA transcript covers every error variant
- Test function: `fn manual_qa_transcript_records_every_boundary_inventory_error_variant_when_release_evidence_is_collated()`
- Given: manual QA has executed the exact error scenarios from `martin-fowler-tests.md`.
- When: manual QA transcript is collated into `formal-verification-report.md`.
- Then: the report contains one observed scenario for each error variant: `WorkspaceNotDiscoverable`, `IncompleteDiscoveryInput`, `UnknownBoundaryClass`, `UnsafeForbiddenViolation`, `MissingOwner`, `MissingThreat`, `MissingEvidencePath`, `InvalidEvidencePath`, `StaleEvidence`, `DuplicateBoundaryId`, `InventoryParseFailure`, `SchemaVersionUnsupported`, and `ReviewStatusInvalid`.

## 4. Inventory Fixtures and Boundary Scan Tests

### Required fixture workspaces
Create fixtures under the eventual test fixture directory; names are normative, paths may be adapted to repo conventions:

| Fixture | Contents | Expected result |
|---|---|---|
| `complete_workspace` | `crates/`, `fuzz/`, `scripts/`, `Cargo.toml`; one IPC frame, one decoder, one external binary script, one generated interface marker, one unsafe-adjacent dependency. | Discovery returns all expected candidates; validation can complete when inventory is fully evidenced. |
| `missing_crates_workspace` | `Cargo.toml`, `fuzz/`, `scripts/`, no `crates/`. | `Err(WorkspaceNotDiscoverable)`. |
| `omitted_surface_config` | Discovery config excludes one required surface class. | `Err(IncompleteDiscoveryInput)`. |
| `first_party_unsafe_workspace` | Production file under `crates/` with unsafe usage. | `Err(UnsafeForbiddenViolation)`. |
| `unknown_boundary_workspace` | Boundary marker/path not matching any allowed class. | `Err(UnknownBoundaryClass)`. |
| `no_boundary_workspace` | All required surfaces present, no matching boundaries, discovery evidence present. | `Ok(UnsafeIsolationStatus::Complete)` with zero boundaries. |
| `malformed_inventory_bytes` | Truncated/wrong-format inventory artifact. | `Err(InventoryParseFailure)`. |
| `stale_schema_inventory` | Inventory with unsupported schema version. | `Err(SchemaVersionUnsupported)`. |
| `bad_evidence_inventory` | Free-text, broken path, outside-repo absolute path, malformed bead id, and unprovenanced URL evidence examples. | `Err(InvalidEvidencePath)`. |
| `stale_evidence_inventory` | Evidence timestamp/version older than boundary source/schema. | `Err(StaleEvidence)`. |

### Boundary scan coverage
- Scan `crates/**` for first-party production unsafe, decoder modules, IPC frame surfaces, C ABI/FFI declarations, generated interfaces, and process-spawning code.
- Scan `fuzz/**` for existing fuzz targets and evidence references.
- Scan `scripts/**` for external binary invocations and process-limit crossings.
- Scan `Cargo.toml` and lock/dependency metadata for unsafe-adjacent dependencies.
- Scan generated-code locations, if any, separately from first-party production code.
- Boundary scan assertions must compare exact boundary ids/classes/source paths against expected fixture manifests.

## 5. Proptest Invariants

### P01 — Classification is total over known classes and exclusive
- Target: `classify_boundary` / pure classifier predicate.
- Invariant: every generated candidate with one known surface marker returns exactly one matching primary class and never a set of classes.
- Strategy: generate candidates tagged with exactly one of `c_abi`, `ffi`, `ipc`, `external_binary`, `decoder`, `generated_code`, `unsafe_adjacent_dependency` plus normalized source paths.
- Anti-invariant: generated candidates with no known marker return `Err(BoundaryInventoryError::UnknownBoundaryClass)`.

### P02 — Complete discovery input is required
- Target: discovery input validator.
- Invariant: removing any required surface class from the complete set returns `Err(BoundaryInventoryError::IncompleteDiscoveryInput)`.
- Strategy: generate subsets of the required surface enum set.
- Anti-invariant: any strict subset fails; the full set returns the complete set.

### P03 — Required field completeness gates validation
- Target: `validate_inventory` pure record predicate.
- Invariant: a boundary can validate only when id, class, source path, owner, threat, evidence path, freshness marker, and review status are present and valid.
- Strategy: generate `BoundaryRecord` values with optional required fields.
- Anti-invariant: missing owner/threat/evidence/review status returns the exact corresponding error variant.

### P04 — Risky boundaries imply evidence requirement
- Target: `required_evidence`.
- Invariant: if `ingests_external_bytes || crosses_process_limit || crosses_language_limit`, then evidence requirement is `FuzzOrIsolationOrManualQa`.
- Strategy: generate classes and boolean risk flags.
- Anti-invariant: risky boundary with `None` evidence returns `Err(BoundaryInventoryError::MissingEvidencePath)` during validation.

### P05 — Evidence references are syntactically constrained
- Target: evidence reference validator.
- Invariant: accepted evidence references are repo-local paths, valid bead ids, or explicit external provenance references.
- Strategy: generate repo-relative paths, bead-like ids, external references with/without provenance metadata, absolute paths, and free text.
- Anti-invariant: free text, broken paths, outside-repo paths, malformed bead ids, and unprovenanced URLs return `Err(BoundaryInventoryError::InvalidEvidencePath)`.

### P06 — Freshness is monotonic
- Target: evidence freshness validator.
- Invariant: evidence is accepted only when evidence version/timestamp is >= boundary source version/timestamp and >= schema evidence requirement version.
- Strategy: generate ordered triples `(source_version, schema_version, evidence_version)`.
- Anti-invariant: `evidence_version < max(source_version, schema_version)` returns `Err(BoundaryInventoryError::StaleEvidence)`.

### P07 — Boundary ids are stable under permutation
- Target: boundary id generator.
- Invariant: for a set of unique normalized `(class, source_identity)` pairs, any permutation produces the same set of ids and same id-to-source mapping.
- Strategy: generate non-empty vectors of unique normalized class/source pairs and random permutations.
- Anti-invariant: duplicate normalized identities or forced id collision returns `Err(BoundaryInventoryError::DuplicateBoundaryId)`.

### P08 — Completion status fails closed
- Target: `inventory_completion_status` pure completion lattice.
- Invariant: any invalid inventory state maps to an error/blocker and never maps to `UnsafeIsolationStatus::Complete`.
- Strategy: generate completion states with flags for parse failure, schema unsupported, missing source, unknown class, invalid evidence, stale evidence, missing fields.
- Anti-invariant: invalid flags must fail with the exact contract error mapping fixed in this plan: parse/missing-source-field -> `InventoryParseFailure`; unsupported schema -> `SchemaVersionUnsupported`; unreadable required source surface -> `WorkspaceNotDiscoverable`; unknown class -> `UnknownBoundaryClass`; invalid evidence -> `InvalidEvidencePath`; stale evidence -> `StaleEvidence`; missing owner/threat/evidence/review -> their named contract variants.

### P09 — Schema version compatibility is explicit
- Target: inventory schema version validator.
- Invariant: only supported schema versions validate; missing/unknown/incompatible versions return `Err(BoundaryInventoryError::SchemaVersionUnsupported)`.
- Strategy: generate optional semantic/schema version values around supported min/max and random strings.
- Anti-invariant: `None`, malformed, and incompatible versions fail with `SchemaVersionUnsupported`.

## 6. Fuzz Targets

### F01 — Inventory parser hostile bytes
- Target: inventory parser for the chosen machine-checkable inventory schema.
- Input type: raw `&[u8]`.
- Risk class: panic, partial success, parser differential, OOM, invalid schema bypass.
- Corpus seeds: empty bytes, single `{`, truncated JSON/TOML/CUE-like record, wrong top-level type, valid minimal inventory, valid full inventory, unsupported schema version, duplicate boundary ids, huge strings, invalid UTF-8 bytes.
- Expected oracle: parser returns either a fully decoded inventory with supported schema or `Err(BoundaryInventoryError::InventoryParseFailure | BoundaryInventoryError::SchemaVersionUnsupported)`; it never panics and never returns partial success.

### F02 — Boundary metadata and evidence requirement bypass
- Target: boundary metadata decoder plus `required_evidence`/validation handoff.
- Input type: arbitrary structured boundary metadata or raw bytes decoded into metadata.
- Risk class: hostile metadata evades risky-boundary evidence requirement or injects unknown class as complete instead of `UnknownBoundaryClass`.
- Corpus seeds: class strings for each allowed class, `unknown`, mixed-case class, empty class, path traversal source path, free-text evidence, repo-local evidence, bead id evidence, unprovenanced external URL.
- Expected oracle: risky byte/process/language-crossing inputs without valid evidence return `MissingEvidencePath` or `InvalidEvidencePath`; unknown class returns `UnknownBoundaryClass`.

### F03 — Evidence reference parser
- Target: evidence reference parser/validator.
- Input type: arbitrary UTF-8 strings and raw bytes if the implementation accepts bytes.
- Risk class: path traversal, outside-repo path acceptance, malformed bead id acceptance, URL/provenance confusion, panic on invalid UTF-8.
- Corpus seeds: `formal-verification-report.md`, `.beads/vb-y1zq/test-plan.md`, `vb-y1zq`, `../outside`, `/tmp/evidence`, `https://example.com/report`, `external:vendor/report#sha256=abc`, empty string, NUL-containing string, very long string.
- Expected oracle: only repo-local artifacts, valid bead ids, or explicit provenance references validate; all others return `InvalidEvidencePath`.

## 7. Kani Harnesses and Proof Obligations

### K01 — Classification exclusivity (`PRE-003-KANI`)
- Property: bounded classification model has no success state with zero classes or more than one primary class.
- Bound: all class enum values plus boolean marker combinations up to 8 markers.
- Rationale: prevents ambiguous risk ownership and evidence assignment.

### K02 — Fallible operations never panic (`PRE-004-KANI`)
- Property: bounded invalid inputs return `BoundaryInventoryError` variants and do not invoke panic paths.
- Bound: all error-triggering enum states and inventory records with optional required fields.
- Rationale: contract forbids unchecked panic paths for fallible inventory operations.

### K03 — Required fields completeness (`INV-002-KANI`, `POST-004-KANI`)
- Property: validation cannot produce a complete record when owner, threat, evidence, class, source path, or review status is absent/invalid.
- Bound: presence bitset for required fields (`2^8` combinations) and allowed review status enum.
- Rationale: required-field omission is release-critical.

### K04 — Risk evidence implication (`THM-POST-002`, `THM-INV-003` refinement)
- Property: any boundary with byte-ingest or process/language-crossing flags requires fuzz, isolation, or manual-QA evidence before completion.
- Bound: all class enum values and all risk-flag combinations.
- Rationale: proves the core unsafe-adjacent safety rule.

### K05 — Boundary id determinism and uniqueness (`INV-004-KANI`)
- Property: unique normalized source identities produce unique ids within the bounded model; duplicate ids are rejected.
- Bound: up to 16 boundaries, class enum, normalized source identity length bounded by 64 bytes.
- Rationale: stable traceability depends on deterministic ids independent of discovery order.

### K06 — Fail-closed completion lattice (`INV-005-KANI`)
- Property: absent inventory, parse failure, stale schema, missing source, unknown class, invalid evidence, stale evidence, or missing required field cannot produce `UnsafeIsolationStatus::Complete`.
- Bound: complete invalid-state boolean lattice (`2^N` invalid flags) plus success state.
- Rationale: completion is a release gate and must be impossible from invalid states.

### K07 — Duplicate id exact error (`ERR-010`)
- Property: if two distinct normalized records share an id, validation returns `Err(BoundaryInventoryError::DuplicateBoundaryId)`.
- Bound: two-record and small-vector inventories up to 8 records.
- Rationale: exact error variant coverage for collision behavior.

Lean theorem obligations from `verification-layers.md` must remain mapped in `formal-verification-report.md`: `required_evidence_assigned_for_risky_classes`, `unknown_class_blocks_completion`, `complete_requires_required_fields`, `crossing_boundary_requires_evidence`, `stable_ids_are_unique_when_sources_unique`, and `invalid_inventory_cannot_complete`.

## 8. Mutation Testing Checkpoints

| Mutation | Must be killed by |
|---|---|
| Remove required `owner` check | `validate_inventory_returns_missing_owner_when_owner_absent` |
| Remove required `threat` check | `validate_inventory_returns_missing_threat_when_threat_absent` |
| Remove evidence requirement for IPC/decoder/external binary | `validate_inventory_returns_missing_evidence_path_when_risky_boundary_lacks_evidence`; P04 |
| Accept free-text evidence as valid | `validate_inventory_returns_invalid_evidence_path_when_evidence_is_free_text_promise`; P05; F03 |
| Accept outside-repo absolute path as evidence | `evidence_validator_returns_invalid_evidence_path_when_reference_is_absolute_path_outside_repo`; F03 |
| Accept broken repo-local path as evidence | `evidence_validator_returns_invalid_evidence_path_when_repo_local_reference_is_missing`; F03 |
| Accept malformed bead id as evidence | `evidence_validator_returns_invalid_evidence_path_when_bead_reference_is_malformed`; F03 |
| Accept unprovenanced external URL as evidence | `evidence_validator_returns_invalid_evidence_path_when_external_url_has_no_provenance`; F03 |
| Ignore evidence freshness comparison | `evidence_validator_returns_stale_evidence_when_evidence_predates_boundary_or_schema`; P06 |
| Permit duplicate boundary id by overwriting map entry | `validate_inventory_returns_duplicate_boundary_id_when_distinct_sources_collide`; K07 |
| Change unknown class to generated code | `classify_boundary_returns_unknown_boundary_class_when_candidate_has_no_allowed_class` |
| Allow unknown class to complete | `inventory_completion_status_returns_unknown_boundary_class_when_unknown_class_present`; K06 |
| Create inventory record for unknown class | `inventory_completion_status_returns_unknown_boundary_class_when_unknown_class_present`; K06 |
| Add alternate non-error handling for unknown class | `inventory_completion_status_returns_unknown_boundary_class_when_unknown_class_present`; K06 |
| Remove unsafe scan from completion | `inventory_completion_returns_unsafe_forbidden_violation_when_first_party_production_unsafe_exists` |
| Treat third-party unsafe-adjacent dependency as first-party unsafe | `inventory_report_marks_external_or_third_party_risk_without_counting_it_as_first_party_unsafe` |
| Ignore schema version field | `validate_inventory_returns_schema_version_unsupported_when_schema_version_is_missing`; P09 |
| Accept schema version `0` | `validate_inventory_returns_schema_version_unsupported_when_schema_version_is_below_minimum`; P09 |
| Accept schema version `2` | `validate_inventory_returns_schema_version_unsupported_when_schema_version_is_above_maximum`; P09 |
| Accept unknown schema string | `validate_inventory_returns_schema_version_unsupported_when_schema_version_is_unknown_string`; P09 |
| Reject supported schema version `1` | `validate_inventory_accepts_schema_version_one_as_supported_minimum`; `validate_inventory_accepts_schema_version_one_as_supported_maximum` |
| Accept truncated inventory bytes as empty inventory | `parse_inventory_returns_inventory_parse_failure_when_bytes_are_truncated`; F01 |
| Accept random inventory bytes as empty inventory | `parse_inventory_returns_inventory_parse_failure_when_bytes_are_random_noise`; F01 |
| Accept wrong top-level inventory shape | `parse_inventory_returns_inventory_parse_failure_when_top_level_schema_shape_is_wrong`; F01 |
| Accept missing source path field | `parse_inventory_returns_inventory_parse_failure_when_source_path_field_is_missing` |
| Accept empty source path field | `parse_inventory_returns_inventory_parse_failure_when_source_path_field_is_empty` |
| Use discovery order in boundary id generation | `boundary_ids_are_stable_when_same_boundaries_are_discovered_in_different_orders`; P07 |
| Remove review status allow-list | `validate_inventory_returns_review_status_invalid_when_review_status_is_unknown`; `validate_inventory_returns_review_status_invalid_when_review_status_is_uppercase_approved`; `validate_inventory_returns_review_status_invalid_when_review_status_is_titlecase_approved`; `validate_inventory_returns_review_status_invalid_when_review_status_is_pending`; `validate_inventory_returns_review_status_invalid_when_review_status_is_blocked` |
| Reject valid review status `approved` | `validate_inventory_accepts_review_status_approved_when_other_fields_are_valid` |
| Reject valid review status `waived` with waiver evidence | `validate_inventory_accepts_review_status_waived_when_explicit_waiver_evidence_exists` |
| Accept `blocked_follow_up` as valid review status | `validate_inventory_returns_review_status_invalid_when_review_status_is_blocked_follow_up` |
| Convert invalid inventory to `Complete` | `inventory_completion_status_never_returns_complete_when_inventory_is_invalid`; K06 |

Required checkpoint command: `moon run :verify-deep` must run mutation and fuzz lanes. If `cargo-mutants` is invoked directly, scope it to the boundary inventory module and require `>= 90%` kill rate with zero surviving critical mutants from the table above.

## 9. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| Complete workspace discovery | all required surfaces present | candidate set equals fixture manifest | integration |
| Missing `crates/` | required surface absent | `Err(WorkspaceNotDiscoverable)` | integration |
| Omitted `scripts` discovery surface | strict subset of required surfaces | `Err(IncompleteDiscoveryInput)` | unit/integration |
| Known class classification | one marker for each allowed class | exact `ClassifiedBoundary.class` per marker | unit |
| Unknown class classification | no allowed marker | `Err(UnknownBoundaryClass)` | unit |
| First-party production unsafe | unsafe block in `crates/` | `Err(UnsafeForbiddenViolation)` | integration/static |
| Fully evidenced IPC | class `ipc`, fuzz evidence | validated boundary with same evidence path | integration |
| Fully evidenced external binary | class `external_binary`, manual QA evidence | validated external-binary boundary | integration |
| Fully evidenced decoder | class `decoder`, fuzz/Bolero evidence | validated decoder boundary | integration |
| Missing owner | owner absent | `Err(MissingOwner)` | unit/integration |
| Missing threat | threat absent | `Err(MissingThreat)` | unit/integration |
| Missing risky evidence | IPC/FFI/C ABI/decoder/external binary with no evidence | `Err(MissingEvidencePath)` | unit/integration |
| Invalid evidence: free text | `"we will fuzz later"` | `Err(InvalidEvidencePath)` | unit/proptest |
| Invalid evidence: outside path | `/tmp/report.md` | `Err(InvalidEvidencePath)` | unit/proptest |
| Invalid evidence: broken repo path | missing file path | `Err(InvalidEvidencePath)` | integration |
| Stale evidence | evidence version older than source/schema | `Err(StaleEvidence)` | unit/proptest |
| Duplicate id | two distinct sources same id | `Err(DuplicateBoundaryId)` | unit/Kani |
| Truncated inventory bytes | mid-record EOF | `Err(InventoryParseFailure)` | fuzz/integration |
| Malformed inventory syntax | invalid delimiters/UTF-8 | `Err(InventoryParseFailure)` | fuzz/integration |
| Random inventory bytes | arbitrary noise | `Err(InventoryParseFailure)` | fuzz/integration |
| Wrong top-level inventory shape | decodes but not inventory root | `Err(InventoryParseFailure)` | fuzz/integration |
| Missing source path field | no `source_path` | `Err(InventoryParseFailure)` | unit/integration |
| Empty source path field | `source_path = ""` | `Err(InventoryParseFailure)` | unit/integration |
| Unreadable source surface | source path under unreadable required surface | `Err(WorkspaceNotDiscoverable)` | integration |
| Missing schema version | no version field | `Err(SchemaVersionUnsupported)` | unit/api-compat |
| Unknown schema version | unsupported version string | `Err(SchemaVersionUnsupported)` | unit/api-compat |
| Lower schema version | version `0` | `Err(SchemaVersionUnsupported)` | unit/api-compat |
| Higher schema version | version `2` | `Err(SchemaVersionUnsupported)` | unit/api-compat |
| Supported schema min/max | version `1` | accepted version `1` | unit/api-compat |
| Review status approved | `approved` | validated boundary contains `approved` | unit/integration |
| Review status waived | `waived` with waiver evidence | validated boundary contains `waived` and waiver reference | unit/integration |
| Review status blocked follow-up | `blocked_follow_up` | `Err(ReviewStatusInvalid)` | integration |
| Missing review status | status absent | `Err(ReviewStatusInvalid)` | unit/integration |
| Empty review status | `""` | `Err(ReviewStatusInvalid)` | unit/integration |
| Unknown review status | `reviewed` | `Err(ReviewStatusInvalid)` | unit/integration |
| Uppercase approved review status | `APPROVED` | `Err(ReviewStatusInvalid)` | unit/integration |
| Titlecase approved review status | `Approved` | `Err(ReviewStatusInvalid)` | unit/integration |
| Pending review status | `pending` | `Err(ReviewStatusInvalid)` | unit/integration |
| Blocked review status | `blocked` | `Err(ReviewStatusInvalid)` | unit/integration |
| Stable ids | same set, different order | identical sorted id set | unit/proptest |
| Empty inventory with boundaries | discovery count > 0, inventory count 0 | `Err(IncompleteDiscoveryInput)`; never `Complete` | integration |
| Empty inventory with no boundaries | discovery count 0 and scan evidence present | `Ok(UnsafeIsolationStatus::Complete)` with `boundary_count == 0` | integration |
| Invalid completion state | parse/schema/source/evidence/class invalid | exact typed error; never `Complete` | Kani/proptest |
| Traceability report | N validated boundaries | N machine-readable rows with evidence/proof/review | integration/static |
| Release provenance | dependency/generated boundaries | `REL-001` evidence present | e2e |
| Proof gate | proof harnesses present | `moon run :verify-proof` exit 0 and obligations passed | e2e |
| Full gate | all evidence present | `moon run :verify-all` exit 0 and `GATE-002` passed | e2e |

## 10. Required Commands and Evidence

Red phase commands before implementation should fail because APIs/tests/harnesses are not implemented yet:

| Phase | Command | Red-phase expectation | Green acceptance |
|---|---|---|---|
| Static/source governance | `moon run :verify-standard` | Fails on missing inventory implementation/tests or missing static check wiring. | Exits 0; unsafe ban, panic-path scan, schema compatibility, and JSONL/static checks pass. |
| Unit/integration tests | `moon run :test` or repository test lane used by Moon | Fails with missing public API/module or failing behavior tests. | Exits 0; all exact-value scenarios pass. |
| Fast gate | `moon run :verify-fast` | Fails until workspace discovery checks are wired. | Exits 0 and records `PRE-001-GATE`. |
| Deep gate | `moon run :verify-deep` | Fails until fuzz and mutation targets exist and execute. | Exits 0; fuzz obligations pass; mutation kill rate >= 90%. |
| Proof gate | `moon run :verify-proof` | Fails until Lean/Kani obligations are implemented. | Exits 0; all listed Lean/Kani obligations pass. |
| Release gate | `moon run :verify-all` | Fails until all evidence artifacts and provenance exist. | Exits 0; `GATE-002` and `REL-001` are passed in `formal-verification-report.md`. |
| JSONL traceability | `jq -c . traceability-matrix.jsonl` and equivalent generated-report check | Fails on malformed rows or missing generated traceability. | Exits 0 and every boundary id maps to evidence/proof/review. |

Required final evidence artifacts:
- `formal-verification-report.md` with Lean, Kani, static, fuzz, mutation, coverage, schema compatibility, manual QA, release provenance, and gauntlet results.
- Inventory evidence report containing each boundary id, class, source path, owner, threat, evidence path, freshness marker, and review status.
- Manual QA transcript demonstrating all 13 `BoundaryInventoryError` variants.
- Mutation report proving `>= 90%` kill rate and no surviving critical mutant.
- Fuzz report for inventory parser, boundary metadata, and evidence reference parser.

## 11. Red-Phase Expectations

- Tests must be written before implementation and initially fail for the right reason: missing API, missing inventory schema, missing validation branch, missing evidence artifact, or missing Moon lane.
- Red tests must not fail due to syntax errors in test files, invalid fixture paths, nondeterministic time assumptions, or reliance on private implementation details.
- Each error-path red test must fail until the exact `BoundaryInventoryError` variant is returned; a generic error string is not acceptable.
- Completion-status tests must fail until invalid states are proven unable to return `UnsafeIsolationStatus::Complete`.
- Fuzz and mutation lanes may be marked expected-fail during red phase only until harnesses exist; after implementation, expected-fail markers must be removed.

## 12. Exit Criteria for Test Writer

- Every public contract signature has behavior tests through public APIs: `discover_boundaries`, `classify_boundary`, `validate_inventory`, `required_evidence`, and `inventory_completion_status`.
- Every `BoundaryInventoryError` variant has an exact BDD scenario and exact assertion.
- Every parser/deserializer/user-input boundary has a fuzz target.
- Every pure function with multiple inputs has a proptest invariant.
- Kani/Lean obligations match `proof-obligations.jsonl` and appear in `formal-verification-report.md`.
- Mutation threshold `>= 90%` is enforced; critical mutants listed in Section 8 must be killed.
- No assertion only checks `is_ok()` or `is_err()`; expected accepted value or exact error variant is always asserted.

## Open Questions

- The final inventory file name is intentionally not fixed by State 1; tests should bind to the chosen public parser/validator once implementation selects the file location, while preserving schema version `1`, the concrete review-status values above, and the exact error taxonomy above.
