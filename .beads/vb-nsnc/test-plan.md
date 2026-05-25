# Test Plan: vb-nsnc — verifier/runtime capability contract schema

## Review Repair Statement

This repaired plan explicitly addresses every finding in `.beads/vb-nsnc/test-plan-review.md`: exact missing/orphan `ValidationError` payloads replace placeholders; CLI oracles name exit code `1` and stable diagnostic codes `E050D..E0511`; all five new variants have CLI/user-facing rendering coverage; mutation waivers are removed for critical mutants; static/resource/panic checks are explicit; duplicate bounds and deterministic schema-vs-orphan precedence are pinned.

## Summary

- Behaviors identified: 24.
- Trophy allocation: 8 unit / 13 integration / 3 e2e / static gates.
- Required test density: at least 7 focused unit/property checks per public validator entry point touched; at least 5 integration checks through `ValidationPipeline::validate_with_contracts`; every new error variant must have unit diagnostic, integration validation, and CLI/shared-render coverage.
- Proptest invariants: 7.
- Fuzz targets: 2.
- Kani harnesses: 4, with mandatory fallback proof if Kani is unavailable.
- Mutation threshold: `cargo-mutants` kill rate must be `>= 90%` for touched validation, diagnostic, and CLI formatting files; every survivor in the critical list below forces a named test, except mutants proven unreachable by Rust type/state construction and cited in the test report.
- Canonical final gate: `moon ci`.
- Assertion rule: no planned test may assert only `is_ok()` or `is_err()`; assertions must name `Ok(())`, exact typed error values, exact diagnostic codes/messages, exact CLI exit code `1`, or exact output substrings.

## 1. Behavior Inventory

1. `validate_with_contracts` accepts action contracts when every `Do` node has a matching contract and every required capability is schema-valid.
2. `validate_with_contracts` accepts empty `required_capabilities` when the enclosing action contract is used.
3. Capability schema accepts one-segment names when the segment follows lowercase ASCII grammar.
4. Capability schema accepts dotted hierarchical names when every segment follows lowercase ASCII grammar.
5. Capability schema accepts tail underscores and digits when the segment starts with lowercase ASCII.
6. Capability schema rejects empty names with `ValidationError::CapabilityNameEmpty { action_id, capability_index }` when byte length is zero.
7. Capability schema rejects names longer than 128 bytes with `ValidationError::CapabilityNameTooLong { action_id, capability_index, len, max: 128 }`.
8. Capability schema rejects non-empty invalid grammar with `ValidationError::CapabilityNameInvalid { action_id, capability_index, name }`.
9. Capability schema rejects action mismatch with `ValidationError::CapabilityActionMismatch { contract_action_id, capability_action_id, capability_index }`.
10. Capability schema rejects duplicate `(name, action)` pairs within one contract with `ValidationError::CapabilityDuplicate { action_id, first_index, duplicate_index, name }`.
11. Capability schema allows the same name in different contracts when each action id matches its own contract.
12. Capability schema reports the earliest duplicate pair in one contract when multiple duplicate pairs exist.
13. Gate 12 returns the first per-requirement schema error before duplicate detection and before orphan detection when an otherwise used contract contains invalid capabilities.
14. Gate 12 preserves missing-contract regression with `ValidationError::ActionContractMissing { action_id: 5, node_index: 0 }` when first `Do` node action 5 lacks a contract.
15. Gate 12 preserves orphan-contract regression with `ValidationError::ActionContractOrphan { action_id: 9 }` when no `Do` node references contract action 9 and no earlier missing/schema error exists.
16. `ValidationPipeline::validate_with_contracts` reaches the live `gates.rs` path, not only `gate_12_14_15.rs`.
17. Diagnostic conversion maps `CapabilityNameEmpty` to `E050D` / `0x050D` and the mandated message.
18. Diagnostic conversion maps `CapabilityNameTooLong` to `E050E` / `0x050E` and includes `len` and `max`.
19. Diagnostic conversion maps `CapabilityNameInvalid` to `E050F` / `0x050F` and includes the rejected name.
20. Diagnostic conversion maps `CapabilityActionMismatch` to `E0510` / `0x0510` and includes both action ids.
21. Diagnostic conversion maps `CapabilityDuplicate` to `E0511` / `0x0511` and includes both indexes and duplicate name.
22. CLI/user-facing validation rendering reports `CapabilityNameInvalid` with exit code `1`, `E050F`, and exact error text.
23. CLI/user-facing rendering covers all five new variants either through five CLI fixtures or one CLI fixture plus unit proof that CLI calls the same shared renderer for all variants.
24. Touched production validation/diagnostic/CLI code contains no forbidden constructs, unchecked indexing/slicing/casts/arithmetic, recursion, unbounded loops, runtime JSON/YAML/HTTP additions, or pathological formatting of unbounded rejected names.

