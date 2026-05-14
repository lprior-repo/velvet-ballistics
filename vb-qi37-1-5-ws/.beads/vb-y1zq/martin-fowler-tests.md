# Martin Fowler Test Plan: vb-y1zq

This is a contract-level test plan only. It does not implement tests.

## Happy Path Tests
- `given_ipc_frame_boundary_when_inventory_validates_then_fuzz_evidence_is_recorded`
  - Given: an IPC frame boundary with source path, owner, threat, class, review status, freshness marker, and fuzz evidence path.
  - When: the inventory is validated.
  - Then: validation succeeds and the boundary remains traceable to fuzz evidence.
- `given_external_binary_boundary_when_inventory_validates_then_owner_threat_and_manual_qa_are_recorded`
  - Given: a script or crate invokes an external binary and the inventory records owner, threat, and manual QA evidence.
  - When: validation runs.
  - Then: the boundary is accepted as unsafe-adjacent but not first-party unsafe.
- `given_decoder_boundary_when_inventory_validates_then_hostile_input_evidence_is_recorded`
  - Given: a decoder boundary that ingests external bytes and has fuzz or Bolero evidence.
  - When: validation runs.
  - Then: completion may proceed if all other required fields are present.

## Exact Error Path Tests
- `given_required_workspace_surfaces_missing_when_discovery_runs_then_workspace_not_discoverable_error`
  - Given: the workspace root does not expose one or more required surfaces: `crates`, `fuzz`, `scripts`, or `Cargo.toml`.
  - When: `discover_boundaries` runs.
  - Then: `Error::WorkspaceNotDiscoverable` is returned.
  - And: unsafe isolation completion is blocked.
- `given_discovery_input_omits_required_surface_when_discovery_runs_then_incomplete_discovery_input_error`
  - Given: the discovery configuration omits a required surface such as scripts, fuzz targets, decoder modules, process spawning code, C ABI declarations, IPC frame surfaces, or external binary invocations.
  - When: discovery input is validated.
  - Then: `Error::IncompleteDiscoveryInput` is returned.
  - And: no inventory can be marked complete.
- `given_unknown_boundary_class_when_inventory_validates_then_unknown_boundary_class_error`
  - Given: a boundary candidate cannot be classified as `c_abi`, `ffi`, `ipc`, `external_binary`, `decoder`, `generated_code`, or `unsafe_adjacent_dependency`.
  - When: `classify_boundary` or completion validation runs.
  - Then: `Error::UnknownBoundaryClass` is returned.
  - And: a follow-up blocker or explicit approved waiver is required.
- `given_first_party_production_unsafe_when_inventory_validates_then_unsafe_forbidden_violation`
  - Given: first-party production Rust contains unsafe usage.
  - When: inventory completion status is requested.
  - Then: `Error::UnsafeForbiddenViolation` is returned even if an inventory entry exists.
  - And: inventory completion is blocked.
- `given_missing_owner_when_inventory_validates_then_missing_owner_error`
  - Given: a boundary entry has class, source path, threat, evidence, freshness marker, and review status but no owner.
  - When: `validate_inventory` runs.
  - Then: `Error::MissingOwner` is returned.
  - And: the boundary cannot be marked complete.
- `given_missing_threat_when_inventory_validates_then_missing_threat_error`
  - Given: a boundary entry has owner, evidence, class, source path, freshness marker, and review status but no threat statement.
  - When: `validate_inventory` runs.
  - Then: `Error::MissingThreat` is returned.
  - And: the boundary cannot be marked complete.
- `given_missing_evidence_when_risky_boundary_validates_then_missing_evidence_path_error`
  - Given: a C ABI, FFI, IPC, external binary, or decoder boundary has owner and threat but no evidence path.
  - When: `validate_inventory` runs.
  - Then: `Error::MissingEvidencePath` is returned.
  - And: unsafe isolation completion is blocked.
- `given_invalid_evidence_path_when_inventory_validates_then_invalid_evidence_path_error`
  - Given: a boundary has a free-text evidence promise, broken path, malformed bead id, or non-provenance external reference.
  - When: evidence references are validated.
  - Then: `Error::InvalidEvidencePath` is returned.
  - And: the boundary cannot be marked complete.
- `given_stale_evidence_when_inventory_validates_then_stale_evidence_error`
  - Given: a boundary has evidence that predates the boundary source change or schema version requirement.
  - When: freshness is validated.
  - Then: `Error::StaleEvidence` is returned.
  - And: completion is blocked until fresh evidence is recorded.
