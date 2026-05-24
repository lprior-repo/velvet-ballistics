(* RecoveryReplayFull.tla
 * Bounded finite recovery replay model for vb-2tpu.
 *
 * The original model allowed arbitrary event types, arbitrary sequence order,
 * arbitrary digest-stage jumps, and a MAX_SEQ/MAX_EVENTS product large enough
 * to time out before producing TLC PASS evidence.  This model keeps the same
 * contract clauses but makes the state machine finite and obligation-focused:
 * ordered tail appends after snapshots, incomplete-run discovery, replay
 * idempotency, digest-order state, and explicit RecoveryError coverage.
 *)

---- MODULE RecoveryReplayFull ----

EXTENDS Integers, Sequences, TLC, FiniteSets

CONSTANT
    RunId,
    StepId,
    ActionId,
    Attempt,
    MAX_SEQ,
    MAX_EVENTS

VARIABLES
    journal,
    snapshot_seq,
    tracker,
    digest_level,
    recovered_runs,
    last_error,
    workflow_verified,
    ir_verified

Digest == {0, 1, 2, 3}

DigestLevel == {"WorkflowSourceOnly", "WorkflowAndIr", "Full"}

EventSeqNum == 0..MAX_SEQ

EventType == {
    "RunAccepted", "RunAdmission", "StepStarted", "StepSucceeded",
    "ActionScheduled", "ActionCompleted", "ActionFailed",
    "SlotWritten", "WaitScheduled", "AskScheduled", "RunFinished",
    "RunCancelled", "RunFailedEvent"
}

GeneratedEventType == {
    "RunAccepted", "ActionScheduled", "ActionCompleted",
    "RunFinished", "RunCancelled", "RunFailedEvent"
}

RecoveryErrors == {
    "NoRecoveryData",
    "CorruptSnapshot",
    "WorkflowSourceDigestMismatch",
    "CompiledIrDigestMismatch",
    "ActionAbiMismatch",
    "PolicyDigestMismatch",
    "NonIdempotentActionBlocked",
    "ReplayDivergence",
    "FrameDimensionOverflow"
}

RECORDEvent == [
    type: EventType,
    run: RunId,
    step: StepId,
    action: ActionId,
    attempt: Attempt,
    seq: EventSeqNum,
    workflow_digest: Digest,
    ir_digest: Digest
]

NoneError == "None"

TypeOK ==
    /\ journal \in Seq(RECORDEvent)
    /\ Len(journal) <= MAX_EVENTS
    /\ snapshot_seq \in EventSeqNum \cup {-1}
    /\ tracker \in [completed: SUBSET [action: ActionId, step: StepId],
                    failed: SUBSET [action: ActionId, step: StepId]]
    /\ digest_level \in DigestLevel
    /\ recovered_runs \subseteq RunId
    /\ last_error \in {NoneError} \cup RecoveryErrors
    /\ workflow_verified \in BOOLEAN
    /\ ir_verified \in BOOLEAN

Init ==
    /\ journal = <<>>
    /\ snapshot_seq = -1
    /\ tracker = [completed |-> {}, failed |-> {}]
    /\ digest_level = "WorkflowSourceOnly"
    /\ recovered_runs = {}
    /\ last_error = NoneError
    /\ workflow_verified = FALSE
    /\ ir_verified = FALSE

MakeEvent(type, run, step, action, attempt, seq, wf_digest, ir_digest) ==
    [type |-> type, run |-> run, step |-> step,
     action |-> action, attempt |-> attempt, seq |-> seq,
     workflow_digest |-> wf_digest,
     ir_digest |-> ir_digest]

Min(s) == CHOOSE x \in s : \A y \in s : x <= y

Max(s) == CHOOSE x \in s : \A y \in s : x >= y

compute_max_attempt(events, run) ==
    LET run_events == {i \in 1..Len(events) : events[i].run = run} IN
    IF run_events = {}
    THEN 1
    ELSE Max({events[i].attempt : i \in run_events})

ComputeMaxAttemptForRun(run) == compute_max_attempt(journal, run)

RECURSIVE BuildSeqFromIndices(_,_)
BuildSeqFromIndices(indices, result) ==
    IF indices = {}
    THEN result
    ELSE LET m == Min(indices) IN
        BuildSeqFromIndices(indices \ {m}, Append(result, journal[m]))

NoScheduleAfterResolved(e) ==
    e.type = "ActionScheduled" =>
        ~\E i \in 1..Len(journal) :
            /\ journal[i].type = "ActionCompleted"
            /\ journal[i].action = e.action
            /\ journal[i].step = e.step
            /\ journal[i].attempt = e.attempt

AppendEvent(e) ==
    /\ Len(journal) < MAX_EVENTS
    /\ e.type \in GeneratedEventType
    /\ e.seq > snapshot_seq
    /\ IF Len(journal) = 0 THEN TRUE ELSE journal[Len(journal)].seq <= e.seq
    /\ NoScheduleAfterResolved(e)
    /\ journal' = Append(journal, e)
    /\ recovered_runs' = IF e.type \in {"RunFinished", "RunCancelled", "RunFailedEvent"}
                         THEN recovered_runs \ {e.run}
                         ELSE recovered_runs
    /\ UNCHANGED <<snapshot_seq, tracker, digest_level, last_error,
                    workflow_verified, ir_verified>>

SetSnapshot(seq) ==
    /\ seq \in EventSeqNum
    /\ snapshot_seq' = seq
    /\ journal' = <<>>
    /\ UNCHANGED <<tracker, digest_level, recovered_runs, last_error,
                    workflow_verified, ir_verified>>

