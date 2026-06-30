# Domain Model Review: vb-qi37.6

## Verdict
STATUS: CONTRACT_READY_WITH_BLOCKERS

The mapped domain has the right security shape only if capability data is treated as a single typed value flow: validated `ActionContract` -> accepted artifact certificate -> run grant profile -> admission -> Do dispatch -> UI projection. Current State 2 evidence shows several breaks in that flow, so implementation must repair the model before release evidence can pass.

## Domain Objects
- `Capability`: value object. Identity is exact name plus action id. It is not a hierarchy and not a prefix namespace.
- `CapabilitySet`: run grant profile. It is not an ACL with wildcard semantics. It is a least-privilege exact profile.
- `ActionContract`: authority for what an action requires. It must be present before artifact acceptance.
- `AcceptedArtifact`: durable certificate. It must preserve required capabilities and proof gates; it must not synthesize, erase, or reinterpret capability requirements.
- `RunAdmission`: admitted run fact. It must exist only after strict artifact and capability checks pass.
- `ActionDescriptionView`: UI projection. It has no authority to invent capability data.

## Illegal States To Make Unrepresentable Or Fail Closed
- Do action exists without one resolved `ActionContract`.
- Required capability name is empty, too long, malformed, duplicate, or tied to a different action id.
- Accepted artifact has empty `required_capabilities` when validated contracts required non-empty capabilities.
- Strict/Journaled accepted artifact carries a 2-gate proof when runtime admission requires 15 gates.
- Runtime admission allocates or journals a run after capability denial.
- Do execution reaches external action dispatch without contracts or without checking admitted capabilities.
- UI shows capability requirements from a data source different from the verifier/runtime source.

## Boundary Decisions
- Pure/core boundary: `CapabilitySet::grants`, schema-valid abstractions, profile cardinality, and certificate preservation belong to Verus/Kani/proptest.
- Temporal boundary: artifact admission, denial, run allocation, journaling, and Do execution ordering belong to TLA+ plus integration/state-model evidence.
- Persistence boundary: Fjall/postcard bytes are a trusted shell for Verus and require integration/fault-injection evidence.
- UI boundary: typed model projection only; not a verifier.

## Blocker Impact
- `BLOCKER_GATE_COUNT_ALIGNMENT`: release evidence must fail until storage/runtime agree on 15-gate accepted artifacts or an approved release waiver exists.
- `BLOCKER_REQUIRED_CAPABILITY_SOURCE`: release evidence must fail until accepted artifacts preserve required capabilities from validated action contracts.
- `BLOCKER_RUNTIME_GRANT_API`: capability-protected workflows cannot be admitted through public runtime APIs until explicit grant input exists or another typed profile source is specified.
- `BLOCKER_ACTION_CONTRACT_THREADING`: Do execution cannot succeed safely through shard drive until validated contracts are threaded to `drive_deterministic_full`.

## Review Requirements For Next State
- Independent contract-verification review must approve or reject these artifacts before test planning/implementation consumes them.
- Any implementation that weakens exact-match or count-exact admission must update this contract first.
