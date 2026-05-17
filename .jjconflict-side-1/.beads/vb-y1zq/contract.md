# Contract Specification: vb-y1zq

## Context
- Feature: Maintain an explicit inventory of unsafe-adjacent, C ABI, IPC, external binary, FFI, and decoder boundaries.
- Purpose: Ensure every boundary that can ingest external bytes or cross process/language/tool limits has an owner, threat statement, and evidence path before unsafe isolation is treated as complete.
- Domain terms:
  - Boundary: Any code path, artifact, process call, data format, protocol, decoder, generated interface, or external executable interface where trust changes.
  - Unsafe-adjacent: A boundary that does not require first-party `unsafe` but can expose memory, layout, byte-decoding, process, or ABI risk.
  - Evidence path: A concrete artifact path or bead reference that proves fuzzing, isolation, static scanning, manual QA, or formal verification coverage.
  - Owner: A named role, team, bead, or maintainer accountable for keeping the inventory entry current.
- Assumptions:
  - First-party production Rust remains unsafe-forbidden.
  - Inventory implementation may be a checked document, manifest, generated report, or typed domain model, but must be machine-checkable.
  - The discovery surface includes `crates`, `fuzz`, `scripts`, `Cargo.toml`, and any external interface referenced from them.
- Open questions:
  - Final inventory file name and schema are not fixed by this State 1 contract.
  - Owner taxonomy is not fixed; downstream work must choose a repo-appropriate owner encoding.

## Preconditions
- PRE-001: The workspace root, `crates`, `fuzz`, `scripts`, and `Cargo.toml` are discoverable before inventory generation or validation begins.
- PRE-002: The inventory input set must include all known first-party crates, fuzz targets, scripts, decoder modules, process-spawning code, C ABI declarations, IPC frame surfaces, and external binary invocations.
- PRE-003: Each candidate boundary must be classified into exactly one primary class: `c_abi`, `ffi`, `ipc`, `external_binary`, `decoder`, `generated_code`, `unsafe_adjacent_dependency`, or `unknown`.
- PRE-004: Fallible operations must return `Result<T, BoundaryInventoryError>` and must not terminate through unchecked panic paths.
- PRE-005: No inventory entry may rely on first-party production `unsafe`; any discovered first-party production unsafe usage is a release blocker separate from inventory completion.

## Postconditions
- POST-001: Every discovered boundary has an inventory entry containing stable id, class, source path, owner, threat, verification evidence path, freshness marker, and review status.
- POST-002: Every boundary that ingests external bytes or crosses a process/language boundary is assigned fuzz, isolation, or manual QA evidence.
- POST-003: Every `unknown` boundary class produces a blocking follow-up issue or explicit waiver and prevents unsafe-isolation-complete status.
- POST-004: Missing owner, missing threat, missing evidence path, invalid path, or stale evidence produces a typed error and prevents completion.
- POST-005: The inventory distinguishes first-party unsafe-forbidden production code from third-party, generated, or external unsafe-adjacent risk.
- POST-006: The completed inventory can be traced from each boundary to evidence artifacts and review status without relying on prose-only claims.

## Invariants
- INV-001: First-party production code remains unsafe-forbidden regardless of the existence of an inventory entry.
- INV-002: No boundary can be marked `complete` unless owner, threat, evidence path, class, source path, and review status are present and valid.
- INV-003: A boundary that ingests external bytes or crosses process/language limits must have fuzz, isolation, or manual QA evidence.
- INV-004: Boundary ids are stable, unique, deterministic, and derived from normalized class plus source identity, not discovery order.
- INV-005: Inventory validation is fail-closed: absence, parse failure, stale schema, missing source, or unknown class cannot produce success.
- INV-006: Evidence paths must point to repo-local artifacts, bead ids, or explicit external provenance references; free-text promises are not sufficient evidence.
- INV-007: The inventory schema is versioned so future incompatible changes can be detected.

## Error Taxonomy
- Error::WorkspaceNotDiscoverable - PRE-001 fails because required workspace surfaces cannot be read.
- Error::IncompleteDiscoveryInput - PRE-002 fails because required surfaces were omitted.
- Error::UnknownBoundaryClass - PRE-003/POST-003 fails for an unclassified boundary.
- Error::UnsafeForbiddenViolation - PRE-005/INV-001 fails because first-party production unsafe usage is discovered.
- Error::MissingOwner - POST-001/POST-004/INV-002 fails because owner is absent.
- Error::MissingThreat - POST-001/POST-004/INV-002 fails because threat is absent.
- Error::MissingEvidencePath - POST-001/POST-004/INV-002/INV-003 fails because required evidence is absent.
- Error::InvalidEvidencePath - POST-004/INV-006 fails because evidence cannot be resolved.
- Error::StaleEvidence - POST-004 fails because evidence predates the boundary or schema version.
- Error::DuplicateBoundaryId - INV-004 fails because stable ids collide.
- Error::InventoryParseFailure - INV-005 fails because inventory cannot be parsed or decoded.
- Error::SchemaVersionUnsupported - INV-007 fails because inventory schema is missing or incompatible.
- Error::ReviewStatusInvalid - POST-001/INV-002 fails because review status is absent or not allowed.

## Error Coverage Requirement
Every `BoundaryInventoryError` variant above must have all of the following before implementation begins:
- one exact Given/When/Then scenario in `martin-fowler-tests.md` naming the expected variant;
- one executable proof obligation in `proof-obligations.jsonl`;
- one traceability row in `traceability-matrix.jsonl` linking the scenario, obligation, checker, evidence, and review file.

## Contract Signatures
- `fn discover_boundaries(workspace: WorkspaceRoot) -> Result<Vec<BoundaryCandidate>, BoundaryInventoryError>`
- `fn classify_boundary(candidate: BoundaryCandidate) -> Result<ClassifiedBoundary, BoundaryInventoryError>`
- `fn validate_inventory(inventory: BoundaryInventory, workspace: WorkspaceRoot) -> Result<ValidatedBoundaryInventory, BoundaryInventoryError>`
- `fn required_evidence(boundary: ClassifiedBoundary) -> Result<EvidenceRequirement, BoundaryInventoryError>`
- `fn inventory_completion_status(inventory: ValidatedBoundaryInventory) -> Result<UnsafeIsolationStatus, BoundaryInventoryError>`

## Lean-Owned Clauses
- INV-002: Completeness predicate over pure inventory records.
- INV-003: Evidence requirement predicate for byte-ingesting and process/language-crossing classes.
- INV-004: Deterministic uniqueness model for boundary ids.
- INV-005: Fail-closed completion lattice.

## Non-goals
- This state does not implement the inventory, schema, production code, proof code, harnesses, or tests.
- This contract does not claim that external binaries, third-party dependencies, or operating system behavior are safe.
- This contract does not permit first-party production unsafe usage.

## Review Requirement
An independent reviewer must write `.beads/vb-y1zq/contract-verification-review.md` with `STATUS: APPROVED` before downstream test planning, proof work, implementation, or verification consumes these artifacts.
