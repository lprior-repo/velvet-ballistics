# Martin Fowler Test Plan: vb-nsnc

## Happy Path Tests

### test_accepts_action_contract_with_no_required_capabilities
Given a compiled workflow with one `Do` node for action `1` and one matching `ActionContract { id: 1, required_capabilities: [] }`
When `ValidationPipeline::validate_with_contracts(parts, contracts)` runs
Then validation succeeds with `Ok(())`

### test_accepts_dotted_capability_requirement_matching_contract_action
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "network.github", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation succeeds with `Ok(())`

### test_accepts_single_segment_capability_name
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "network", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation succeeds with `Ok(())`

### test_accepts_underscore_and_digit_in_capability_segment
Given matching `ActionContract { id: 1, required_capabilities: [Capability { name: "fs.read_tmp2", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation succeeds with `Ok(())`

### test_accepts_max_length_128_byte_capability_name
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: 128-byte-valid-lowercase-name, action: 1 }] }`
When gate 12 capability schema validation runs
Then validation succeeds with `Ok(())`

### test_accepts_same_capability_name_in_different_contracts
Given workflow uses actions `1` and `2`; contract `1` requires `network@1`; contract `2` requires `network@2`
When public validation runs
Then validation succeeds with `Ok(())`

---

## Error Path Tests

### test_rejects_empty_capability_name
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameEmpty { action_id: 1, capability_index: 0 })`

### test_rejects_capability_name_too_long_at_129_bytes
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: 129-byte-valid-shaped-name, action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameTooLong { action_id: 1, capability_index: 0, len: 129, max: 128 })`

### test_rejects_invalid_capability_grammar_with_colon
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "network:github", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "network:github".to_string() })`

### test_rejects_invalid_capability_grammar_with_leading_dot
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: ".network", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: ".network".to_string() })`

### test_rejects_invalid_capability_grammar_with_trailing_dot
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "network.", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "network.".to_string() })`

### test_rejects_invalid_capability_grammar_with_doubled_dot
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "network..github", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "network..github".to_string() })`

### test_rejects_invalid_capability_grammar_with_uppercase
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "Network", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "Network".to_string() })`

### test_rejects_invalid_capability_grammar_with_hyphen
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "network-github", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "network-github".to_string() })`

### test_rejects_invalid_capability_grammar_with_slash
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "secrets/read", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "secrets/read".to_string() })`

### test_rejects_invalid_capability_grammar_with_whitespace
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "network github", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "network github".to_string() })`

