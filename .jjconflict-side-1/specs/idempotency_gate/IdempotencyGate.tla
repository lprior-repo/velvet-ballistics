---- MODULE IdempotencyGate ----
EXTENDS Naturals

\* Obligations: TLA-RETRY-001, TLA-REPLAY-002, TLA-ADMIT-003.
\* Standalone finite lifecycle model for idempotency validation, certificate
\* admission, retry/replay scheduling, and duplicate completion collapse.

Bool == {TRUE, FALSE}
DecisionStates == {"unknown", "accepted", "rejected"}
AdmissionStates == {"pending", "runnable", "denied"}
Actions == {"action_a", "action_b"}
Runs == {"run_1", "run_2"}
Tickets == {"ticket_a", "ticket_b"}
Digests == {"digest_a", "digest_b"}

VARIABLES decision, evidence_present, evidence_passed, schema_compatible,
          admission, scheduled, resolved, recorded_action, recorded_run,
          recorded_ticket, recorded_digest, completion_action, completion_run,
          completion_ticket, completion_digest, duplicate_action,
          duplicate_run, duplicate_ticket, duplicate_digest,
          duplicate_accepted

vars == <<decision, evidence_present, evidence_passed, schema_compatible,
          admission, scheduled, resolved, recorded_action, recorded_run,
          recorded_ticket, recorded_digest, completion_action, completion_run,
          completion_ticket, completion_digest, duplicate_action,
          duplicate_run, duplicate_ticket, duplicate_digest,
          duplicate_accepted>>

Init ==
  /\ decision = "unknown"
  /\ evidence_present = FALSE
  /\ evidence_passed = FALSE
  /\ schema_compatible \in Bool
  /\ admission = "pending"
  /\ scheduled = FALSE
  /\ resolved = FALSE
  /\ recorded_action \in Actions
  /\ recorded_run \in Runs
  /\ recorded_ticket \in Tickets
  /\ recorded_digest \in Digests
  /\ completion_action \in Actions
  /\ completion_run \in Runs
  /\ completion_ticket \in Tickets
  /\ completion_digest \in Digests
  /\ duplicate_action \in Actions
  /\ duplicate_run \in Runs
  /\ duplicate_ticket \in Tickets
  /\ duplicate_digest \in Digests
  /\ duplicate_accepted = FALSE

ValidateAccept ==
  /\ decision = "unknown"
  /\ decision' = "accepted"
  /\ UNCHANGED <<evidence_present, evidence_passed, schema_compatible,
                  admission, scheduled, resolved, recorded_action,
                  recorded_run, recorded_ticket, recorded_digest,
                  completion_action, completion_run, completion_ticket, completion_digest,
                  duplicate_action, duplicate_run, duplicate_ticket, duplicate_digest,
                  duplicate_accepted>>

ValidateReject ==
  /\ decision = "unknown"
  /\ decision' = "rejected"
  /\ UNCHANGED <<evidence_present, evidence_passed, schema_compatible,
                  admission, scheduled, resolved, recorded_action,
                  recorded_run, recorded_ticket, recorded_digest,
                  completion_action, completion_run, completion_ticket, completion_digest,
                  duplicate_action, duplicate_run, duplicate_ticket, duplicate_digest,
                  duplicate_accepted>>

EmitPassingCertificate ==
  /\ decision = "accepted"
  /\ evidence_present' = TRUE
  /\ evidence_passed' = TRUE
  /\ UNCHANGED <<decision, schema_compatible, admission, scheduled, resolved,
                  recorded_action, recorded_run, recorded_ticket,
                  recorded_digest, completion_action, completion_run,
                  completion_ticket, completion_digest, duplicate_action,
                  duplicate_run, duplicate_ticket, duplicate_digest,
                  duplicate_accepted>>

