---- MODULE RecoveryCrashRestart ----
EXTENDS Naturals, FiniteSets

\* Obligation: TLA-REC-001 / PO-001.
\* Bounded crash/restart model for ordered journal recovery.  Fjall is
\* abstracted as durable headers, events, and snapshots; external actions are
\* ticket facts only and have no side-effect fairness assumption. Fairness is
\* restricted to crash/recovery decision steps so the terminal property cannot
\* be satisfied or defeated by infinite stuttering.

RunIds == {0, 1}
Attempts == {0, 1}
Seqs == 0..4
Watermarks == 0..4
Bool == {TRUE, FALSE}
Status == {"empty", "active", "waiting", "asking", "terminal"}
ActionState == {"none", "pending", "resolved"}
CollectState == {"none", "valid", "corrupt", "wrong_identity"}
Terminal == {"none", "recovered", "rejected"}
Errors == {"none", "no_recovery_data", "replay_divergence", "corrupt_snapshot", "non_idempotent_action_blocked", "collect_extra_hydration_failed"}

VARIABLES header, ordered, seq_gap, snapshot_valid, snapshot_watermark,
          tail_after_watermark, active_attempt, stale_attempt_present,
          mixed_stale_attempt, durable_slot, durable_taint_secret,
          recovered_taint_secret, fact_erased, wait_fact, ask_fact,
          action_state, action_duplicated, collect_state, lifecycle_only,
          terminal, error, crashed

vars == <<header, ordered, seq_gap, snapshot_valid, snapshot_watermark,
          tail_after_watermark, active_attempt, stale_attempt_present,
          mixed_stale_attempt, durable_slot, durable_taint_secret,
          recovered_taint_secret, fact_erased, wait_fact, ask_fact,
          action_state, action_duplicated, collect_state, lifecycle_only,
          terminal, error, crashed>>

Init ==
  /\ header \in Bool
  /\ ordered \in Bool
  /\ seq_gap \in Bool
  /\ snapshot_valid \in Bool
  /\ snapshot_watermark \in Watermarks
  /\ tail_after_watermark \in Bool
  /\ active_attempt \in Attempts
  /\ stale_attempt_present \in Bool
  /\ mixed_stale_attempt = FALSE
  /\ durable_slot \in Bool
  /\ durable_taint_secret \in Bool
  /\ recovered_taint_secret = FALSE
  /\ fact_erased = FALSE
  /\ wait_fact \in Bool
  /\ ask_fact \in Bool
  /\ action_state \in ActionState
  /\ action_duplicated = FALSE
  /\ collect_state \in CollectState
  /\ lifecycle_only \in Bool
  /\ terminal = "none"
  /\ error = "none"
  /\ crashed = FALSE

CanRecover ==
  /\ header
  /\ ordered
  /\ ~seq_gap
  /\ snapshot_valid
  /\ tail_after_watermark
  /\ durable_slot
  /\ collect_state # "corrupt"
  /\ collect_state # "wrong_identity"
  /\ action_state # "pending"
  /\ ~lifecycle_only

Crash ==
  /\ terminal = "none"
  /\ crashed' = TRUE
  /\ UNCHANGED <<header, ordered, seq_gap, snapshot_valid, snapshot_watermark,
                  tail_after_watermark, active_attempt, stale_attempt_present,
                  mixed_stale_attempt, durable_slot, durable_taint_secret,
                  recovered_taint_secret, fact_erased, wait_fact, ask_fact,
                  action_state, action_duplicated, collect_state, lifecycle_only,
                  terminal, error>>

RecoverFullJournal ==
  /\ crashed
  /\ terminal = "none"
  /\ CanRecover
  /\ terminal' = "recovered"
  /\ error' = "none"
  /\ recovered_taint_secret' = durable_taint_secret
  /\ mixed_stale_attempt' = FALSE
  /\ fact_erased' = FALSE
  /\ action_duplicated' = FALSE
  /\ UNCHANGED <<header, ordered, seq_gap, snapshot_valid, snapshot_watermark,
                  tail_after_watermark, active_attempt, stale_attempt_present,
                  durable_slot, durable_taint_secret, wait_fact, ask_fact,
                  action_state, collect_state, lifecycle_only, crashed>>

RecoverSnapshotTail ==
  /\ crashed
  /\ terminal = "none"
  /\ CanRecover
  /\ snapshot_watermark \in Watermarks
  /\ terminal' = "recovered"
  /\ error' = "none"
  /\ recovered_taint_secret' = durable_taint_secret
  /\ mixed_stale_attempt' = FALSE
  /\ fact_erased' = FALSE
  /\ action_duplicated' = FALSE
  /\ UNCHANGED <<header, ordered, seq_gap, snapshot_valid, snapshot_watermark,
                  tail_after_watermark, active_attempt, stale_attempt_present,
                  durable_slot, durable_taint_secret, wait_fact, ask_fact,
                  action_state, collect_state, lifecycle_only, crashed>>

RejectCorruptOrUnsupported ==
  /\ crashed
  /\ terminal = "none"
  /\ ~CanRecover
  /\ terminal' = "rejected"
  /\ error' = IF ~header \/ ~durable_slot THEN "no_recovery_data"
              ELSE IF ~ordered \/ seq_gap \/ ~tail_after_watermark THEN "replay_divergence"
              ELSE IF ~snapshot_valid THEN "corrupt_snapshot"
              ELSE IF action_state = "pending" THEN "non_idempotent_action_blocked"
              ELSE IF collect_state = "corrupt" \/ collect_state = "wrong_identity" THEN "collect_extra_hydration_failed"
              ELSE "replay_divergence"
  /\ recovered_taint_secret' = FALSE
  /\ mixed_stale_attempt' = FALSE
  /\ fact_erased' = FALSE
  /\ action_duplicated' = FALSE
  /\ UNCHANGED <<header, ordered, seq_gap, snapshot_valid, snapshot_watermark,
                  tail_after_watermark, active_attempt, stale_attempt_present,
                  durable_slot, durable_taint_secret, wait_fact, ask_fact,
                  action_state, collect_state, lifecycle_only, crashed>>

Stutter == UNCHANGED vars

Next == Crash \/ RecoverFullJournal \/ RecoverSnapshotTail \/ RejectCorruptOrUnsupported \/ Stutter

Spec ==
  /\ Init
  /\ [][Next]_vars
  /\ WF_vars(Crash)
  /\ WF_vars(RecoverFullJournal \/ RecoverSnapshotTail \/ RejectCorruptOrUnsupported)

NoSuccessWithoutDurableState ==
  terminal = "recovered" => header /\ durable_slot /\ ordered /\ ~seq_gap

NoStaleAttemptMixing == mixed_stale_attempt = FALSE

SnapshotTailAfterWatermark == terminal = "recovered" => tail_after_watermark

TaintExact == terminal = "recovered" => recovered_taint_secret = durable_taint_secret

ActionTicketNotDuplicated == action_duplicated = FALSE

CollectIdentityExact == terminal = "recovered" => collect_state = "none" \/ collect_state = "valid"

TypedFailureForInvalidInput ==
  terminal = "rejected" => error # "none"

LifecycleDiagnosticsNonAuthority ==
  lifecycle_only => terminal # "recovered"

MonotonicFacts == fact_erased = FALSE

EventuallyRecoveredOrRejected == <>(terminal # "none")

====
