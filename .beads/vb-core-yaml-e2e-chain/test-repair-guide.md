# Test Repair Guide: vb-core-yaml-e2e-chain State 9 Retry

## Decision

- Test plan review is approved; no State 7 plan repair is required.
- Test suite review is rejected because the repaired contract suite is still red on the preserved strict accepted-artifact implementation blocker.

## Exact Route

1. State 7: no-op. Keep `.beads/vb-core-yaml-e2e-chain/test-plan.md` as repaired in lines 319-370. Do not relax density, fuzz, or exact-assertion requirements.
2. State 8: no test repair required unless implementation API changes force mechanical updates. Preserve `tests/vb_core_yaml_e2e_chain_contract.rs:166-183` exactly in intent: strict YAML-origin `submit_artifact` must return an artifact whose digest and verification digest equal `workflow.digest()`, whose verification flags are true, and whose gate count equals `REQUIRED_GATE_COUNT`.
3. Implementation repair owner: fix the production checksum/parity defect causing `artifact checksum mismatch` for `submit_artifact(&journal, &workflow_from_yaml, RuntimePolicy::Strict)`.
4. Return to State 9 from Tier 0 after implementation repair. Coverage and mutation remain blocked until the contract suite is green.

## Required Evidence for Next Review

- `rtk cargo test -p vb_compile --test vb_core_yaml_e2e_chain_strict_yaml -- --nocapture` passes with 10 tests.
- `rtk cargo test -p velvet-ballistics-workspace --test vb_core_yaml_e2e_chain_contract -- --nocapture` passes with all 35 tests.
- The three fuzz bins `strict_yaml_profile`, `accepted_artifact_decode`, and `recovery_decode` remain present and smoke-runnable or are replaced by stricter cargo-fuzz evidence.
