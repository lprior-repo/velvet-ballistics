# vb-kyyf BDD-KYYF-003 durable scenario evidence

bead: vb-kyyf
scenario_id: BDD-KYYF-003
given: executable contract fixture for BDD-KYYF-003
when: public surface is exercised and normalized observations are collected
then: ReplayPolicyBlocked
public_surface: vb_runtime recovery API
evidence_artifact: .evidence/vb-kyyf/non-replay-safe-actions.md
normalized_digest_or_mismatch: ReplayPolicyBlocked
raw_observation_summary:
before_events=[ActionScheduled { run: RunId(30003), seq: EventSeq(0), step: StepIdx(2), action: ActionId(3), attempt: 1 }]
first_blocked=Err("ReplayPolicyBlocked")
second_blocked=Err("ReplayPolicyBlocked")
after_first_events=[ActionScheduled { run: RunId(30003), seq: EventSeq(0), step: StepIdx(2), action: ActionId(3), attempt: 1 }]
after_second_events=[ActionScheduled { run: RunId(30003), seq: EventSeq(0), step: StepIdx(2), action: ActionId(3), attempt: 1 }]
