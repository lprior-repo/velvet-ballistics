# vb-kyyf BDD-KYYF-004 durable scenario evidence

bead: vb-kyyf
scenario_id: BDD-KYYF-004
given: executable contract fixture for BDD-KYYF-004
when: public surface is exercised and normalized observations are collected
then: ReplayDigestMismatch
public_surface: vb_storage journal and recovery APIs
evidence_artifact: .evidence/vb-kyyf/recovery-bdd-errors.md
normalized_digest_or_mismatch: ReplayDigestMismatch
raw_observation_summary:
case=corrupt-snapshot,attempt1=MissingSnapshot,attempt2=MissingSnapshot,expected_typed_error=MissingSnapshot
case=sequence-gap,attempt1=ReplayDivergence,attempt2=ReplayDivergence,expected_typed_error=ReplayDivergence
case=duplicate-sequence,attempt1=ReplayDivergence,attempt2=ReplayDivergence,expected_typed_error=ReplayDivergence
case=out-of-order-sequence,attempt1=ReplayDivergence,attempt2=ReplayDivergence,expected_typed_error=ReplayDivergence
case=workflow-source-digest-mismatch,attempt1=WorkflowSourceDigestMismatch,attempt2=WorkflowSourceDigestMismatch,expected_typed_error=WorkflowSourceDigestMismatch
case=compiled-ir-digest-mismatch,attempt1=CompiledIrDigestMismatch,attempt2=CompiledIrDigestMismatch,expected_typed_error=CompiledIrDigestMismatch
case=action-abi-digest-mismatch,attempt1=ActionAbiMismatch,attempt2=ActionAbiMismatch,expected_typed_error=ActionAbiMismatch
case=policy-digest-mismatch,attempt1=PolicyDigestMismatch,attempt2=PolicyDigestMismatch,expected_typed_error=PolicyDigestMismatch
