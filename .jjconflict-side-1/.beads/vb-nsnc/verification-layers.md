# Verification Layers: vb-nsnc

## Boundary

**Verified kernel:** `vb_validate::gates` — pure capability schema validation (grammar, length, action relation, duplicate detection)

**Lean contract projection:** `.beads/vb-nsnc/lean-contract.md` — theorems for grammar_valid, length_bound, first_error, duplicate_detection, duplicate_scope, action_relation

**Runtime shell:** `vb_validate::gates::validate_gate_12_action_contract_completeness`, `validate_action_contract_capability_schema`, `validate_required_capability` — orchestration over `WorkflowParts` and `ActionContract` slices

**External systems excluded from formal proof:** Fjall storage, IPC frames, action ABI, generated workflows

## Layer Assignment

| Contract Clause | Verification Layer | Tool/Command | Evidence Artifact |
|----------------|-------------------|--------------|------------------|
| I3 (valid name grammar) | kani | `cargo kani --tests -p vb_validate --harness capability_schema_kani` | formal-verification-report.md |
| I3 (valid name grammar) | proptest | `cargo nextest run -p vb_validate --test capability_contract_schema proptest` | nextest report |
| I3 (grammar invalid rejection) | cargo-fuzz | `cargo fuzz run capability_name_schema` | fuzz report |
| I4 (action relation) | kani | `cargo kani --tests -p vb_validate --harness action_relation_kani` | formal-verification-report.md |
| I4 (action relation) | proptest | `cargo nextest run -p vb_validate --test capability_contract_schema action_relation` | nextest report |
| I5 (no duplicates) | kani | `cargo kani --tests -p vb_validate --harness duplicate_detection_kani` | formal-verification-report.md |
| I5 (no duplicates) | proptest | `cargo nextest run -p vb_validate --test capability_contract_schema duplicate` | nextest report |
| I5 (earliest pair) | kani | `cargo kani --tests -p vb_validate --harness first_error_precedence_kani` | formal-verification-report.md |
| I9 (first error wins) | kani | `cargo kani --tests -p vb_validate --harness first_error_precedence_kani` | formal-verification-report.md |
| I9 (first error wins) | proptest | `cargo nextest run -p vb_validate --test capability_contract_schema first_error` | nextest report |
| PRE-1 (trusted WorkflowParts) | integration | `cargo nextest run -p vb_validate --test capability_contract_schema integration` | nextest report |
| POST-1 (schema valid passes) | integration | `cargo nextest run -p vb_validate --test capability_contract_schema integration` | nextest report |
| POST-3 (empty name rejected) | unit + integration | `cargo nextest run -p vb_validate --test capability_contract_schema` | nextest report |
| POST-4 (action mismatch rejected) | unit + integration | `cargo nextest run -p vb_validate --test capability_contract_schema` | nextest report |
| POST-5 (duplicate rejected) | unit + integration | `cargo nextest run -p vb_validate --test capability_contract_schema` | nextest report |
| POST-6 (invalid grammar rejected) | unit + integration | `cargo nextest run -p vb_validate --test capability_contract_schema` | nextest report |
| ERR-taxonomy (5 variants) | unit | `cargo nextest run -p vb_validate --test capability_contract_schema error_taxonomy` | nextest report |
| ERR-taxonomy (E050D..E0511) | unit | `cargo nextest run -p vb_validate --test diag_convert` | nextest report |
| ERR-taxonomy (CLI render) | e2e | `cargo nextest run -p velvet-ballastics --test cli_integration` | nextest report |
| I10 (missing/orphan preserved) | integration | `cargo nextest run -p vb_validate --test capability_contract_schema regression` | nextest report |
| Static safety (no unsafe/unwrap/panic) | static-scan | `moon ci` | clippy/fmt report |
| Static safety (no hot JSON/YAML/HTTP) | static-scan | `rg 'JSON|YAML|HTTP' vb_validate/src/*.rs` | rg report |
| Resource bounds (bounded loops) | static-scan | `moon ci` | source-length report |
| API compat | api-compat | `cargo semver-checks check-release -p vb_validate` | semver report |

## Lean Scope

**Theorem module:** `VBValidate.Capability` (lean4 project under `leans/vb_validate` or equivalent)

**Rust target:** `vb_validate::gates::is_capability_name_grammar_valid`, `validate_capability_name`, `validate_no_duplicate_capability_requirements`

**Abstraction relation:** Rust `String` ↔ Lean `String` (byte sequence); Rust `ActionId` ↔ Lean `Nat` (bounded); Rust `Capability` ↔ Lean `Capability` (name + action pair)

**Shell exclusions:** `validate_gate_12_action_contract_completeness` (iterates over `WorkflowParts`), `validate_action_contract_capability_schema` (orchestration), `validate_required_capability` (calls pure functions with context)

**Non-goals:**
- Diagnostic string formatting
- CLI exit codes
- WorkflowParts construction
- ActionContract construction

## Verification Evidence Summary

| Layer | Count | Critical |
|-------|-------|----------|
| kani | 4 | yes (bounded model check for grammar, action, duplicates, precedence) |
| proptest | 5 | yes (grammar fuzz, action relation, duplicate, first error, regression) |
| cargo-fuzz | 1 | yes (grammar adversarial) |
| unit | 8 | yes (exact error variants, diagnostic codes) |
| integration | 3 | yes (pipeline, regressions, precedence) |
| e2e | 1 | yes (CLI rendering) |
| static | 3 | yes (no forbidden constructs) |
| api-compat | 1 | no (future compatibility) |
| **Total** | **26** | |

## Waivers

**WAIVER-001:** `validate_gate_12_action_contract_completeness` missing/orphan orchestration — verified by Kani harness on bounded input space; proptest with generated `WorkflowParts`; integration tests with exact payloads

**WAIVER-002:** Diagnostic string formatting — verified by unit tests on exact `E050D..E0511` codes and messages; CLI integration tests on exit code 1 and rendered output

**WAIVER-003:** `lean-contract.md` theorems not yet encoded in Lean — this verification layer plan provides the contract for future Lean implementation; Kani/proptest provide compensating evidence until Lean is available
