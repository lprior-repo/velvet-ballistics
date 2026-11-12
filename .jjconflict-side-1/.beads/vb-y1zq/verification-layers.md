# Verification Layers: vb-y1zq

## Boundary
- Verified kernel: Pure inventory predicates, boundary class to evidence rules, completion lattice, deterministic id generation, and typed error mapping.
- Lean contract projection: `BoundaryInventory.*` theorem obligations from `lean-contract.md`.
- Runtime shell: Repository scanning, parsing, filesystem/path validation, bead references, external command discovery, and report generation.
- External systems excluded from formal proof: OS, git, bd, Moon, cargo-fuzz runtime, shell execution, third-party binaries, and human review.

## Executable Layer Assignment
Every layer named here has a matching executable obligation in `proof-obligations.jsonl`.

- PRE-001 -> PRE-001-MANUAL, PRE-001-STATIC, PRE-001-GATE
- PRE-002 -> PRE-002-STATIC, PRE-002-MANUAL, PRE-002-PROP
- PRE-003 -> PRE-003-PROP, PRE-003-KANI
- PRE-004 -> PRE-004-STATIC, PRE-004-KANI
- PRE-005 -> PRE-005-STATIC, PRE-005-GATE
- POST-001 -> POST-001-PROP, POST-001-MUTATION, POST-001-COVERAGE
- POST-002 -> THM-POST-002, POST-002-PROP, POST-002-FUZZ
- POST-003 -> THM-POST-003, POST-003-MUTATION
- POST-004 -> POST-004-KANI, POST-004-PROP, POST-004-MUTATION
- POST-005 -> POST-005-STATIC, POST-005-MANUAL
- POST-006 -> POST-006-JSONL, POST-006-MANUAL
- INV-001 -> INV-001-STATIC, INV-001-GATE
- INV-002 -> THM-INV-002, INV-002-PROP, INV-002-KANI
- INV-003 -> THM-INV-003, INV-003-PROP, INV-003-FUZZ
- INV-004 -> THM-INV-004, INV-004-PROP, INV-004-KANI
- INV-005 -> THM-INV-005, INV-005-KANI, INV-005-MUTATION
- INV-006 -> INV-006-PROP, INV-006-MANUAL, INV-006-STATIC
- INV-007 -> INV-007-PROP, INV-007-MUTATION, INV-007-COMPAT
- Error::WorkspaceNotDiscoverable -> ERR-001
- Error::IncompleteDiscoveryInput -> ERR-002
- Error::UnknownBoundaryClass -> ERR-003
- Error::UnsafeForbiddenViolation -> ERR-004
- Error::MissingOwner -> ERR-005
- Error::MissingThreat -> ERR-006
- Error::MissingEvidencePath -> ERR-007
- Error::InvalidEvidencePath -> ERR-008
- Error::StaleEvidence -> ERR-009
- Error::DuplicateBoundaryId -> ERR-010
- Error::InventoryParseFailure -> ERR-011
- Error::SchemaVersionUnsupported -> ERR-012
- Error::ReviewStatusInvalid -> ERR-013
- Release provenance -> REL-001

## Moon Gauntlet Mapping
- `moon run :verify-fast`: PRE-001-GATE and fast static discovery checks.
- `moon run :verify-standard`: PRE-005-GATE, INV-001-GATE, unsafe ban, panic-path scan, inventory validation tests, coverage threshold evidence, and static error checks.
- `moon run :verify-deep`: fuzz/Bolero obligations, cargo-mutants obligations, manual QA artifact collation, and cargo-careful if waiver conditions are voided.
- `moon run :verify-proof`: Lean obligations and Kani model checks for the pure kernel.
- `moon run :verify-all`: GATE-002 release-critical evidence bundle before unsafe-isolation-complete status is accepted.

## Lean Scope
- Theorem modules: `BoundaryInventory.Completeness`, `BoundaryInventory.Evidence`, `BoundaryInventory.Identity`, `BoundaryInventory.Status`.
- Rust target: future `velvet_ballistics::quality::boundary_inventory` module or equivalent downstream implementation.
- Abstraction relation: Rust parsed inventory validates into pure Lean records; Lean predicates define allowed completion status; Rust shell proves it only accepts statuses admitted by the Lean model.
- Shell exclusions: I/O, path existence, parser mechanics, bd status, git state, external tool output, and human waiver approval.

## Required Evidence Artifacts
- `contract-verification-review.md` with independent `STATUS: APPROVED`.
- `formal-verification-report.md` containing Lean, Kani, static-scan, fuzz, mutation, coverage, API/schema compatibility, release provenance, and gauntlet results.
- Inventory evidence report containing each boundary id, class, owner, threat, evidence path, freshness marker, and review status.
- Manual QA transcript demonstrating every error variant scenario listed in `martin-fowler-tests.md`.

## Waivers
- Waiver W-Loom-001: Clause IDs: PRE-001, PRE-002, POST-006. Waived layer: Loom/Shuttle/Lockbud. Reason: State 1 contract defines no concurrent shared-state mutation, async runtime behavior, or interleaving-sensitive transition. Compensating evidence: PRE-001-MANUAL, PRE-002-MANUAL, POST-006-MANUAL, static scans, and gauntlet lanes. Owner: downstream implementer. Expiry/follow-up: Void before implementation review if concurrent inventory mutation, async scanning, background workers, or shared mutable cache are introduced; then add Loom/Shuttle/Lockbud obligations.
- Waiver W-Miri-001: Clause IDs: PRE-004, PRE-005, INV-001. Waived layer: Miri/cargo-careful. Reason: Contract requires safe first-party production Rust and no FFI execution in the inventory core. Compensating evidence: PRE-004-STATIC, PRE-004-KANI, PRE-005-STATIC, INV-001-STATIC. Owner: downstream implementer. Expiry/follow-up: Void before deep verification if unsafe, FFI, raw pointers, custom allocators, or layout-sensitive code enter implementation; then add Miri/cargo-careful obligations.
- Waiver W-Perf-001: Clause IDs: ALL. Waived layer: performance and assembly-ir. Reason: This bead makes no speed, vectorization, zero-cost, hot-path, or assembly claim. Compensating evidence: no performance acceptance criterion is used for completion. Owner: contract agent. Expiry/follow-up: Void if downstream adds performance or zero-cost claims; then add benchmark and assembly-ir obligations with baselines.
