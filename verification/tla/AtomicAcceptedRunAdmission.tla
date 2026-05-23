---- MODULE AtomicAcceptedRunAdmission ----

\* Obligations: TLA-ATOM-001.
\* Finite temporal model for strict accepted-run admission.  The model treats
\* Fjall OwnedWriteBatch commit as the single durable atomic boundary: records
\* staged before commit are not durable and are erased by injected failures.

CONSTANTS Runs, Workflows

RecordKinds == {"source", "artifact", "header", "run_accepted", "status_index", "workflow_index", "action_index"}
Bool == {TRUE, FALSE}

VARIABLES run, workflow, staged, durable, commit_state, ack, allocated, failed,
          restarted, readback_decision

vars == <<run, workflow, staged, durable, commit_state, ack, allocated, failed,
          restarted, readback_decision>>

TypeOK ==
  /\ run \in Runs
  /\ workflow \in Workflows
  /\ staged \subseteq RecordKinds
  /\ durable \subseteq RecordKinds
  /\ commit_state \in {"pending", "committed", "failed"}
  /\ ack \in BOOLEAN
  /\ allocated \in BOOLEAN
  /\ failed \in BOOLEAN
  /\ restarted \in BOOLEAN
  /\ readback_decision \in {"none", "accepted", "absent"}

Init ==
  /\ run \in Runs
  /\ workflow \in Workflows
  /\ staged = {}
  /\ durable = {}
  /\ commit_state = "pending"
  /\ ack = FALSE
  /\ allocated = FALSE
  /\ failed = FALSE
  /\ restarted = FALSE
  /\ readback_decision = "none"

CanStage == commit_state = "pending" /\ ~failed

StageRecord ==
  /\ CanStage
  /\ staged # RecordKinds
  /\ \E record \in RecordKinds \ staged:
        staged' = staged \cup {record}
  /\ UNCHANGED <<run, workflow, durable, commit_state, ack, allocated, failed,
                  restarted, readback_decision>>

Commit ==
  /\ CanStage
  /\ staged = RecordKinds
  /\ durable' = RecordKinds
  /\ commit_state' = "committed"
  /\ UNCHANGED <<run, workflow, staged, ack, allocated, failed,
                  restarted, readback_decision>>

FailBeforeOrDuringCommit ==
  /\ commit_state = "pending"
  /\ ~failed
  /\ failed' = TRUE
  /\ staged' = {}
  /\ durable' = {}
  /\ commit_state' = "failed"
  /\ ack' = FALSE
  /\ allocated' = FALSE
  /\ readback_decision' = "none"
  /\ restarted' = restarted
  /\ UNCHANGED <<run, workflow>>

Acknowledge ==
  /\ commit_state = "committed"
  /\ durable = RecordKinds
  /\ ~ack
  /\ ack' = TRUE
  /\ UNCHANGED <<run, workflow, staged, durable, commit_state, allocated, failed,
                  restarted, readback_decision>>

AllocateRuntime ==
  /\ commit_state = "committed"
  /\ durable = RecordKinds
  /\ ~allocated
  /\ allocated' = TRUE
  /\ UNCHANGED <<run, workflow, staged, durable, commit_state, ack, failed,
                  restarted, readback_decision>>

Restart ==
  /\ ~restarted
  /\ restarted' = TRUE
  /\ IF commit_state = "committed" /\ durable = RecordKinds
        THEN
          /\ readback_decision' = "accepted"
          /\ UNCHANGED <<staged, durable, commit_state, ack, allocated, failed>>
        ELSE
          /\ staged' = {}
          /\ durable' = {}
          /\ commit_state' = "failed"
          /\ ack' = FALSE
          /\ allocated' = FALSE
          /\ failed' = TRUE
          /\ readback_decision' = "absent"
  /\ UNCHANGED <<run, workflow>>

ReadbackAccepted ==
  /\ durable = RecordKinds
  /\ readback_decision' = "accepted"
  /\ UNCHANGED <<run, workflow, staged, durable, commit_state, ack, allocated,
                  failed, restarted>>

ReadbackAbsent ==
  /\ durable # RecordKinds
  /\ readback_decision' = "absent"
  /\ UNCHANGED <<run, workflow, staged, durable, commit_state, ack, allocated,
                  failed, restarted>>

Stutter == UNCHANGED vars

Next == StageRecord \/ Commit \/ FailBeforeOrDuringCommit \/ Acknowledge
        \/ AllocateRuntime \/ Restart \/ ReadbackAccepted \/ ReadbackAbsent
        \/ Stutter

CommitOrFail == Commit \/ FailBeforeOrDuringCommit

Spec ==
  /\ Init
  /\ [][Next]_vars
  /\ WF_vars(StageRecord)
  /\ WF_vars(CommitOrFail)
  /\ WF_vars(Acknowledge)
  /\ WF_vars(Restart)
  /\ WF_vars(ReadbackAccepted)

AllRecordsOrNoAcceptedRun ==
  readback_decision = "accepted" => durable = RecordKinds

NoPartialAfterFailure ==
  failed => durable = {} /\ staged = {} /\ ~ack /\ ~allocated

IndexesOnlyCommitted ==
  ("status_index" \in durable \/ "workflow_index" \in durable \/ "action_index" \in durable)
    => durable = RecordKinds /\ commit_state = "committed"

ReadbackOnlyCommitted ==
  readback_decision = "accepted" => commit_state = "committed" /\ durable = RecordKinds

RestartReadbackDeterministic ==
  restarted =>
    IF durable = RecordKinds
      THEN commit_state = "committed" /\ readback_decision = "accepted"
      ELSE durable = {} /\ commit_state = "failed" /\ readback_decision = "absent"

NoAckBeforeCommit ==
  ack => commit_state = "committed" /\ durable = RecordKinds

NoRuntimeAllocationBeforeCommit ==
  allocated => commit_state = "committed" /\ durable = RecordKinds

EventuallyAckOrFail == <>(ack \/ failed)

EventuallyReadableAfterCommit == [](commit_state = "committed" => <>(readback_decision = "accepted"))

EventuallyRestartReadbackAfterCommit ==
  [](commit_state = "committed" => <>(restarted /\ readback_decision = "accepted"))

====