## 2. Trophy Allocation

| # | Behavior | Layer | Tool/location | Rationale |
|---|----------|-------|---------------|-----------|
| 1-2 | Public acceptance of valid/empty requirements | Integration | `ValidationPipeline::validate_with_contracts` | Public contract, real `WorkflowParts` + `ActionContract`. |
| 3-5 | Valid name grammar | Unit + proptest | `vb_validate::gates` tests | Pure grammar, many combinations. |
| 6-8 | Name error taxonomy | Unit + integration | grammar units plus pipeline exact errors | Unit isolates grammar; integration proves public path. |
| 9 | Action mismatch | Integration | pipeline exact error | Crosses contract and capability relation. |
| 10-12 | Duplicate semantics | Integration + proptest | pipeline and generated vectors | Slice-level state, deterministic indexes. |
| 13 | First-error precedence | Integration + Kani | multi-error fixtures | Prevents nondeterministic diagnostics. |
| 14-15 | Missing/orphan regressions | Integration | existing public gate fixtures, exact payloads | Existing API semantics must not drift. |
| 16 | Live gate wiring | Integration | call shared public API only | Catches edits only in test-only parallel gate. |
| 17-21 | Diagnostic mapping | Unit | `diagnostic.rs`, `diag_convert.rs`, `diag_render.rs` | Pure conversion/rendering; exact codes. |
| 22-23 | CLI/user rendering | E2E or CLI renderer integration | `crates/velvet_ballistics/tests/cli_integration.rs` or existing renderer harness | User-visible contract from process boundary/shared CLI renderer. |
| 24 | Static/resource safety | Static | `moon ci`, `rg`/repo forbidden checks, clippy, review checklist | Safety/resource obligations are cheaper and stricter statically. |

Integration dominates because capability validation is a public verifier contract. Unit tests concentrate on pure grammar/diagnostic conversion. E2E stays narrow but must cover every new variant's user-facing rendering path.

## 3. BDD Scenarios

### Behavior: valid capability requirements pass through public validation
- Test function name: `fn validation_pipeline_returns_unit_when_contract_capabilities_are_schema_valid()`
- Given: workflow parts with one `Do` node at node index `0` for action `1`, and one `ActionContract { id: 1, required_capabilities: [network.github@1, secrets.read.github_token@1] }`.
- When: `ValidationPipeline::validate_with_contracts(parts, contracts)` runs.
- Then: result is exactly `Ok(())`.

### Behavior: empty required capabilities pass
- Test function name: `fn validation_pipeline_returns_unit_when_required_capabilities_are_empty()`
- Given: workflow parts with one `Do` node at node index `0` for action `1`, and matching `ActionContract { id: 1, required_capabilities: [] }`.
- When: public validation runs.
- Then: result is exactly `Ok(())`.

### Behavior: valid one-segment and dotted grammar pass
- Test function names: `fn capability_schema_returns_unit_when_name_is_single_lowercase_segment()`, `fn capability_schema_returns_unit_when_name_is_valid_dotted_hierarchy()`, `fn capability_schema_returns_unit_when_tail_contains_digit_or_underscore()`
- Given: used contract action `1` with names `network`, `network.github`, `secrets.read.github_token`, and `fs.read_tmp2` in separate cases.
- When: gate 12 capability schema validation runs.
- Then: each result is exactly `Ok(())`.

### Behavior: empty capability name is rejected
- Test function name: `fn validation_pipeline_returns_capability_name_empty_when_requirement_name_is_empty()`
- Given: used contract action `1` has `required_capabilities[0] = Capability { name: "", action: 1 }`.
- When: public validation runs.
- Then: result is exactly `Err(ValidationError::CapabilityNameEmpty { action_id: 1, capability_index: 0 })`.

### Behavior: too-long capability name is rejected
- Test function name: `fn validation_pipeline_returns_capability_name_too_long_when_name_has_129_bytes()`
- Given: used contract action `1` has a 129-byte otherwise-valid lowercase capability name at index `0`, and `MAX_CAPABILITY_NAME_BYTES = 128`.
- When: public validation runs.
- Then: result is exactly `Err(ValidationError::CapabilityNameTooLong { action_id: 1, capability_index: 0, len: 129, max: 128 })`.