- `given_duplicate_boundary_ids_when_inventory_validates_then_duplicate_boundary_id_error`
  - Given: two distinct normalized boundary sources produce the same stable boundary id.
  - When: inventory id uniqueness is validated.
  - Then: `Error::DuplicateBoundaryId` is returned.
  - And: completion is blocked until ids are deterministic and unique.
- `given_malformed_inventory_bytes_when_inventory_parses_then_inventory_parse_failure_error`
  - Given: the inventory artifact is malformed, truncated, or cannot be decoded according to the chosen schema.
  - When: the inventory parser runs.
  - Then: `Error::InventoryParseFailure` is returned.
  - And: no panic or partial success is allowed.
- `given_unsupported_schema_version_when_inventory_validates_then_schema_version_unsupported_error`
  - Given: an inventory has a missing, unknown, or incompatible schema version.
  - When: schema compatibility is checked.
  - Then: `Error::SchemaVersionUnsupported` is returned.
  - And: completion is blocked.
- `given_invalid_review_status_when_inventory_validates_then_review_status_invalid_error`
  - Given: a boundary has all required fields but review status is missing or not one of the allowed states.
  - When: `validate_inventory` runs.
  - Then: `Error::ReviewStatusInvalid` is returned.
  - And: the boundary cannot be marked complete.

## Edge Case Tests
- `given_empty_inventory_when_workspace_has_boundaries_then_completion_is_blocked`
- `given_no_boundaries_discovered_when_workspace_scan_is_complete_then_empty_inventory_is_valid_only_with_discovery_evidence`
- `given_same_boundary_discovered_in_different_order_when_ids_generated_then_ids_are_stable`
- `given_free_text_evidence_promise_when_inventory_validates_then_invalid_evidence_path_error`
- `given_stale_evidence_when_boundary_changed_after_evidence_then_stale_evidence_error`

## Contract Verification Tests
- `test_precondition_workspace_surfaces_are_discoverable`
- `test_precondition_discovery_input_includes_required_surfaces`
- `test_precondition_candidate_boundary_has_exactly_one_primary_class`
- `test_precondition_fallible_ops_return_result_not_panic`
- `test_postcondition_every_discovered_boundary_has_required_inventory_fields`
- `test_postcondition_external_byte_boundary_has_fuzz_or_isolation_evidence`
- `test_invariant_first_party_production_unsafe_remains_forbidden`
- `test_invariant_complete_status_requires_all_required_fields`
- `test_invariant_unknown_or_invalid_inventory_cannot_complete`
- `test_invariant_schema_version_is_checked`
- `test_every_boundary_inventory_error_has_named_error_scenario`

## Given/When/Then Scenarios

### Scenario 1: IPC frame boundary is assigned to fuzz evidence
Given: the workspace contains an IPC frame boundary that ingests external bytes.
And: the inventory entry includes source path, owner, threat, class, review status, freshness marker, and fuzz evidence path.
When: the boundary inventory is validated.
Then: validation succeeds for that boundary.
And: the traceability matrix links the boundary to the fuzz evidence artifact.

### Scenario 2: External binary boundary is documented
Given: a script or crate invokes an external executable.
And: the inventory entry records owner, threat, class `external_binary`, manual QA or isolation evidence, and review status.
When: completion status is computed.
Then: the external binary boundary is included in the completed inventory.
And: the inventory does not claim the external binary is formally safe.

### Scenario 3: Full repository inventory completes only with evidence
Given: discovery has scanned crates, fuzz targets, scripts, decoders, process-spawning code, C ABI declarations, IPC frame surfaces, and external binary invocations.
And: every discovered boundary has owner, threat, evidence path, freshness marker, and review status.
When: completion status is computed.
Then: `UnsafeIsolationStatus::Complete` may be returned.
And: every boundary traces to proof obligations and review evidence.

### Scenario 4: Invalid states fail closed
Given: inventory parsing fails, schema is stale, a source is missing, evidence is invalid, or a boundary class is unknown.
When: completion status is computed.
Then: completion is blocked by the exact `BoundaryInventoryError` variant for the violated clause.
And: no invalid state can produce success.

## Mutation Expectations
- Removing any owner, threat, evidence, schema, class, review status, or unsafe-ban check must be killed by the tests.
- Changing unknown-class handling from blocked to complete must be killed.
- Disabling first-party unsafe-forbidden detection must be killed.
- Accepting free-text evidence, stale evidence, duplicate ids, malformed inventory bytes, or invalid review status must be killed.

## Coverage Expectations
- Every error variant in `contract.md` has an exact named error-path test above.
- Every precondition, postcondition, invariant, and error variant maps to at least one proof obligation and traceability row.
- Every verification layer assigned in `verification-layers.md` appears in `proof-obligations.jsonl`.
