# Lean Contract Projection: vb-nsnc

## Boundary

**Lean-owned kernel:**
- `is_capability_name_grammar_valid` — pure grammar validation for ASCII dotted capability names
- `validate_capability_name` — pure length + grammar validation with early-error ordering
- `validate_no_duplicate_capability_requirements` — deterministic O(n²) duplicate detection with first-error ordering

**Rust/runtime shell:**
- `validate_gate_12_action_contract_completeness` — workflow parts iteration, contract iteration, missing/orphan contract checks
- `validate_action_contract_capability_schema` — contract-level orchestration
- `validate_required_capability` — capability-level orchestration with action relation check

**External systems excluded from Lean proof:**
- Fjall persistence (cold path)
- IPC frame encoding/decoding
- Action ABI dispatch
- Generated workflow code

## Lean-Owned Clauses

### THM-GRAMMAR-VALID-001
- **Contract clause:** Schema grammar rule 1-7 (Section "Capability Name Schema" in contract.md)
- **Rust/spec target:** `vb_validate::gates::is_capability_name_grammar_valid`
- **Lean module:** `VBValidate.Capability`
- **Theorem shape:** `grammar_valid : ∀ (name : String), is_capability_name_grammar_valid name = true ↔ grammar_preconditions_hold name`
- **Model:** ASCII byte string with segment/dot structure
- **Refinement:** Rust `is_capability_name_grammar_valid` returns `true` exactly when Lean `grammar_preconditions_hold` is satisfied
- **Shell exclusions:** None (pure function)
- **Evidence command:** `lake build` on `VBValidate.Capability.lean`

### THM-LENGTH-BOUND-001
- **Contract clause:** Schema rule 1 (byte length 1..=128)
- **Rust/spec target:** `vb_validate::gates::validate_capability_name`
- **Lean module:** `VBValidate.Capability`
- **Theorem shape:** `length_bound : ∀ (name : String) (len > 128), validate_capability_name name = CapabilityNameTooLong`
- **Model:** String byte length
- **Refinement:** Length > 128 always returns `CapabilityNameTooLong` before grammar classification
- **Shell exclusions:** None (pure function)
- **Evidence command:** `lake build` on `VBValidate.Capability.lean`

### THM-FIRST-ERROR-001
- **Contract clause:** I9 (first error wins)
- **Rust/spec target:** `vb_validate::gates::validate_capability_name`
- **Lean module:** `VBValidate.Capability`
- **Theorem shape:** `first_error : ∀ (name : String), name.len = 0 → result = CapabilityNameEmpty ∧ name.len > 128 → result = CapabilityNameTooLong ∧ valid_grammar → result = Ok`
- **Model:** Ordering: empty → too_long → invalid_grammar → valid
- **Refinement:** Error ordering is deterministic and matches contract precedence
- **Shell exclusions:** None (pure function)
- **Evidence command:** `lake build` on `VBValidate.Capability.lean`

### THM-DUPLICATE-DETECTION-001
- **Contract clause:** I5 (no duplicate `(name, action)` pairs)
- **Rust/spec target:** `vb_validate::gates::validate_no_duplicate_capability_requirements`
- **Lean module:** `VBValidate.Capability`
- **Theorem shape:** `duplicate_detection : ∀ (caps : List Capability) (i j : Nat) (i < j) (caps[i] = caps[j]), result = CapabilityDuplicate first_index i duplicate_index j`
- **Model:** O(n²) search returning earliest duplicate pair
- **Refinement:** First duplicate by index ordering is reported, not arbitrary
- **Shell exclusions:** None (pure function)
- **Evidence command:** `lake build` on `VBValidate.Capability.lean`

### THM-DUPLICATE-SCOPE-001
- **Contract clause:** I5 duplicate scope (within one contract, not across contracts)
- **Rust/spec target:** `vb_validate::gates::validate_no_duplicate_capability_requirements`
- **Lean module:** `VBValidate.Capability`
- **Theorem shape:** `duplicate_scope : ∀ (caps : List Capability) (c1 c2 : Capability) (c1 ∈ caps) (c2 ∈ caps) (c1.name = c2.name) (c1.action ≠ c2.action), result ≠ CapabilityDuplicate`
- **Model:** Same name with different action is not a duplicate
- **Refinement:** Scope is exactly one `ActionContract.required_capabilities` list
- **Shell exclusions:** Only called within one contract's capability list
- **Evidence command:** `lake build` on `VBValidate.Capability.lean`

### THM-ACTION-RELATION-001
- **Contract clause:** I4 (capability action equals enclosing contract id)
- **Rust/spec target:** `vb_validate::gates::validate_required_capability`
- **Lean module:** `VBValidate.Capability`
- **Theorem shape:** `action_relation : ∀ (contract_action : ActionId) (cap : Capability) (cap.action ≠ contract_action), result = CapabilityActionMismatch`
- **Model:** ActionId equality check
- **Refinement:** Capability's action must match enclosing contract's id
- **Shell exclusions:** None (pure comparison)
- **Evidence command:** `lake build` on `VBValidate.Capability.lean`

## Waivers

**WAIVER-001: Gate 12 completeness/orphan check (not Lean-owned)**
- Owner: vb-nsnc contract
- Reason: `validate_gate_12_action_contract_completeness` iterates over `WorkflowParts` which involves Rust data structures not easily translatable to Lean. The pure capability schema functions (grammar, length, action relation, duplicates) are Lean-owned; the orchestration/shell remains tested by Kani + proptest + integration.
- Expiry: Never — shell behavior verified by other layers
- Compensating evidence: Kani harness on `validate_gate_12_action_contract_completeness` covers missing/orphan path; proptest covers schema validation with generated `WorkflowParts`; integration tests cover full pipeline

**WAIVER-002: Diagnostic code mapping (not Lean-owned)**
- Owner: vb-nsnc contract
- Reason: Diagnostic conversion (`diagnostic.rs`, `diag_convert.rs`, `diag_render.rs`) is string formatting with no pure logical content; tested by unit assertions on exact codes/messages
- Expiry: Never
- Compensating evidence: Unit tests verify exact `E050D..E0511` codes and messages; CLI integration tests verify exit code 1 and rendered output
