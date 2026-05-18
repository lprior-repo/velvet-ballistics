bead_id: vb-j0m0
bead_title: quality: Add unsafe boundary fuzz harnesses
phase: 1
updated_at: 2026-05-17T20:30:00Z
attempt: 1-of-7

# Baseline Report: Current Fuzz State

## Existing Fuzz Infrastructure

### Fuzz Crate: `velvet-ballastics-fuzz`
- Location: `fuzz/` in workspace root
- Framework: libfuzzer-sys (cargo-fuzz compatible)
- Features: `fuzz` feature flag for stdin-driven execution
- Dependencies: libfuzzer-sys, postcard, blake3, tempfile, xtask, vb_* crates

### Existing Fuzz Targets (39 binaries in fuzz/src/bin/)
1. `ipc_frame.rs` - IPC frame parsing (calls `fuzz_lib::fuzz_ipc_frame`)
2. `journal_event.rs` - Journal record envelope (calls `fuzz_lib::fuzz_journal_event`)
3. `boundary_inventory_parser.rs` - Boundary inventory parsing (libfuzzer-style, calls `parse_inventory`)
4. `vb_qi37_12_persisted_payload_decode.rs` - Persisted payload decode (calls `fuzz_lib::fuzz_vb_qi37_12_persisted_payload_decode`)
5. `vb_ui_model_postcard_decode.rs` - UI model envelope decode
6. `admission_flow.rs` - Admission flow with workflow construction
7. `admission_fuzz.rs` - Arbitrary artifact bytes admission
8. `expr_eval.rs` - Expression evaluator postcard decode
9. `compiled_ir.rs` - Compiled IR decode/validation
10. `expression.rs` - Expression lexer/parser/compiler/evaluator
11. `yaml_events.rs` - YAML event parser
12. `accessor_traversal.rs` - Accessor path traversal
13. `slot_value_roundtrip.rs` - SlotValue postcard roundtrip
14. `resource_budget.rs` - Resource budget counting
15. `budget_compute.rs` - WholeWorkflowBudget compute
16. `taint_propagation.rs` - Taint propagation invariant
17. `generated_compare.rs` - Generated-vs-IR comparison
18. `verifier_gates.rs` - Validation gate exercises
19. `recovery_decode.rs` - Recovery snapshot/frame/journal decode
20. `accepted_artifact_decode.rs` - Accepted artifact decode
21. `accepted_artifact_envelope_qi37_4_2.rs` - Accepted artifact envelope
22. `boundary_evidence_reference.rs` - Boundary evidence reference
23. `boundary_metadata.rs` - Boundary metadata
24. `capability_name_schema.rs` - Capability name schema
25. `capability_contract_schema.rs` - Capability contract schema
26. `aggregate_artifact_budget.rs` - Aggregate artifact budget
27. `aggregate_workflow_budget.rs` - Aggregate workflow budget
28. `step_budget_new.rs` - StepBudget::new clamping
29. `structured_status_render_hostile.rs` - Structured status render
30. `xtask_parse_argv_hostile.rs` - xtask argv parsing
31. `xtask_parse_options_hostile.rs` - xtask options parsing
32. `strict_yaml_profile.rs` - Strict YAML profile
33. `vb_qi37_12_persisted_payload_decode.rs` - Persisted payload decode
34. `ipc_decode.rs` - IPC decode
35. `collect_page_pagination.rs` - Page pagination
36. `replay_events.rs` - Event replay
37. `extract_terminal.rs` - Terminal extraction
38. `action_tracker.rs` - Action tracker
39. `recover_runtime_frame_seed_contract.rs` - Runtime frame seed contract

### Existing Fuzz Target Bodies (in fuzz/src/lib.rs)
- `fuzz_ipc_frame` - IPC frame parsing with bounded input
- `fuzz_journal_event` - Journal event envelope decode
- `fuzz_vb_qi37_12_persisted_payload_decode` - Persisted payload with truncation/corruption tests
- `fuzz_vb_ui_model_postcard_decode` - UI model envelope decode
- `fuzz_admission_flow` - Admission flow with all policies
- `fuzz_admission_fuzz` - Arbitrary artifact bytes admission
- `fuzz_expr_eval` - Expression evaluator
- `fuzz_accessor_traversal` - Accessor path traversal
- `fuzz_slot_value_roundtrip` - SlotValue roundtrip
- `fuzz_budget_compute` - Budget computation
- `fuzz_strict_artifact_decoder` - Strict artifact decode
- `fuzz_digest_coherence` - Digest coherence
- `fuzz_readback_family_set` - Readback family set
- `fuzz_admission_input_surface` - CLI/runtime admission input
- `fuzz_strict_yaml_profile` - Strict YAML profile
- `fuzz_accepted_artifact_decode` - Accepted artifact decode
- `fuzz_recovery_decode` - Recovery decode
- `fuzz_step_budget_new` - StepBudget clamping

### Boundary Inventory (vb_boundary_inventory)
- BoundaryClass: CAbi, Ffi, Ipc, ExternalBinary, Decoder, GeneratedCode, UnsafeAdjacentDependency, Unknown
- EvidenceRequirement: FuzzOrIsolationOrManualQa
- UnsafeIsolationStatus: Complete { boundary_count }

### Current Gaps Identified
1. **IPC Frame Boundary**: `fuzz_ipc_frame` exists but targets only the libfuzzer entry point; no dedicated harness for truncated frames, oversized payloads, or malformed headers with explicit typed error assertions
2. **Storage Envelope Decoding**: `fuzz_journal_event` exists but no dedicated harness for corrupt digests, invalid envelopes, or truncated data with typed error assertions
3. **Binary Payload Decoding**: `fuzz_vb_qi37_12_persisted_payload_decode` exists with truncation/corruption tests but needs expansion for encoding attacks
4. **External Input Adapters**: `boundary_inventory_parser.rs` exists as a libfuzzer target but no comprehensive harness for all external input adapter boundaries

### CI Gate Status
- `moon ci` is the canonical gate
- Rust: zero-tolerance lint (no unsafe, unwrap, expect, panic, todo, unimplemented, dbg)
- Tests must compile and run
