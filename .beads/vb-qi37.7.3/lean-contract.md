# Lean Contract Projection: vb-qi37.7.3

## Boundary
- Lean-owned kernel: pure reference-validity predicates over abstract `WorkflowPartsModel`, `SymbolId`, slot IDs, constant IDs, handler IDs, `ActionId`, `ActionContractId`, and `ResourceContractModel`.
- Rust/runtime shell: concrete Rust iteration, enum decoding, diagnostic construction, `Result` plumbing, ownership/borrowing, Moon/Cargo execution, and all I/O-free implementation details.
- External systems excluded from Lean proof: file system, network, runtime registries, YAML/JSON parsing, HTTP, async scheduling, storage adapters, and diagnostic text rendering.

## Abstract Model
- `WorkflowPartsModel` contains finite lists/counts for symbols, slots, constants, handlers, actions used by `Do` nodes, and declared resource limits.
- `ReferenceLocation` tags accessor field, constant symbol, build-object field, slot use, constant use, handler use, and action use.
- `ValidationResult` is abstract success/failure, not concrete Rust error formatting.
- Refinement relation: Rust `WorkflowParts` validates into `WorkflowPartsModel` by preserving counts, referenced IDs, declared resources, and action-contract IDs. Rust success must imply Lean predicate success; Lean predicate failure must be observable as a precise Rust typed error.

## Lean-Owned Clauses
- INV-001 -> `Velvet.IR.ReferenceValidation::all_symbol_refs_bounded_iff_valid`
- INV-002 -> `Velvet.IR.ReferenceValidation::zero_symbols_rejects_symbol_ref`
- INV-003 -> `Velvet.IR.ReferenceValidation::action_contract_bijection_exact`
- INV-004 -> `Velvet.IR.ReferenceValidation::all_slot_refs_owned_and_bounded`
- INV-005 -> `Velvet.IR.ReferenceValidation::all_constant_refs_owned_kind_correct_and_bounded`
- INV-006 -> `Velvet.IR.ReferenceValidation::all_handler_refs_owned_kind_correct_and_bounded`
- INV-007 -> `Velvet.IR.ResourceValidation::declared_resources_within_hard_limits`
- INV-008 -> `Velvet.IR.ResourceValidation::actual_usage_within_declared_resources`
- POST-007 -> `Velvet.IR.ReferenceValidation::valid_references_are_artifact_or_contract_owned`

## Theorem Obligations

### THM-INV-001
- Contract clause: INV-001
- Rust/spec target: `vb_core::workflow::validate_symbol_references` and verifier parity gate
- Lean module: `Velvet.IR.ReferenceValidation`
- Theorem shape: `all_symbol_refs_bounded_iff_valid`
- Model: finite symbol reference list and `symbols_count : Nat`
- Refinement: Rust extracts every symbol-bearing location into the Lean symbol reference list without omission or duplication relevant to validity.
- Shell exclusions: enum traversal mechanics, diagnostic text, allocation strategy, Cargo/Moon execution.
- Evidence command: `moon run :verify-proof`

### THM-INV-002
- Contract clause: INV-002
- Rust/spec target: `vb_core::workflow::validate_symbol_references`
- Lean module: `Velvet.IR.ReferenceValidation`
- Theorem shape: `zero_symbols_rejects_symbol_ref`
- Model: `symbols_count = 0` and non-empty symbol reference list
- Refinement: `SymbolId::new(0)` and larger IDs map to natural symbol IDs, and no Rust symbol carrier bypasses extraction.
- Shell exclusions: concrete constructor validation and diagnostic formatting.
- Evidence command: `moon run :verify-proof`

### THM-INV-003
- Contract clause: INV-003
- Rust/spec target: `vb_validate::gates::validate_gate_12_action_contract_completeness`
- Lean module: `Velvet.IR.ReferenceValidation`
- Theorem shape: `action_contract_bijection_exact`
- Model: finite multiset/list of used action IDs and supplied contract IDs; validity is set equality.
- Refinement: Rust duplicate `Do.action` references collapse to unique action IDs; supplied duplicate contracts do not create false success outside set equality and must be handled by Rust diagnostics if semantically invalid.
- Shell exclusions: action registry storage, runtime dispatch, diagnostic code rendering.
- Evidence command: `moon run :verify-proof`

### THM-INV-004
- Contract clause: INV-004
- Rust/spec target: slot validation gates in `vb_core::workflow` and `vb_validate::gates`
- Lean module: `Velvet.IR.ReferenceValidation`
- Theorem shape: `all_slot_refs_owned_and_bounded`
- Model: finite slot reference list and `slot_count : Nat`
- Refinement: Rust extracts all slot-use sites from nodes/expressions; success implies every extracted slot index is below `slot_count`.
- Shell exclusions: Rust enum traversal and concrete error payload choice.
- Evidence command: `moon run :verify-proof`

### THM-INV-005
- Contract clause: INV-005
- Rust/spec target: constant reference validation and `ConstValue::Symbol` validation
- Lean module: `Velvet.IR.ReferenceValidation`
- Theorem shape: `all_constant_refs_owned_kind_correct_and_bounded`
- Model: constant table, constant reference list, and required constant kinds by use site
- Refinement: Rust constant indexes and symbol-valued constants map to Lean constants with kind tags.
- Shell exclusions: diagnostic rendering and memory layout.
- Evidence command: `moon run :verify-proof`

