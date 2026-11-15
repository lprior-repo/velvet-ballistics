# Policy pin expectations during full journal replay

Nonempty caller policy expectations must match a consistent RunPolicyPin::Pinned admission identity. Missing admission preserves the existing StepIdx::ZERO refusal; unverified/conflicting pins refuse using the first caller label, and mismatched expectations refuse at the first mismatched caller label. Empty expectations and NoRecoveryData precedence remain unchanged. Verify the actual admitted policy digest, not a RuntimePolicy/profile hash. Check before replay mutates the action tracker. No new API, event variant, dependency or persisted format.

Public tests use genuine current Strict submission with issued artifact, Budget/Digest/Pinned admission and positive origin; require exact returned events, error variants/step labels, retained tracker entries and raw stored journal bytes. Invalid-authority histories append current typed events only as negative fixtures. Independent CODE/TEST review and canonical gates precede landing.
