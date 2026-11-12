bead_id: vb-ssei
phase: 7
updated_at: 2026-05-18T21:50:13Z
attempt: 1-of-7

# Test plan

- Happy path: all verification gates pass -> strict admission accepts.
- Happy path: strict verified safe workflow -> certificate carries capability/idempotency evidence.
- Error path: capability missing -> exact `CapabilityDenied`.
- Error path: artifact/proof digest mismatch -> exact `ArtifactDigestMismatch`.
- Traceability: acceptance catalog maps `vb-ssei` scenario to executable target and removes deferred follow-up.
