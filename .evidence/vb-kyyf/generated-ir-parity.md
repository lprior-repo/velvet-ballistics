# vb-kyyf BDD-KYYF-005 durable scenario evidence

bead: vb-kyyf
scenario_id: BDD-KYYF-005
given: executable contract fixture for BDD-KYYF-005
when: public surface is exercised and normalized observations are collected
then: generated replay parity digest
public_surface: vb_codegen and vb_runtime public surfaces
evidence_artifact: .evidence/vb-kyyf/generated-ir-parity.md
normalized_digest_or_mismatch: generated replay parity digest
raw_observation_summary:
ir_observation:result=Ok,taint=Clean,event_signature=8,event_payload_signature=7,digest_status=workflow_source=true,compiled_ir=true,action_abi=true,policy=true,replay_policy_blocked=false,unsupported_generated_subset=false,semantic_slot_signature=42,semantic_action_signature=0,semantic_suspension=false,semantic_taint_signature=2
generated_observation:result=Ok,taint=Clean,event_signature=8,event_payload_signature=7,digest_status=workflow_source=true,compiled_ir=true,action_abi=true,policy=true,replay_policy_blocked=false,unsupported_generated_subset=false,semantic_slot_signature=42,semantic_action_signature=0,semantic_suspension=false,semantic_taint_signature=2