### Behavior: invalid capability grammar is rejected
- Test function name: `fn validation_pipeline_returns_capability_name_invalid_when_name_contains_colon()`
- Given: used contract action `1` has `required_capabilities[0] = Capability { name: "network:github", action: 1 }`.
- When: public validation runs.
- Then: result is exactly `Err(ValidationError::CapabilityNameInvalid { action_id: 1, capability_index: 0, name: "network:github".to_string() })`.
- Required separate negative paths, each with exact same variant fields and exact rejected `name`: `.network`, `network.`, `network..github`, `Network`, `network-github`, `secrets/read`, `network github`, `netwørk`, `network.2github`, `network._github`, and `network\ngithub`.

### Behavior: action mismatch is rejected
- Test function name: `fn validation_pipeline_returns_capability_action_mismatch_when_requirement_action_differs_from_contract()`
- Given: used `ActionContract { id: 1, required_capabilities[0] = Capability { name: "network", action: 2 } }`.
- When: public validation runs.
- Then: result is exactly `Err(ValidationError::CapabilityActionMismatch { contract_action_id: 1, capability_action_id: 2, capability_index: 0 })`.

### Behavior: duplicate requirement in one contract is rejected
- Test function name: `fn validation_pipeline_returns_capability_duplicate_when_same_name_and_action_repeat_in_one_contract()`
- Given: used contract action `1` has `required_capabilities[0] = network@1` and `required_capabilities[1] = network@1`.
- When: public validation runs.
- Then: result is exactly `Err(ValidationError::CapabilityDuplicate { action_id: 1, first_index: 0, duplicate_index: 1, name: "network".to_string() })`.

### Behavior: earliest duplicate indexes are reported
- Test function name: `fn validation_pipeline_returns_earliest_capability_duplicate_when_multiple_duplicates_exist()`
- Given: used contract action `1` has `[network@1, fs.read@1, network@1, fs.read@1]`.
- When: public validation runs.
- Then: result is exactly `Err(ValidationError::CapabilityDuplicate { action_id: 1, first_index: 0, duplicate_index: 2, name: "network".to_string() })`.

### Behavior: same name across contracts is allowed
- Test function name: `fn validation_pipeline_returns_unit_when_same_capability_name_appears_in_different_contracts()`
- Given: workflow uses actions `1` and `2`; contract `1` requires `network@1`; contract `2` requires `network@2`.
- When: public validation runs.
- Then: result is exactly `Ok(())`.

### Behavior: first per-requirement schema error wins before duplicates and orphan checks
- Test function name: `fn validation_pipeline_returns_first_schema_error_before_duplicate_and_orphan_checks()`
- Given: workflow uses action `1`; contract `1` has `required_capabilities[0] = ""@1`, `[1] = "network:github"@2`, `[2] = "network"@1`, `[3] = "network"@1`; an additional otherwise valid orphan contract action `9` is also supplied.
- When: public validation runs.
- Then: result is exactly `Err(ValidationError::CapabilityNameEmpty { action_id: 1, capability_index: 0 })`.
- Precedence pinned: missing-contract checks run first; for present used contracts, capability schema checks run before orphan checks; per-requirement name/action errors run in capability index order before duplicate detection.

### Behavior: missing contract regression is exact
- Test function name: `fn validation_pipeline_returns_action_contract_missing_when_do_node_has_no_contract()`
- Given: workflow parts whose first node is a `Do` node at node index `0` for action `5`, and `action_contracts = []`.
- When: public validation runs.
- Then: result is exactly `Err(ValidationError::ActionContractMissing { action_id: 5, node_index: 0 })`.

### Behavior: orphan contract regression is exact
- Test function name: `fn validation_pipeline_returns_action_contract_orphan_when_contract_is_unused_and_no_earlier_error_exists()`
- Given: workflow parts with no `Do` node for action `9`, no missing contract condition, and one schema-valid `ActionContract { id: 9, required_capabilities: [] }`.
- When: public validation runs.
- Then: result is exactly `Err(ValidationError::ActionContractOrphan { action_id: 9 })`.

