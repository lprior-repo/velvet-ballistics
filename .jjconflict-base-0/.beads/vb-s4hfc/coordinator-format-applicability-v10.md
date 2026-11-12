# Canonical formatting applicability

V9 bare rustfmt used different style settings. The pinned cargo formatter changed only wrapping of the known-kind38 assertion and import item order in the two new test files. No test/assertion was removed or weakened; production source is unchanged from V9. Actual pinned cargo fmt check passed in ASSEMBLY_V10_FMT.json. Exact final source hashes are final-source-v10.json. V9 review remains scoped to V9 and any final formatting finding needs independent disposition on V10.

```diff
diff --git a/crates/vb_storage/src/codec/tests/replay_integrity.rs b/crates/vb_storage/src/codec/tests/replay_integrity.rs
index bb52e2cdd..ce5b74d9e 100644
--- a/crates/vb_storage/src/codec/tests/replay_integrity.rs
+++ b/crates/vb_storage/src/codec/tests/replay_integrity.rs
@@ -229,7 +229,10 @@ fn kind_28_29_31_through_35_admission_for_journal_magic() {
         );
     }
     // Kinds outside the journal range remain rejected.
-    assert!(is_known_record_kind(38), "kind 38 (PolicyDecision) must be known");
+    assert!(
+        is_known_record_kind(38),
+        "kind 38 (PolicyDecision) must be known"
+    );
     assert!(
         validate_kind_family(MAGIC_JOURNAL_EVENT, 38).is_ok(),
         "kind 38 must be admitted for journal magic"
diff --git a/crates/vb_storage/tests/policy_decision_full_record.rs b/crates/vb_storage/tests/policy_decision_full_record.rs
index d118379e7..bfd396238 100644
--- a/crates/vb_storage/tests/policy_decision_full_record.rs
+++ b/crates/vb_storage/tests/policy_decision_full_record.rs
@@ -9,6 +9,7 @@ use vb_core::value::Taint;
 #[path = "policy_decision_full_record/pd_helpers.rs"]
 mod pd_helpers;
 use vb_core::workflow::LifecycleState;
+use vb_storage::JournalError;
 use vb_storage::codec::{decode_journal_event, encode_record};
 use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
 use vb_storage::derive_lifecycle_state_from_events;
@@ -16,7 +17,6 @@ use vb_storage::events::JournalEvent;
 use vb_storage::journal::FjallJournal;
 use vb_storage::records::RecordKind;
 use vb_storage::types::EventSeq;
-use vb_storage::JournalError;
 
 #[test]
 fn policy_decision_event_round_trip() -> Result<(), Box<dyn std::error::Error>> {
diff --git a/crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs b/crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs
index 76d53a304..7f4828ec2 100644
--- a/crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs
+++ b/crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs
@@ -1,6 +1,6 @@
 #![forbid(unsafe_code)]
 
-use vb_core::action::{compute_action_idempotency_key, ActionTicket};
+use vb_core::action::{ActionTicket, compute_action_idempotency_key};
 use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
 use vb_core::policy::{
     DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy, PolicyDigest, PolicyLayer,
```