EmitFailingCertificate ==
  /\ decision = "rejected"
  /\ evidence_present' = TRUE
  /\ evidence_passed' = FALSE
  /\ UNCHANGED <<decision, schema_compatible, admission, scheduled, resolved,
                  recorded_action, recorded_run, recorded_ticket,
                  recorded_digest, completion_action, completion_run,
                  completion_ticket, completion_digest, duplicate_action,
                  duplicate_run, duplicate_ticket, duplicate_digest,
                  duplicate_accepted>>

AdmitRunnable ==
  /\ admission = "pending"
  /\ evidence_present
  /\ evidence_passed
  /\ schema_compatible
  /\ decision = "accepted"
  /\ admission' = "runnable"
  /\ UNCHANGED <<decision, evidence_present, evidence_passed,
                  schema_compatible, scheduled, resolved, recorded_action,
                  recorded_run, recorded_ticket, recorded_digest,
                  completion_action, completion_run, completion_ticket, completion_digest,
                  duplicate_action, duplicate_run, duplicate_ticket, duplicate_digest,
                  duplicate_accepted>>

RejectAdmission ==
  /\ admission = "pending"
  /\ decision # "unknown"
  /\ ~(evidence_present /\ evidence_passed /\ schema_compatible /\ decision = "accepted")
  /\ admission' = "denied"
  /\ scheduled' = FALSE
  /\ UNCHANGED <<decision, evidence_present, evidence_passed,
                  schema_compatible, resolved, recorded_action, recorded_run,
                  recorded_ticket, recorded_digest, completion_action,
                  completion_run, completion_ticket, completion_digest,
                  duplicate_action, duplicate_run, duplicate_ticket, duplicate_digest,
                  duplicate_accepted>>

ScheduleAction ==
  /\ admission = "runnable"
  /\ decision = "accepted"
  /\ ~resolved
  /\ scheduled' = TRUE
  /\ UNCHANGED <<decision, evidence_present, evidence_passed,
                  schema_compatible, admission, resolved, recorded_action,
                  recorded_run, recorded_ticket, recorded_digest,
                  completion_action, completion_run, completion_ticket, completion_digest,
                  duplicate_action, duplicate_run, duplicate_ticket, duplicate_digest,
                  duplicate_accepted>>

CompleteAction ==
  /\ scheduled
  /\ resolved' = TRUE
  /\ scheduled' = FALSE
  /\ recorded_action' = completion_action
  /\ recorded_run' = completion_run
  /\ recorded_ticket' = completion_ticket
  /\ recorded_digest' = completion_digest
  /\ UNCHANGED <<decision, evidence_present, evidence_passed,
                  schema_compatible, admission, completion_action,
                  completion_run, completion_ticket, completion_digest,
                  duplicate_action, duplicate_run, duplicate_ticket,
                  duplicate_digest, duplicate_accepted>>

RetryAction ==
  /\ admission = "runnable"
  /\ decision = "accepted"
  /\ ~resolved
  /\ scheduled' = TRUE
  /\ UNCHANGED <<decision, evidence_present, evidence_passed,
                  schema_compatible, admission, resolved, recorded_action,
                  recorded_run, recorded_ticket, recorded_digest,
                  completion_action, completion_run, completion_ticket, completion_digest,
                  duplicate_action, duplicate_run, duplicate_ticket, duplicate_digest,
                  duplicate_accepted>>

ReplayJournal ==
  /\ resolved
  /\ scheduled' = FALSE
  /\ UNCHANGED <<decision, evidence_present, evidence_passed,
                  schema_compatible, admission, resolved, recorded_action,
                  recorded_run, recorded_ticket, recorded_digest,
                  completion_action, completion_run, completion_ticket, completion_digest,
                  duplicate_action, duplicate_run, duplicate_ticket, duplicate_digest,
                  duplicate_accepted>>

SameCompletion ==
  /\ recorded_action = duplicate_action
  /\ recorded_run = duplicate_run
  /\ recorded_ticket = duplicate_ticket
  /\ recorded_digest = duplicate_digest

SameActionTicketDifferentDigest ==
  /\ recorded_action = duplicate_action
  /\ recorded_ticket = duplicate_ticket
  /\ recorded_digest # duplicate_digest

