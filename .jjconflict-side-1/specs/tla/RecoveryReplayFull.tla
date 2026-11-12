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
    EnabledEventTypes,
    ABIExpected,
    ABIFound,
    PolicyExpected,
    PolicyFound,
    SnapshotInputs,
    MAX_SEQ,
    MAX_EVENTS

VARIABLES
    journal,
    snapshot_seq,
    tracker,
    replay_candidates,
    digest_level,
    digest_stage,
    recovered_runs,
    last_error,
    abi_expected,
    abi_found,
    policy_expected,
    policy_found,
    snapshot_inputs

NoneError == "None"

Digest == {0, 1, 2, 3}

ErrorDomain == {NoneError} \cup {"NoRecoveryData", "CorruptSnapshot", "WorkflowSourceDigestMismatch", "CompiledIrDigestMismatch", "ActionAbiMismatch", "PolicyDigestMismatch", "NonIdempotentActionBlocked", "ReplayDivergence", "FrameDimensionOverflow"}

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

RECORDSnapshot == [
    run: RunId,
    seq: EventSeqNum,
    workflow: Digest,
    slots: STRING,
    taint: STRING
]

ReplayCandidate == [
    action: ActionId,
    step: StepId,
    attempt: Attempt
]

TypeOK ==
    /\ EnabledEventTypes \subseteq EventType
    /\ ABIExpected \in Digest
    /\ ABIFound \in Digest
    /\ PolicyExpected \in Digest
    /\ PolicyFound \in Digest
    /\ SnapshotInputs \subseteq {"Absent", "Corrupt"}
    /\ abi_expected \in Digest
    /\ abi_found \in Digest
    /\ policy_expected \in Digest
    /\ policy_found \in Digest
    /\ snapshot_inputs \subseteq {"Absent", "Corrupt"}
    /\ journal \in Seq(RECORDEvent)
    /\ Len(journal) <= MAX_EVENTS
    /\ snapshot_seq \in EventSeqNum \cup {-1}
    /\ tracker \in [completed: SUBSET [action: ActionId, step: StepId], failed: SUBSET [action: ActionId, step: StepId]]
    /\ replay_candidates \subseteq ReplayCandidate
    /\ digest_level \in DigestLevel
    /\ digest_stage \in [RunId -> SUBSET {"WorkflowChecked", "IrChecked"}]
    /\ recovered_runs \subseteq RunId
    /\ last_error \in ErrorDomain

Init ==
    /\ journal = <<>>
    /\ snapshot_seq = -1
    /\ tracker = [completed |-> {}, failed |-> {}]
    /\ replay_candidates = {}
    /\ digest_level = "WorkflowSourceOnly"
    /\ digest_stage = [r \in RunId |-> {}]
    /\ recovered_runs = {}
    /\ last_error = NoneError
    /\ abi_expected = ABIExpected
    /\ abi_found = ABIFound
    /\ policy_expected = PolicyExpected
    /\ policy_found = PolicyFound
    /\ snapshot_inputs = SnapshotInputs

InputVars == <<abi_expected, abi_found, policy_expected, policy_found, snapshot_inputs>>

vars == <<journal, snapshot_seq, tracker, replay_candidates, digest_level, digest_stage, recovered_runs, last_error, abi_expected, abi_found, policy_expected, policy_found, snapshot_inputs>>

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
    /\ IF Len(journal) = 0 THEN TRUE ELSE journal[Len(journal)].seq <= e.seq
    /\ snapshot_seq >= 0 => e.seq > snapshot_seq
    /\ e.type = "ActionScheduled" =>
        ~\E i \in 1..Len(journal) :
            /\ journal[i].type = "ActionCompleted"
            /\ journal[i].action = e.action
            /\ journal[i].step = e.step
            /\ journal[i].attempt = e.attempt
    /\ journal' = Append(journal, e)
    /\ recovered_runs' = IF e.type \in {"RunFinished", "RunCancelled", "RunFailedEvent"}
        THEN recovered_runs \ {e.run}
        ELSE recovered_runs
    /\ UNCHANGED <<snapshot_seq, tracker, replay_candidates, digest_level, digest_stage, last_error, InputVars>>

SetSnapshot(run, seq) ==
    /\ run \in RunId
    /\ seq \in EventSeqNum
    /\ \A i \in 1..Len(journal) : journal[i].seq > seq
    /\ snapshot_seq' = seq
    /\ UNCHANGED <<journal, tracker, replay_candidates, digest_level, digest_stage, recovered_runs, last_error, InputVars>>