### Behavior: shared validation path invokes live gate implementation
- Test function name: `fn shared_validate_with_contracts_returns_capability_name_empty_when_live_gate_rejects_empty_name()`
- Given: the empty-name invalid fixture is called only through `vb_validate::shared::ValidationPipeline::validate_with_contracts` or the public shared equivalent.
- When: validation runs.
- Then: result is exactly `Err(ValidationError::CapabilityNameEmpty { action_id: 1, capability_index: 0 })`.

### Behavior: diagnostics map every new variant to stable codes
- Test function names:
  - `fn diagnostic_conversion_returns_e050d_when_error_is_capability_name_empty()` expects code `DiagnosticCode::new(0x050D)` / display `E050D`, message `capability name is empty for action 1 at required_capabilities[0]`.
  - `fn diagnostic_conversion_returns_e050e_when_error_is_capability_name_too_long()` expects `E050E`, message containing `capability name too long`, `action 1`, `required_capabilities[0]`, and `129 > 128`.
  - `fn diagnostic_conversion_returns_e050f_when_error_is_capability_name_invalid()` expects `E050F`, message containing `invalid capability name`, `action 1`, `required_capabilities[0]`, and `network:github`.
  - `fn diagnostic_conversion_returns_e0510_when_error_is_capability_action_mismatch()` expects `E0510`, message containing `capability action 2`, `contract action 1`, and `required_capabilities[0]`.
  - `fn diagnostic_conversion_returns_e0511_when_error_is_capability_duplicate()` expects `E0511`, message containing `duplicate capability requirement`, `action 1`, `network`, `required_capabilities[0]`, and `required_capabilities[1]`.

### Behavior: CLI/user-facing rendering covers all new variants
- Required oracle: process exit status code is exactly `1` (`std::process::ExitCode::FAILURE` on Linux), not merely non-success.
- Preferred tests, if fixture generation is available:
  - `fn cli_returns_exit_1_and_e050d_when_contract_has_empty_capability_name()` expects stderr/stdout contains `E050D` and `capability name is empty for action 1 at required_capabilities[0]`.
  - `fn cli_returns_exit_1_and_e050e_when_contract_has_too_long_capability_name()` expects `E050E` and `129 > 128`.
  - `fn cli_returns_exit_1_and_e050f_when_contract_has_invalid_capability_name()` expects `E050F` and `network:github`.
  - `fn cli_returns_exit_1_and_e0510_when_contract_has_capability_action_mismatch()` expects `E0510`, `capability action 2`, and `contract action 1`.
  - `fn cli_returns_exit_1_and_e0511_when_contract_has_duplicate_capability()` expects `E0511`, `network`, `required_capabilities[0]`, and `required_capabilities[1]`.
- Acceptable shared-renderer proof instead of five process tests: one process-level `E050F` test plus unit/integration tests proving the CLI validation command formats `diagnostic_from_error` / shared renderer output for all five variants without variant-specific matching. That proof must cite the exact renderer function and assert exact rendered strings for `E050D..E0511`.

## 4. Proptest Invariants

1. **Valid grammar acceptance**: generated valid names with byte length `1..=128` return exactly `Ok(())`; strategy: 1..8 segments, lowercase first char, tail `[a-z0-9_]`, dot join, reject above 128.
2. **Invalid grammar rejection**: generated non-empty names `1..=128` with uppercase, whitespace, slash, colon, hyphen, leading/trailing/doubled dot, non-ASCII, control byte, digit-start, or underscore-start segments return exactly `CapabilityNameInvalid` preserving original name.
3. **Length is byte length**: valid-shaped ASCII names length `1`, `127`, `128` do not return length errors; lengths `129..=256` return exactly `CapabilityNameTooLong { len, max: 128, .. }` before grammar classification.
4. **Action relation**: generated valid capability names with equal ids return `Ok(())` for relation; unequal ids return exact `CapabilityActionMismatch` with both ids and index.
5. **Duplicate detection earliest pair**: generated vectors length `0..=MAX_TEST_CAPABILITY_REQUIREMENTS` report no duplicate for unique pairs and exact earliest `first_index < duplicate_index` for repeated pairs.
6. **Duplicate scope**: same `(name, action)` in different contracts is never a duplicate for one contract; same pair within one contract always is.
7. **Determinism and precedence**: same ordered inputs repeatedly return the exact same `Result`; when both schema and orphan errors exist but no missing contract exists, exact result is the first schema error, not `ActionContractOrphan`.

Production-bound note: if implementation defines a maximum capability-list length, add boundary cases at `max`, `max + 1`, and name the exact error. If no production bound exists, proptest uses `MAX_TEST_CAPABILITY_REQUIREMENTS = 64` plus integration fixtures at `0`, `1`, `2`, `32`, and `64` to catch accidental low caps.