DifferentTicketSameDigest ==
  /\ recorded_action = duplicate_action
  /\ recorded_ticket # duplicate_ticket
  /\ recorded_digest = duplicate_digest

DifferentTicketDifferentDigest ==
  /\ recorded_action = duplicate_action
  /\ recorded_ticket # duplicate_ticket
  /\ recorded_digest # duplicate_digest

DifferentActionOrRun ==
  \/ recorded_action # duplicate_action
  \/ recorded_run # duplicate_run

AcceptDuplicateCompletion ==
  /\ resolved
  /\ SameCompletion
  /\ duplicate_accepted' = TRUE
  /\ scheduled' = FALSE
  /\ UNCHANGED <<decision, evidence_present, evidence_passed,
                  schema_compatible, admission, resolved, recorded_action,
                  recorded_run, recorded_ticket, recorded_digest,
                  completion_action, completion_run, completion_ticket,
                  completion_digest, duplicate_action, duplicate_run,
                  duplicate_ticket, duplicate_digest>>

RejectDuplicateCompletion ==
  /\ resolved
  /\ ~SameCompletion
  /\ duplicate_accepted' = FALSE
  /\ scheduled' = FALSE
  /\ UNCHANGED <<decision, evidence_present, evidence_passed,
                  schema_compatible, admission, resolved, recorded_action,
                  recorded_run, recorded_ticket, recorded_digest,
                  completion_action, completion_run, completion_ticket,
                  completion_digest, duplicate_action, duplicate_run,
                  duplicate_ticket, duplicate_digest>>

SelectDuplicateAttempt ==
  /\ resolved
  /\ duplicate_action' \in Actions
  /\ duplicate_run' \in Runs
  /\ duplicate_ticket' \in Tickets
  /\ duplicate_digest' \in Digests
  /\ duplicate_accepted' = FALSE
  /\ UNCHANGED <<decision, evidence_present, evidence_passed,
                  schema_compatible, admission, scheduled, resolved,
                  recorded_action, recorded_run, recorded_ticket,
                  recorded_digest, completion_action, completion_run,
                  completion_ticket, completion_digest>>

Stutter == UNCHANGED vars

Next == ValidateAccept \/ ValidateReject \/ EmitPassingCertificate
        \/ EmitFailingCertificate \/ AdmitRunnable \/ RejectAdmission
        \/ ScheduleAction \/ CompleteAction \/ RetryAction \/ ReplayJournal
        \/ SelectDuplicateAttempt
        \/ AcceptDuplicateCompletion \/ RejectDuplicateCompletion \/ Stutter

Spec == Init /\ [][Next]_vars
        /\ WF_vars(EmitPassingCertificate)
        /\ WF_vars(EmitFailingCertificate)
        /\ WF_vars(RejectAdmission)
        /\ WF_vars(AdmitRunnable)
        /\ WF_vars(ReplayJournal)

NoRejectedEffectScheduled ==
  decision = "rejected" => scheduled = FALSE

CertificateSound ==
  evidence_passed => decision = "accepted"

AdmissionRequiresEvidence ==
  admission = "runnable" => evidence_present /\ evidence_passed /\ schema_compatible

AdmissionRequiresPassedIdempotencyEvidence ==
  admission = "runnable" => evidence_present /\ evidence_passed /\ schema_compatible /\ decision = "accepted"

ResolvedActionMonotonic ==
  resolved => scheduled = FALSE

DuplicateCompletionSameDigestOnly ==
  duplicate_accepted => SameCompletion

ConflictingDuplicateRejected ==
  (SameActionTicketDifferentDigest \/ DifferentTicketSameDigest
   \/ DifferentTicketDifferentDigest \/ DifferentActionOrRun) => ~duplicate_accepted

EventuallyAdmittedOrRejected ==
  [](decision # "unknown" /\ evidence_present => <>(admission = "runnable" \/ admission = "denied"))

EventuallyReplaySettles ==
  [](resolved => <>~scheduled)

====