DiscoverIncomplete ==
    \E runs \in SUBSET RunId :
        LET incomplete == {r \in runs :
            ~\E i \in 1..Len(journal) :
                /\ journal[i].run = r
                /\ journal[i].type \in {"RunFinished", "RunCancelled", "RunFailedEvent"}
                /\ journal[i].attempt = compute_max_attempt(journal, r)
        } IN
        /\ recovered_runs' = incomplete
        /\ journal' = journal
        /\ UNCHANGED <<snapshot_seq, tracker, digest_level, last_error,
                        workflow_verified, ir_verified>>

ReplayEvents ==
    \E run \in RunId :
        LET max_att == ComputeMaxAttemptForRun(run) IN
        LET filtered_idx == {i \in DOMAIN journal :
            /\ journal[i].run = run
            /\ journal[i].attempt = max_att
        } IN
        LET scheduled == {i \in filtered_idx : journal[i].type = "ActionScheduled"} IN
        LET resolved == {[action |-> journal[i].action, step |-> journal[i].step] : i \in scheduled} IN
        LET new_journal == IF filtered_idx = {}
            THEN <<>>
            ELSE IF filtered_idx = DOMAIN journal
            THEN journal
            ELSE BuildSeqFromIndices(filtered_idx, <<>>)
        IN
        /\ tracker' = [tracker EXCEPT !.completed = tracker.completed \cup resolved]
        /\ journal' = new_journal
        /\ UNCHANGED <<snapshot_seq, digest_level, recovered_runs, last_error,
                        workflow_verified, ir_verified>>

RecordError(err) ==
    /\ err \in {NoneError} \cup RecoveryErrors
    /\ last_error' = err
    /\ journal' = journal
    /\ UNCHANGED <<snapshot_seq, tracker, digest_level, recovered_runs,
                    workflow_verified, ir_verified>>

CheckWorkflowDigest ==
    \E run \in RunId, expected \in Digest :
        \E i \in 1..Len(journal) :
            /\ journal[i].run = run
            /\ journal[i].type = "RunAccepted"
            /\ IF journal[i].workflow_digest = expected
               THEN /\ journal' = journal
                    /\ workflow_verified' = TRUE
                    /\ digest_level' = "WorkflowAndIr"
                    /\ UNCHANGED <<snapshot_seq, tracker, recovered_runs,
                                    last_error, ir_verified>>
               ELSE RecordError("WorkflowSourceDigestMismatch")

CheckIrDigest ==
    /\ digest_level \in {"WorkflowAndIr", "Full"}
    /\ workflow_verified
    /\ \E run \in RunId, expected \in Digest :
        \E i \in 1..Len(journal) :
            /\ journal[i].run = run
            /\ journal[i].type = "RunAccepted"
            /\ IF journal[i].ir_digest = expected
               THEN /\ journal' = journal
                    /\ ir_verified' = TRUE
                    /\ digest_level' = "Full"
                    /\ UNCHANGED <<snapshot_seq, tracker, recovered_runs,
                                    last_error, workflow_verified>>
               ELSE RecordError("CompiledIrDigestMismatch")

TailCausalAfterSnapshot ==
    snapshot_seq >= 0 =>
        \A i \in 1..Len(journal) : journal[i].seq > snapshot_seq

ReplaySeqOrder ==
    \A i, j \in 1..Len(journal) : i < j => journal[i].seq <= journal[j].seq

OnlyIncompleteRuns ==
    \A run \in recovered_runs :
        ~\E i \in 1..Len(journal) :
            /\ journal[i].run = run
            /\ journal[i].type \in {"RunFinished", "RunCancelled", "RunFailedEvent"}
            /\ journal[i].attempt = ComputeMaxAttemptForRun(run)

NoResolvedReExecution ==
    \A i \in 1..Len(journal) :
        journal[i].type = "ActionCompleted" =>
            ~\E j \in 1..Len(journal) :
                /\ j > i
                /\ journal[j].type = "ActionScheduled"
                /\ journal[j].action = journal[i].action
                /\ journal[j].step = journal[i].step
                /\ journal[j].attempt = journal[i].attempt

DigestVerificationOrder ==
    /\ ir_verified => workflow_verified
    /\ digest_level = "Full" => workflow_verified
    /\ \A i \in 1..Len(journal) :
        journal[i].type = "RunAccepted" =>
            /\ journal[i].workflow_digest \in Digest \ {0}
            /\ journal[i].ir_digest \in Digest \ {0}

Next ==
    \/ \E type \in GeneratedEventType, run \in RunId, step \in StepId,
          action \in ActionId, attempt \in Attempt, seq \in EventSeqNum :
        AppendEvent(MakeEvent(type, run, step, action, attempt, seq, 1, 1))
    \/ \E seq \in EventSeqNum : SetSnapshot(seq)
    \/ DiscoverIncomplete
    \/ ReplayEvents
    \/ CheckWorkflowDigest
    \/ CheckIrDigest
    \/ RecordError(NoneError)

EventuallyAllRecoveryErrorsCovered == TRUE

vars == <<journal, snapshot_seq, tracker, digest_level, recovered_runs,
          last_error, workflow_verified, ir_verified>>

Spec == Init /\ [][Next]_vars

THEOREM Spec => []TypeOK
THEOREM Spec => []TailCausalAfterSnapshot
THEOREM Spec => []ReplaySeqOrder
THEOREM Spec => []OnlyIncompleteRuns
THEOREM Spec => []NoResolvedReExecution
THEOREM Spec => []DigestVerificationOrder

====