## 5. Fuzz Targets

### Fuzz Target: capability name validation boundary
- Input type: arbitrary bytes converted to valid UTF-8 strings only through safe construction; invalid UTF-8 corpus is tested at the harness boundary and must not enter production `&str` unsafely.
- Risk: panic from byte/char boundary handling, unchecked indexing, OOM/pathological formatting, incorrect control/non-ASCII acceptance.
- Corpus seeds: `""`, `"a"`, `"network"`, `"network.github"`, `"secrets.read.github_token"`, `"fs.read_tmp2"`, `".network"`, `"network."`, `"network..github"`, `"Network"`, `"network-github"`, `"network:github"`, `"secrets/read"`, `"network github"`, `"netwørk"`, `"network\ngithub"`, 128-byte valid name, 129-byte valid-shaped name.
- Oracle: exact `Ok(())`, `CapabilityNameEmpty`, `CapabilityNameTooLong`, or `CapabilityNameInvalid` according to contract; harness fails on panic, timeout, allocation blow-up, or mutated input.

### Fuzz Target: compiled action contract capability schema
- Input type: arbitrary small `ActionContract`-like structs using safe generated `ActionId` values and bounded vectors `0..=64`.
- Risk: duplicate-detection panics, action-id conversion mistakes, nondeterministic first-error ordering, pathological allocations from many capabilities.
- Corpus seeds: empty requirements, one valid, two unique, duplicate 0/1, duplicate 0/3, mismatched action, invalid before duplicate, duplicate before invalid, orphan plus invalid used contract, 64 unique capabilities.
- Oracle: exact first deterministic `Result<(), ValidationError>`; never panics; input contract remains unchanged.

## 6. Kani Harnesses

1. **Segment state machine completeness**: for ASCII byte arrays length `0..=128`, classification is exactly one of empty/valid/invalid; no panic/unreachable. Bound `0..=128`.
2. **Length boundary cannot overflow**: symbolic length `0..=256`; lengths above 128 classify too-long before per-byte grammar. Bound capacity 256.
3. **Duplicate indexes in bounds and ordered**: vector length `0..=8`, 4-name universe; reported duplicate has `first_index < duplicate_index < len` and equal referenced pair.
4. **First-error precedence**: vector length `0..=4`; missing contract precedes schema; schema precedes orphan; per-index schema precedes duplicate. Exact expected variant must match this plan.

If Kani is unavailable, the implementing state must create a tracked follow-up bead named `vb-nsnc-kani-proof` and add deterministic exhaustive unit/proptest fallback over the same bounds before closing this bead.

## 7. Mutation Checkpoints

Threshold: `cargo-mutants` must report `>= 90%` killed mutants for touched files. Critical mutant survivors below are not waivable by documentation; each requires the named test to fail red before implementation is accepted unless a proof shows Rust types make the mutant unconstructable.

