# Packed executor held-version expectation normalization

Parent storage-unit-r616.log executes the unchanged executor::versioned_codec_tests::versioned_codec_packed_held_minor_and_patch_versions case and reports exact held1.2.0/required2.0.0/Rejected; full storage result is1834 passed,1 failed, exit101. The packed verifier uses the same authorized compiled-IR reader before pack reconstruction and expected-binding checks; CURRENT_IR_VERSION remains2.0.0 and no IR migration or replay allowance exists.

Retain the original registered case name, both original raw PackedFixture instances and held1.2.0/held1.0.1 literals as exact ExecutorError::Artifact(JournalError::ArtifactVersionIncompatible) negatives, asserting held, required2.0.0 and Rejected. Unexpected success or another error is fatal. Preserve raw carrier bytes and absent binding before intentionally changing the outer held version.

Retain the existing bound key payload at held2.1.0 and Some(empty) at held2.0.1 cold roundtrips unchanged. Add supported raw-current2.2.0 and raw-current2.0.1 cold roundtrips using the same original PackedFixture constructor, keeping the existing strict sequence1, exact artifact bytes, whole workflow, exact pack list, persistence and actual reopen assertions through the unchanged cold_roundtrip/read helpers.

Only executor/versioned_codec_tests.rs changes, with small private exact-error and fixture helpers if needed. Every changed or new function is at most25 nonblank, noncomment lines including signature and punctuation, parameters at most5, file at most300. No production versions, compatibility policy, writer bytes, profiles, proof flags, APIs, schemas, dependencies, gates or other tests change. Source CODE and independent TEST review remain required; parent performs native qualification.

Preserve frozen aa7e0e0d7df81afa8200085958c01ed56bd58d4d and its capsule. The separately assigned recovery identity analysis resumes afterward; no identity test or full-checkpoint semantics are changed by this test-only successor.
