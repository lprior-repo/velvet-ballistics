# Contract Specification: vb-5xs4

## Context
- Feature: inventory weak Rust test loop and table-loop patterns before mutation refresh.
- Goal: every risky loop/table-loop occurrence in Rust tests is assigned to exactly one outcome: repair bead, accepted exception, or proof that case labeling is safe.
- Source surfaces: `tests/**` and `crates/**` Rust test code in the workspace.
- Domain terms:
  - Test loop pattern: repeated assertions inside `for`, iterator, macro-expanded, or helper-driven table execution.
  - Weak loop: a loop whose failure output cannot identify the failing behavior or case.
  - Case identity: stable, human-readable label tying a failing iteration to behavior and fixture input.
  - Repair assignment: a bead or work item that will split, label, or strengthen the test.
  - Accepted exception: documented reason a loop is safe enough to keep.
  - Safe labeling proof: evidence that all loop failures expose case identity and behavior context.
- Assumptions:
  - This state only creates contract and verification planning artifacts; it does not scan, implement, or test.
  - Inventory output may be produced later as a report, structured data, or bead updates, but the runtime core must not depend on YAML, JSON, or HTTP.
  - Existing repository rules remain binding: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg`; no unchecked indexing, slicing, casts, or arithmetic.
- Open questions:
  - What exact inventory report path and schema will the implementation use?
  - What bead command/API is authoritative for creating repair beads in later states?
  - Are macro-generated test loops in dependency-generated code in scope, or only first-party repository code?

## Preconditions
- PRE-001: The workspace root exists and is readable.
- PRE-002: The inventory input roots are explicitly bounded to first-party Rust test surfaces: `tests/**` and `crates/**`.
- PRE-003: Candidate Rust files are discoverable without following external, vendored, or generated paths unless explicitly whitelisted.
- PRE-004: The scanner/classifier receives valid text input for every candidate file or returns a typed read/encoding error.
- PRE-005: Assignment sinks for repair, exception, and safe-labeling proof are available before the quality gate is considered passable.
- PRE-006: Case-label sufficiency rules are configured before classification begins.

## Postconditions
- POST-001: Every discovered risky loop pattern is present in the inventory with file path, stable location, pattern kind, risk reason, and owner/action.
- POST-002: Every risky loop pattern is assigned exactly one disposition: `RepairRequired`, `AcceptedException`, or `SafeLabelingProven`.
- POST-003: Any unlabeled or ambiguously labeled loop that can hide the failing case is classified as `RepairRequired`.
- POST-004: Any accepted exception records a reason, scope, owner, and expiry/review trigger.
- POST-005: Any safe-labeling proof records the specific evidence that failure output identifies behavior and case.
- POST-006: The inventory operation is deterministic for the same source tree and configuration.
- POST-007: The quality gate fails closed when any risky loop lacks a disposition.
- POST-008: Non-risky loops may be recorded, but they must not mask or suppress risky findings.

## Invariants
- INV-001: No risky loop pattern may remain unassigned.
- INV-002: Each risky finding has exactly one disposition; duplicate or conflicting dispositions are invalid.
- INV-003: Failure diagnostics for retained loops must identify the broken behavior and failing case.
- INV-004: Classification is monotonic with respect to evidence: adding stronger labeling evidence may downgrade risk, but absence or ambiguity cannot downgrade risk.
- INV-005: The classifier never treats deletion or disappearance of a test as a repair.
- INV-006: Generated inventory never claims mutation-quality improvement without mutation evidence; this bead inventories and assigns only.
- INV-007: Runtime core inventory/classification logic remains free of YAML, JSON, and HTTP dependencies.
- INV-008: All fallible operations use typed `Result<T, InventoryError>` railway-oriented error handling.

## Error Taxonomy
- ERR-001 `InventoryError::WorkspaceUnreadable` - workspace root or bounded input root cannot be read.
- ERR-002 `InventoryError::InputRootOutOfScope` - caller requests paths outside `tests/**` or `crates/**`.
- ERR-003 `InventoryError::FileReadFailed` - candidate file cannot be read.
- ERR-004 `InventoryError::InvalidUtf8` - candidate file is not valid text for the scanner.
- ERR-005 `InventoryError::ParseFailed` - Rust syntax cannot be parsed well enough to classify loop patterns.
- ERR-006 `InventoryError::AmbiguousCaseLabel` - a loop has labels, but they do not uniquely identify failing behavior and case.
- ERR-007 `InventoryError::UnassignedRiskyPattern` - a risky finding has no repair, exception, or proof disposition.
- ERR-008 `InventoryError::ConflictingDisposition` - a finding has more than one disposition.
- ERR-009 `InventoryError::DestructiveChangeDetected` - a test disappeared and is being presented as quality improvement.
- ERR-010 `InventoryError::UnsupportedGeneratedSource` - generated or macro-expanded source cannot be traced to a stable first-party test location.
- ERR-011 `InventoryError::PolicyViolation` - implementation violates repository engineering rules.

## Contract Signatures
- `fn discover_rust_test_files(root: WorkspaceRoot, scope: InventoryScope) -> Result<Vec<TestFile>, InventoryError>`
- `fn scan_test_file(file: TestFile, text: SourceText) -> Result<Vec<LoopPattern>, InventoryError>`
- `fn classify_loop_pattern(pattern: LoopPattern, policy: LabelingPolicy) -> Result<LoopRisk, InventoryError>`
- `fn assign_disposition(risk: LoopRisk, evidence: AssignmentEvidence) -> Result<Disposition, InventoryError>`
- `fn validate_inventory(findings: Inventory) -> Result<ValidatedInventory, InventoryError>`
- `fn render_inventory_report(inventory: ValidatedInventory) -> Result<InventoryReport, InventoryError>`

## Lean-Owned Clauses
- THM-INV-001 covers INV-001 and POST-007 as a pure disposition-completeness lattice.
- THM-INV-002 covers INV-002 and ERR-008 as a pure exactly-one-disposition predicate.
- THM-INV-003 covers INV-003 as a pure retained-loop diagnostic sufficiency predicate.
- THM-POST-005 covers POST-005 as a pure safe-labeling proof completeness predicate.
- THM-ERR-006 covers ERR-006 as a pure ambiguous-label rejection predicate.
- THM-POST-006 covers POST-006 as determinism of pure classification over ordered inputs.
- THM-INV-004 covers INV-004 as monotonic evidence refinement.

## Rust-Realization Evidence Required For Pure Critical Clauses
- POST-003, POST-005, INV-003, and ERR-006 require Rust-realization evidence beyond Lean: Kani constructor/transition checks, proptest generation over labels and loop patterns, fuzz/Bolero coverage for hostile source-shaped labels, mutation checks that weaken label sufficiency, and `moon run :verify-proof` gauntlet evidence.
- Every error variant ERR-001 through ERR-011 requires an individual proof obligation and an individual traceability row.
- Every waiver requires clause ID(s), waived layer, reason, compensating evidence, owner, and expiration/follow-up condition.

## Non-goals
- No production scanner, tests, Lean proofs, Kani harnesses, fuzz targets, or bead mutations are implemented in State 1.
- No performance, vectorization, public API compatibility, or release-provenance claim is made by this contract.
- No direct Lean proof is attempted for filesystem traversal, bead database updates, process execution, async runtime behavior, or external services.

## Review Gate
- Downstream test planning, test writing, implementation, and proof work must not consume these artifacts until an independent reviewer writes `contract-verification-review.md` with `STATUS: APPROVED`.
