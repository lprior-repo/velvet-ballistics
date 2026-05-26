# Lean Contract Projection: vb-y1zq

## Boundary
- Lean-owned kernel: Pure predicates over inventory records, boundary classes, evidence requirements, completion status, id uniqueness, and fail-closed status transitions.
- Rust/runtime shell: Filesystem discovery, source scanning, path resolution, bead lookup, git status, command execution, parser I/O, and report writing.
- External systems excluded from Lean proof: OS filesystem, bd database, git, Moon, cargo tools, shell scripts, external binaries, third-party crate internals, fuzz engines, and human review behavior.

## Lean-Owned Clauses
- INV-002 -> `BoundaryInventory.Completeness::complete_requires_required_fields`
- INV-003 -> `BoundaryInventory.Evidence::crossing_boundary_requires_evidence`
- INV-004 -> `BoundaryInventory.Identity::stable_ids_are_unique_when_sources_unique`
- INV-005 -> `BoundaryInventory.Status::invalid_inventory_cannot_complete`
- POST-002 -> `BoundaryInventory.Evidence::required_evidence_assigned_for_risky_classes`
- POST-003 -> `BoundaryInventory.Status::unknown_class_blocks_completion`

## Theorem Obligations

### THM-INV-002
- Contract clause: INV-002
- Rust/spec target: `velvet_ballistics::quality::boundary_inventory::validate_inventory`
- Lean module: `BoundaryInventory.Completeness`
- Theorem shape: `complete_requires_required_fields`
- Model: Abstract `BoundaryRecord` with optional owner, threat, evidence path, class, source path, and review status fields.
- Refinement: Rust validated inventory reifies into Lean records; Lean `Complete` corresponds to Rust `UnsafeIsolationStatus::Complete`.
- Shell exclusions: filesystem, parsing, path existence checks, bead lookup, human review.
- Evidence command: `moon run :verify-proof`

### THM-INV-003
- Contract clause: INV-003
- Rust/spec target: `velvet_ballistics::quality::boundary_inventory::required_evidence`
- Lean module: `BoundaryInventory.Evidence`
- Theorem shape: `crossing_boundary_requires_evidence`
- Model: Abstract boundary class plus booleans `ingests_external_bytes` and `crosses_process_or_language_limit`.
- Refinement: Rust classification maps each boundary to the Lean risk predicate before completion is evaluated.
- Shell exclusions: actual fuzz execution, manual QA execution, artifact freshness checks.
- Evidence command: `moon run :verify-proof`

### THM-INV-004
- Contract clause: INV-004
- Rust/spec target: `velvet_ballistics::quality::boundary_inventory::boundary_id`
- Lean module: `BoundaryInventory.Identity`
- Theorem shape: `stable_ids_are_unique_when_sources_unique`
- Model: Normalized class and normalized source identity produce an abstract boundary id.
- Refinement: Rust id normalization must match the Lean normalization model for class plus source identity.
- Shell exclusions: OS-specific path canonicalization and symlink resolution.
- Evidence command: `moon run :verify-proof`

### THM-INV-005
- Contract clause: INV-005
- Rust/spec target: `velvet_ballistics::quality::boundary_inventory::inventory_completion_status`
- Lean module: `BoundaryInventory.Status`
- Theorem shape: `invalid_inventory_cannot_complete`
- Model: Algebraic lattice with `Complete`, `Incomplete`, and `Blocked` statuses.
- Refinement: Rust `Result<UnsafeIsolationStatus, BoundaryInventoryError>` maps errors and blockers to non-complete Lean states.
- Shell exclusions: parser implementation, filesystem availability, command exit codes.
- Evidence command: `moon run :verify-proof`

### THM-POST-002
- Contract clause: POST-002
- Rust/spec target: `velvet_ballistics::quality::boundary_inventory::required_evidence`
- Lean module: `BoundaryInventory.Evidence`
- Theorem shape: `required_evidence_assigned_for_risky_classes`
- Model: Abstract risk class to evidence requirement relation.
- Refinement: Rust `EvidenceRequirement` must admit completion only for evidence kinds allowed by the Lean relation.
- Shell exclusions: external fuzz execution, manual QA execution, artifact freshness.
- Evidence command: `moon run :verify-proof`

### THM-POST-003
- Contract clause: POST-003
- Rust/spec target: `velvet_ballistics::quality::boundary_inventory::inventory_completion_status`
- Lean module: `BoundaryInventory.Status`
- Theorem shape: `unknown_class_blocks_completion`
- Model: Completion lattice over classified boundary records.
- Refinement: Rust `unknown` classification maps to Lean `Blocked` unless an approved waiver is modeled as explicit non-complete follow-up state.
- Shell exclusions: bead creation side effects, human waiver review, filesystem.
- Evidence command: `moon run :verify-proof`

## Lean Waivers
- Waiver W-Lean-PRE-001: Clause ID: PRE-001. Waived layer: Lean. Reason: workspace discovery is filesystem I/O, not a pure deterministic kernel. Compensating evidence: PRE-001-MANUAL, PRE-001-STATIC, PRE-001-GATE. Owner: contract agent. Expiry/follow-up: Re-evaluate before implementation review if discovery is reduced to a pure manifest-selection kernel.
- Waiver W-Lean-PRE-002: Clause ID: PRE-002. Waived layer: Lean. Reason: discovery completeness depends on repository contents and static scans. Compensating evidence: PRE-002-STATIC, PRE-002-MANUAL, PRE-002-PROP. Owner: contract agent. Expiry/follow-up: Re-evaluate before implementation review if a pure closed-world discovery manifest is introduced.
- Waiver W-Lean-PRE-004: Clause ID: PRE-004. Waived layer: Lean. Reason: panic-freedom and `Result` usage are Rust implementation properties. Compensating evidence: PRE-004-STATIC and PRE-004-KANI. Owner: contract agent. Expiry/follow-up: Re-evaluate if a pure error-return lattice is added beyond THM-INV-005.
- Waiver W-Lean-ERR-SHELL: Clause IDs: Error::WorkspaceNotDiscoverable, Error::IncompleteDiscoveryInput, Error::UnsafeForbiddenViolation, Error::InvalidEvidencePath, Error::StaleEvidence, Error::InventoryParseFailure. Waived layer: Lean. Reason: these variants depend on filesystem, scanner, parser, freshness, or static-analysis shell behavior. Compensating evidence: ERR-001, ERR-002, ERR-004, ERR-008, ERR-009, ERR-011 plus Fowler scenarios. Owner: contract agent. Expiry/follow-up: Re-evaluate before proof work if pure constructors are separated from shell effects.
