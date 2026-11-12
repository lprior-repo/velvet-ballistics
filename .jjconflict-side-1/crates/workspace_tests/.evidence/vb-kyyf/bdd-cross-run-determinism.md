# vb-kyyf BDD-KYYF-001 durable scenario evidence

bead: vb-kyyf
scenario_id: BDD-KYYF-001
given: executable contract fixture for BDD-KYYF-001
when: public surface is exercised and normalized observations are collected
then: normalized digest
public_surface: vb_runtime public API
evidence_artifact: .evidence/vb-kyyf/bdd-cross-run-determinism.md
normalized_digest_or_mismatch: normalized digest
raw_observation_summary:
left:result=Ok,taint=Clean,event_signature=8,event_payload_signature=7,digest_status=workflow_source=true,compiled_ir=true,action_abi=true,policy=true,replay_policy_blocked=false,unsupported_generated_subset=false,semantic_slot_signature=42,semantic_action_signature=0,semantic_suspension=false,semantic_taint_signature=2
right:result=Ok,taint=Clean,event_signature=8,event_payload_signature=7,digest_status=workflow_source=true,compiled_ir=true,action_abi=true,policy=true,replay_policy_blocked=false,unsupported_generated_subset=false,semantic_slot_signature=42,semantic_action_signature=0,semantic_suspension=false,semantic_taint_signature=2
comparison=Ok