### THM-INV-006
- Contract clause: INV-006
- Rust/spec target: handler reference validation gates
- Lean module: `Velvet.IR.ReferenceValidation`
- Theorem shape: `all_handler_refs_owned_kind_correct_and_bounded`
- Model: handler table and handler reference list tagged by use site
- Refinement: Rust handler IDs map only to handlers in the same `WorkflowParts` model.
- Shell exclusions: runtime handler dispatch and external plugin systems.
- Evidence command: `moon run :verify-proof`

### THM-INV-007
- Contract clause: INV-007
- Rust/spec target: `vb_core::workflow::validate_resource_contract`
- Lean module: `Velvet.IR.ResourceValidation`
- Theorem shape: `declared_resources_within_hard_limits`
- Model: resource contract natural-number fields and hard-limit constants
- Refinement: Rust integer fields are embedded as natural numbers after checked conversion; overflow behavior is excluded and covered by Kani/static scans.
- Shell exclusions: concrete numeric representation and diagnostics.
- Evidence command: `moon run :verify-proof`

### THM-INV-008
- Contract clause: INV-008
- Rust/spec target: `vb_core::workflow::validate_budget` and verifier resource gates
- Lean module: `Velvet.IR.ResourceValidation`
- Theorem shape: `actual_usage_within_declared_resources`
- Model: actual usage counts and declared resource limits
- Refinement: Rust counts for nodes, slots, constants, accessors, expressions, handlers, and max expression stack map exactly to Lean usage fields.
- Shell exclusions: concrete iteration, checked arithmetic implementation, and diagnostics.
- Evidence command: `moon run :verify-proof`

### THM-POST-007
- Contract clause: POST-007
- Rust/spec target: `vb_core::workflow` reference gates and `vb_validate::shared::validate_with_contracts`
- Lean module: `Velvet.IR.ReferenceValidation`
- Theorem shape: `valid_references_are_artifact_or_contract_owned`
- Model: `WorkflowPartsModel` containing artifact-owned symbol, slot, constant, handler, and resource domains plus the supplied action-contract ID set for the current validation call.
- Refinement: Rust extracts every checked reference from the admitted `WorkflowParts` into the Lean model; Rust action references refine to the supplied `ActionContract.id` set; Rust success implies each reference belongs either to the same artifact model or to that exact supplied contract set.
- Shell exclusions: concrete Rust traversal, diagnostic construction, external registries, runtime action dispatch, storage, filesystem, network, and Moon/Cargo execution.
- Evidence command: `moon run :verify-proof`

## Waivers
- WAIVE-PRE-002: Clause: PRE-002. Waived layer: Lean. Reason: borrowing/no-mutation is Rust ownership-shell behavior, not a pure reference predicate. Compensating evidence: Miri/cargo-careful, unit tests, static scan for mutation/global state. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-PRE-003: Clause: PRE-003. Waived layer: Lean. Reason: `Result` API shape and ownership are Rust API-shell behavior. Compensating evidence: compile checks, unit tests, static scan for panic paths. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-PRE-004: Clause: PRE-004. Waived layer: Lean. Reason: caller boundary between `validate` and `validate_with_contracts` is API-shell behavior. Compensating evidence: integration tests and traceability entries. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-PRE-006: Clause: PRE-006. Waived layer: Lean. Reason: prohibition on runtime I/O is environmental/static behavior. Compensating evidence: static scans for JSON/YAML/HTTP/filesystem/network use. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-POST-004: Clause: POST-004. Waived layer: Lean. Reason: default validator's non-action-complete API semantics are Rust-shell/API behavior. Compensating evidence: integration tests proving `validate` skips Gate 12 and mutation tests killing false action-complete claims. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-POST-009: Clause: POST-009. Waived layer: Lean. Reason: concrete typed error construction and no-partial-artifact `Result` plumbing are Rust-shell observability and ownership behavior. Compensating evidence: unit/integration tests for precise errors, mutation tests for error branches, static scan for panic/partial acceptance paths. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-ERR-001-009: Clause: ERR-001..ERR-009. Waived layer: Lean. Reason: concrete typed errors and diagnostic codes are Rust-shell observability, not theorem content. Compensating evidence: unit/integration tests, mutation, diagnostic rendering tests. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-INV-009-010: Clause: INV-009..INV-010. Waived layer: Lean. Reason: determinism, boundedness, and banned constructs are implementation governance properties. Compensating evidence: Kani, static scans, clippy/governance scripts, Miri where applicable. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-AC-008: Clause: AC-008. Waived layer: Lean. Reason: runtime purity, deterministic bounded scans, and absence of YAML/JSON/HTTP are shell/static implementation properties. Compensating evidence: static scans, Miri, and `moon run :verify-standard`. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-AC-009: Clause: AC-009. Waived layer: Lean. Reason: diagnostic enum variants, stable codes, and assertion coverage are Rust observability contracts. Compensating evidence: unit/integration diagnostic tests, mutation, cargo-llvm-cov branch evidence. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
- WAIVE-AC-010: Clause: AC-010. Waived layer: Lean. Reason: CI and verification gauntlet execution is release/process evidence, not a pure theorem. Compensating evidence: `moon run :verify-fast`, `moon run :verify-standard`, `moon run :verify-proof`, and `moon run :verify-all` logs. Owner: Rust Contract Agent. Expiration/follow-up: before implementation review.