### test_rejects_invalid_capability_grammar_with_non_ascii
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "netwørk", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "netwørk".to_string() })`

### test_rejects_capability_action_mismatch
Given an `ActionContract { id: 1, required_capabilities: [Capability { name: "network", action: 2 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityActionMismatch { contract_action_id: 1, capability_action_id: 2, capability_index: 0 })`

### test_rejects_duplicate_capability_requirement_adjacent
Given an `ActionContract { id: 1, required_capabilities: [Capability { name: "network", action: 1 }, Capability { name: "network", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityDuplicate { action_id: 1, first_index: 0, duplicate_index: 1, name: "network".to_string() })`

### test_rejects_duplicate_capability_requirement_non_adjacent
Given an `ActionContract { id: 1, required_capabilities: [Capability { name: "network", action: 1 }, Capability { name: "fs.read", action: 1 }, Capability { name: "network", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityDuplicate { action_id: 1, first_index: 0, duplicate_index: 2, name: "network".to_string() })`

---

## Edge Case Tests

### test_first_schema_error_wins_before_duplicate_check
Given an `ActionContract { id: 1, required_capabilities: [Capability { name: "", action: 1 }, Capability { name: "network", action: 1 }, Capability { name: "network", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `Err(ValidationError::CapabilityNameEmpty { action_id: 1, capability_index: 0 })` (empty name error, not duplicate)

### test_first_schema_error_wins_before_orphan_check
Given workflow uses action `1`; contract `1` has invalid capability; orphan contract action `9` exists
When gate 12 runs
Then validation fails with the capability schema error before orphan check

### test_preserves_missing_contract_failure_precedence
Given workflow parts whose first node is a `Do` node at node index `0` for action `5`, and `action_contracts = []`
When `validate_with_contracts` runs
Then validation fails with `Err(ValidationError::ActionContractMissing { action_id: 5, node_index: 0 })`

### test_preserves_orphan_contract_failure_semantics
Given workflow parts with no `Do` node for action `9`, and one `ActionContract { id: 9, required_capabilities: [] }`
When `validate_with_contracts` runs
Then validation fails with `Err(ValidationError::ActionContractOrphan { action_id: 9 })`

### test_shared_validation_path_invokes_live_gate_implementation
Given the empty-name invalid fixture
When validation runs only through `vb_validate::shared::ValidationPipeline::validate_with_contracts`
Then result is exactly `Err(ValidationError::CapabilityNameEmpty { action_id: 1, capability_index: 0 })`

---

## Contract Verification Tests

### test_precondition_workflow_parts_trusted
Given a fully constructed `WorkflowParts` reference
When `validate_gate_12_action_contract_completeness(parts, contracts)` is called
Then it processes all `Do` nodes and `ActionContract` entries without panicking

### test_invariant_schema_validity_per_capability
Given a capability passes `validate_capability_name`
When its name is inspected
Then it satisfies: byte length in 1..=128, ASCII only, one or more dot-separated segments, each segment non-empty starting with lowercase a-z, tail may contain a-z0-9_

### test_invariant_action_equality_per_capability
Given a capability in `ActionContract.required_capabilities`
When validated
Then its `action` field equals the enclosing `ActionContract.id`

### test_invariant_no_duplicate_within_contract
Given an `ActionContract` with unique `(name, action)` pairs
When `validate_no_duplicate_capability_requirements` runs
Then result is `Ok(())`

### test_postcondition_empty_capabilities_accepted
Given valid `ActionContract` with empty `required_capabilities`
When validated through `validate_with_contracts`
Then result is `Ok(())`

### test_postcondition_valid_schema_returns_ok
Given `ActionContract` with valid capability names
When validated through `validate_with_contracts`
Then result is `Ok(())`

---

## Diagnostic Coverage Tests

### test_diagnostic_code_e050d_for_capability_name_empty
Given `ValidationError::CapabilityNameEmpty { action_id: 1, capability_index: 0 }`
When `diagnostic_from_error` runs
Then result has code `E050D` and message containing "capability name is empty for action 1 at required_capabilities[0]"

### test_diagnostic_code_e050e_for_capability_name_too_long
Given `ValidationError::CapabilityNameTooLong { action_id: 1, capability_index: 0, len: 129, max: 128 }`
When `diagnostic_from_error` runs
Then result has code `E050E` and message containing "capability name too long", "action 1", "required_capabilities[0]", "129 > 128"

### test_diagnostic_code_e050f_for_capability_name_invalid
Given `ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "network:github" }`
When `diagnostic_from_error` runs
Then result has code `E050F` and message containing "invalid capability name", "action 1", "required_capabilities[0]", "network:github"

### test_diagnostic_code_e0510_for_capability_action_mismatch
Given `ValidationError::CapabilityActionMismatch { contract_action_id: 1, capability_action_id: 2, capability_index: 0 }`
When `diagnostic_from_error` runs
Then result has code `E0510` and message containing "capability action 2", "contract action 1", "required_capabilities[0]"

### test_diagnostic_code_e0511_for_capability_duplicate
Given `ValidationError::CapabilityDuplicate { action_id: 1, first_index: 0, duplicate_index: 1, name: "network" }`
When `diagnostic_from_error` runs
Then result has code `E0511` and message containing "duplicate capability requirement", "action 1", "network", "required_capabilities[0]", "required_capabilities[1]"

---

## Given-When-Then Scenarios (Comprehensive)

### Scenario 1: Valid empty capability list passes
Given a compiled workflow with one `Do` node for action `1` and one matching `ActionContract { id: 1, required_capabilities: [] }`
When `validate_with_contracts(parts, contracts)` runs
Then validation succeeds

### Scenario 2: Valid dotted hierarchical name passes
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "network.github", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation succeeds

### Scenario 3: Empty capability name is rejected
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `ValidationError::CapabilityNameEmpty { action_id: 1, capability_index: 0 }`

### Scenario 4: Invalid capability grammar with colon is rejected
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: "network:github", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "network:github" }`

### Scenario 5: Too-long capability name is rejected
Given a matching `ActionContract { id: 1, required_capabilities: [Capability { name: 129-byte-name, action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `ValidationError::CapabilityNameTooLong { len: 129, max: 128, .. }`

### Scenario 6: Capability action mismatch is rejected
Given an `ActionContract { id: 1, required_capabilities: [Capability { name: "network", action: 2 }] }`
When gate 12 capability schema validation runs
Then validation fails with `ValidationError::CapabilityActionMismatch { contract_action_id: 1, capability_action_id: 2, capability_index: 0 }`

### Scenario 7: Duplicate capability requirement in one contract is rejected
Given an `ActionContract { id: 1, required_capabilities: [Capability { name: "network", action: 1 }, Capability { name: "network", action: 1 }] }`
When gate 12 capability schema validation runs
Then validation fails with `ValidationError::CapabilityDuplicate { action_id: 1, first_index: 0, duplicate_index: 1, name: "network" }`

### Scenario 8: Missing contract failure takes precedence
Given workflow parts with a `Do` node for action `5` and `action_contracts = []`
When `validate_with_contracts` runs
Then validation fails with `ValidationError::ActionContractMissing { action_id: 5, node_index: 0 }` before capability schema checks

### Scenario 9: Orphan contract failure respects precedence
Given workflow parts with no `Do` node for action `9` and a supplied `ActionContract { id: 9, required_capabilities: [] }`
When `validate_with_contracts` runs
Then validation fails with `ValidationError::ActionContractOrphan { action_id: 9 }` unless an earlier schema violation exists

### Scenario 10: All five new error variants have diagnostic codes
Given each new `ValidationError` variant in the capability schema
When `diagnostic_from_error` and CLI validation rendering run
Then each returns stable E050D..E0511 codes and human-readable messages