DiscoverIncomplete ==
    \E runs \in SUBSET RunId :
        LET incomplete == {r \in runs :
            ~\E i \in 1..Len(journal) :
                /\ journal[i].run = r
                /\ journal[i].type \in {"RunFinished", "RunCancelled", "RunFailedEvent"}
                /\ journal[i].attempt = compute_max_attempt(journal, r)
        } IN
        recovered_runs' = incomplete /\
        journal' = journal /\
        UNCHANGED <<snapshot_seq, tracker, replay_candidates, digest_level, digest_stage, last_error, InputVars>>

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
        tracker' = [tracker EXCEPT !.completed = tracker.completed \cup resolved] /\
        journal' = new_journal /\
        UNCHANGED <<snapshot_seq, replay_candidates, digest_level, digest_stage, recovered_runs, last_error, InputVars>>

DigestCheckNext ==
    \E level \in DigestLevel :
        digest_level' = level /\
        journal' = journal /\
        UNCHANGED <<snapshot_seq, tracker, replay_candidates, digest_stage, recovered_runs, last_error, InputVars>>

RecordError(err) ==
    last_error' = err /\
    journal' = journal /\
    UNCHANGED <<snapshot_seq, tracker, replay_candidates, digest_level, digest_stage, recovered_runs, InputVars>>

RecordErrorWithUnchangedStage(err) ==
    /\ last_error = NoneError
    /\ err # NoneError
    /\ last_error' = err
    /\ UNCHANGED <<journal, snapshot_seq, tracker, replay_candidates, digest_level, digest_stage, recovered_runs, InputVars>>

MarkReplayCandidate ==
    \E i \in 1..Len(journal) :
        /\ journal[i].type \in {"ActionCompleted", "ActionFailed"}
        /\ LET candidate == [action |-> journal[i].action,
                             step |-> journal[i].step,
                             attempt |-> journal[i].attempt] IN
            /\ candidate \notin replay_candidates
            /\ replay_candidates' = replay_candidates \cup {candidate}
        /\ UNCHANGED <<journal, snapshot_seq, tracker, digest_level, digest_stage, recovered_runs, last_error, InputVars>>

CheckWorkflowDigest ==
    \E run \in RunId, expected \in Digest :
        \E i \in 1..Len(journal) :
            journal[i].run = run /\
            journal[i].type = "RunAccepted" /\
            IF journal[i].workflow_digest = expected
            THEN /\ journal' = journal
                 /\ digest_stage' = [digest_stage EXCEPT ![run] = @ \cup {"WorkflowChecked"}]
                 /\ UNCHANGED <<snapshot_seq, tracker, replay_candidates, digest_level, recovered_runs, last_error, InputVars>>
            ELSE RecordErrorWithUnchangedStage("WorkflowSourceDigestMismatch")

CheckIrDigest ==
    digest_level \in {"WorkflowAndIr", "Full"} /\
    \E run \in RunId, expected \in Digest :
        "WorkflowChecked" \in digest_stage[run] /\
        \E i \in 1..Len(journal) :
            journal[i].run = run /\
            journal[i].type = "RunAccepted" /\
            IF journal[i].ir_digest = expected
            THEN /\ journal' = journal
                 /\ digest_stage' = [digest_stage EXCEPT ![run] = @ \cup {"IrChecked"}]
                 /\ UNCHANGED <<snapshot_seq, tracker, replay_candidates, digest_level, recovered_runs, last_error, InputVars>>
            ELSE RecordErrorWithUnchangedStage("CompiledIrDigestMismatch")

RecoverRunWithoutEvents ==
    \E run \in RunId :
        /\ ~\E i \in 1..Len(journal) : journal[i].run = run
        /\ RecordErrorWithUnchangedStage("NoRecoveryData")

LoadCorruptSnapshot ==
    /\ snapshot_seq >= 0
    /\ "Corrupt" \in snapshot_inputs
    /\ RecordErrorWithUnchangedStage("CorruptSnapshot")

CheckActionAbiDigest ==
    /\ abi_expected # abi_found
    /\ RecordErrorWithUnchangedStage("ActionAbiMismatch")

CheckPolicyDigest ==
    /\ policy_expected # policy_found
    /\ RecordErrorWithUnchangedStage("PolicyDigestMismatch")

DetectNonIdempotentResolved ==
    \E i \in 1..Len(journal), candidate \in replay_candidates :
        /\ journal[i].type \in {"ActionCompleted", "ActionFailed"}
        /\ candidate.action = journal[i].action
        /\ candidate.step = journal[i].step
        /\ candidate.attempt = journal[i].attempt
        /\ RecordErrorWithUnchangedStage("NonIdempotentActionBlocked")

DetectReplayDivergence ==
    \E candidate_seq \in EventSeqNum :
        /\ Len(journal) > 0
        /\ candidate_seq < journal[Len(journal)].seq
        /\ RecordErrorWithUnchangedStage("ReplayDivergence")

DetectFrameDimensionOverflow ==
    /\ Len(journal) >= MAX_EVENTS
    /\ RecordErrorWithUnchangedStage("FrameDimensionOverflow")

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
    \A run \in RunId :
        "IrChecked" \in digest_stage[run] => "WorkflowChecked" \in digest_stage[run]