- `len == 0` changed to `len <= 1`: killed by empty-name rejection and `"a"` valid boundary tests.
- `len > 128` changed to `len >= 128`: killed by 128-byte valid and 129-byte too-long tests.
- max constant `128` changed to `127` or `129`: killed by boundary unit/proptest tests.
- ASCII-only check removed: killed by `netwørk` invalid test and fuzz seed.
- uppercase allowed: killed by `Network` invalid test.
- colon/slash/hyphen/space/control allowed: killed by dedicated invalid grammar tests.
- leading/trailing/doubled dot allowed: killed by dedicated dot tests.
- digit/underscore segment start allowed: killed by segment-start tests.
- action compared with itself or equality inverted: killed by valid action-match plus mismatch integration tests.
- duplicate detection made global: killed by same-name-across-contracts allowed test.
- duplicate detection removed: killed by adjacent and non-adjacent duplicate tests.
- duplicate indexes reversed/off-by-one: killed by exact `CapabilityDuplicate` tests and Kani index proof.
- last error returned instead of first: killed by first-schema-error precedence test.
- orphan checked before schema for used invalid contract: killed by schema-before-orphan test.
- schema validation omitted from shared path or only added to `gate_12_14_15.rs`: killed by shared public-path empty-name test.
- missing contract payload changed: killed by exact `ActionContractMissing { action_id: 5, node_index: 0 }` test.
- orphan payload changed: killed by exact `ActionContractOrphan { action_id: 9 }` test.
- diagnostic code changed for any new variant: killed by `E050D..E0511` tests.
- diagnostic omits name/len/max/action ids/indexes: killed by message-content tests.
- CLI returns success or generic failure text: killed by exit-code-1 and exact diagnostic code/message CLI tests for all variants/shared renderer proof.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| valid empty list | used contract, no requirements | `Ok(())` | integration |
| valid min | `a` | `Ok(())` | unit |
| valid one segment | `network` | `Ok(())` | unit |
| valid dotted | `network.github` | `Ok(())` | unit |
| valid underscore/digit tail | `secrets.read.github_token`, `fs.read_tmp2` | `Ok(())` | unit |
| valid max length | 128-byte valid name | `Ok(())` | unit/proptest |
| empty | `""` | `Err(CapabilityNameEmpty { action_id: 1, capability_index: 0 })` | integration |
| too long | 129-byte valid-shaped name | `Err(CapabilityNameTooLong { action_id: 1, capability_index: 0, len: 129, max: 128 })` | unit/integration |
| invalid grammar classes | leading/trailing/doubled dot, uppercase, hyphen, colon, slash, space, non-ASCII, control, digit-start, underscore-start | `Err(CapabilityNameInvalid { action_id: 1, capability_index: 0, name: exact_input })` | unit/proptest/fuzz |
| mismatch | contract 1, capability 2 | `Err(CapabilityActionMismatch { contract_action_id: 1, capability_action_id: 2, capability_index: 0 })` | integration |
| duplicate adjacent | `network@1`, `network@1` | `Err(CapabilityDuplicate { action_id: 1, first_index: 0, duplicate_index: 1, name: "network" })` | integration |
| duplicate non-adjacent | `network@1`, `fs.read@1`, `network@1` | `Err(CapabilityDuplicate { action_id: 1, first_index: 0, duplicate_index: 2, name: "network" })` | integration |
| same name across contracts | `network@1`, `network@2` in separate used contracts | `Ok(())` | integration |
| first schema before orphan | invalid used contract plus orphan 9 | `Err(CapabilityNameEmpty { action_id: 1, capability_index: 0 })` | integration |
| missing contract | first node `Do` action 5, no contract | `Err(ActionContractMissing { action_id: 5, node_index: 0 })` | integration |
| orphan contract | no `Do` action 9, valid contract 9 | `Err(ActionContractOrphan { action_id: 9 })` | integration |
| diagnostics | each new variant | exact `E050D`, `E050E`, `E050F`, `E0510`, `E0511` and mandated message content | unit |
| CLI/user render | each new variant or shared renderer proof | exit code `1`, exact code and substring | e2e/integration |
| static safety | touched production files | no forbidden constructs, indexing/slicing/casts/arithmetic hazards, recursion, runtime JSON/YAML/HTTP | static |

## Static, Resource, Panic, and Command Gates

Implementing state must attach evidence for:

1. `moon ci` passes with exit status `0`.
2. Forbidden construct scan over touched production Rust files finds no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` uses.
3. Manual/static review or repo lint proves no unchecked indexing (`[]` on slices/vecs/strings), unchecked slicing, unchecked casts, or unchecked arithmetic were added in touched validation/diagnostic/CLI code.
4. Static scan proves no recursion and no unbounded `loop`; loops are bounded by provided slices, vectors, or string byte length.
5. Static scan proves no runtime core JSON/YAML/HTTP parsing additions in this bead.
6. Resource guardrail test/review proves rejected-name formatting is bounded by `MAX_CAPABILITY_NAME_BYTES` for normal invalid names and too-long names do not clone/format arbitrarily large input beyond the diagnostic contract fields `len` and `max`.
7. Focused `vb_validate` tests execute all unit/integration/proptest groups above.
8. CLI/render tests execute the `E050D..E0511` user-facing coverage above.
9. Fuzz targets compile and run for a bounded CI-safe smoke duration.
10. Kani harnesses pass, or follow-up bead `vb-nsnc-kani-proof` is created and bounded exhaustive fallback tests are present.
11. `cargo-mutants` over touched files reports `>= 90%` kill rate and zero survivors in the critical checkpoint list.

## Open Questions

- If existing diagnostic code allocation changes during implementation, update all planned assertions to the final stable assigned codes; under the current contract they are `E050D..E0511`.
- CLI fixture construction may require using an existing compile/validate fixture builder. Do not invent a new CLI protocol; reuse the project harness while preserving exact exit code `1` and exact diagnostic assertions.
