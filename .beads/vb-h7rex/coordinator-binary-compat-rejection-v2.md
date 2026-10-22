# V2 binary compatibility rejection

The constructor repair and new artifact-pin unit checks are retained, but V2 is not accepted. Its serde_json named-map test does not represent the postcard sequence inside RunCheckpointV1.admission. The new policy_pin field is inserted before budget and idempotency_attested. Serde default does not insert a missing middle element into the old binary sequence. New checkpoint roundtrip test actually serializes only RunAdmission. Claims of old checkpoint compatibility and full checkpoint coverage in handoff-v2.md are incorrect.

Actual production durable checkpoint decode is Runtime in crates/vb_runtime/src/runtime.rs around497-498: postcard::from_bytes(durable_payload) into RunCheckpointV1. That real boundary needs an explicit version-aware decode or schema migration. New checkpoint captures must preserve the pin. Historical unsupported versions may be rejected through a typed error before interpreting the new layout, as allowed by the child contract. No silent default authority and no JSON surrogate. Existing historical assertions must remain; only new rejected surrogate tests may be replaced.

Raw V2 tests/clippy pass within their real scope; they do not close binary compatibility, public Runtime/Fjall crash recovery, or parent effect authorization.