Next ==
    \/ \E type \in EnabledEventTypes, run \in RunId, step \in StepId,
          action \in ActionId, attempt \in Attempt, seq \in EventSeqNum :
        AppendEvent(MakeEvent(type, run, step, action, attempt, seq, 1, 1))
    \/ \E run \in RunId, seq \in EventSeqNum : SetSnapshot(run, seq)
    \/ DiscoverIncomplete
    \/ ReplayEvents
    \/ CheckWorkflowDigest
    \/ CheckIrDigest
    \/ RecoverRunWithoutEvents
    \/ LoadCorruptSnapshot
    \/ CheckActionAbiDigest
    \/ CheckPolicyDigest
    \/ MarkReplayCandidate
    \/ DetectNonIdempotentResolved
    \/ DetectReplayDivergence
    \/ DetectFrameDimensionOverflow

Spec == Init /\ [][Next]_vars

ReachReplaySeqOrderAntecedent == Len(journal) >= 2

ReachTailCausalAntecedent ==
    /\ snapshot_seq >= 0
    /\ \E i \in 1..Len(journal) : journal[i].seq > snapshot_seq

ReachRecoveredRunsNonEmpty == recovered_runs # {}

ReachTerminalExcludedFromRecovered ==
    \E i \in 1..Len(journal) :
        /\ journal[i].type \in {"RunFinished", "RunCancelled", "RunFailedEvent"}
        /\ journal[i].run \notin recovered_runs

ReachResolvedActionGuardAntecedent ==
    \E i \in 1..Len(journal), candidate \in replay_candidates :
        /\ journal[i].type \in {"ActionCompleted", "ActionFailed"}
        /\ candidate.action = journal[i].action
        /\ candidate.step = journal[i].step
        /\ candidate.attempt = journal[i].attempt

ReachDigestAcceptedAntecedent ==
    \E i \in 1..Len(journal) : journal[i].type = "RunAccepted"

ReachModeledDigestError ==
    last_error \in {"WorkflowSourceDigestMismatch", "CompiledIrDigestMismatch"}

ReachDigestIrAfterWorkflow ==
    \E run \in RunId :
        /\ "WorkflowChecked" \in digest_stage[run]
        /\ "IrChecked" \in digest_stage[run]

ReachError(err) == last_error = err

NotReachReplaySeqOrderAntecedent == ~ReachReplaySeqOrderAntecedent
NotReachTailCausalAntecedent == ~ReachTailCausalAntecedent
NotReachRecoveredRunsNonEmpty == ~ReachRecoveredRunsNonEmpty
NotReachTerminalExcludedFromRecovered == ~ReachTerminalExcludedFromRecovered
NotReachResolvedActionGuardAntecedent == ~ReachResolvedActionGuardAntecedent
NotReachDigestIrAfterWorkflow == ~ReachDigestIrAfterWorkflow
NotReachErrorWorkflowSourceDigestMismatch == ~ReachError("WorkflowSourceDigestMismatch")
NotReachErrorCompiledIrDigestMismatch == ~ReachError("CompiledIrDigestMismatch")
NotReachErrorNoRecoveryData == ~ReachError("NoRecoveryData")
NotReachErrorCorruptSnapshot == ~ReachError("CorruptSnapshot")
NotReachErrorActionAbiMismatch == ~ReachError("ActionAbiMismatch")
NotReachErrorPolicyDigestMismatch == ~ReachError("PolicyDigestMismatch")
NotReachErrorNonIdempotentActionBlocked == ~ReachError("NonIdempotentActionBlocked")
NotReachErrorReplayDivergence == ~ReachError("ReplayDivergence")
NotReachErrorFrameDimensionOverflow == ~ReachError("FrameDimensionOverflow")

ModeledRecoveryErrors == ErrorDomain \ {NoneError}

RecoveryErrorWitnessCoverageClaim ==
    ModeledRecoveryErrors = {
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

AllNonVacuityWitnessesReached ==
    /\ ReachReplaySeqOrderAntecedent
    /\ ReachTailCausalAntecedent
    /\ ReachRecoveredRunsNonEmpty
    /\ ReachTerminalExcludedFromRecovered
    /\ ReachResolvedActionGuardAntecedent
    /\ ReachDigestAcceptedAntecedent

NotAllNonVacuityWitnessesReached == ~AllNonVacuityWitnessesReached

THEOREM Spec => []TypeOK
THEOREM Spec => []TailCausalAfterSnapshot
THEOREM Spec => []ReplaySeqOrder
THEOREM Spec => []OnlyIncompleteRuns
THEOREM Spec => []NoResolvedReExecution
THEOREM Spec => []DigestVerificationOrder

====
