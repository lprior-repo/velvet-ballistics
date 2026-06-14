---- MODULE ResumeStateMachine ----

EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS RunIds, MaxJournalLength, MaxOpLogLength

VARIABLES runtimeState, journal, opLog, lastOutcome, driveFailures

vars == <<runtimeState, journal, opLog, lastOutcome, driveFailures>>

States == {"Initial", "Running", "Resumable", "Resuming", "Failed"}
JournalKinds == {"Started", "Resumed", "Failed"}
RuntimeEventKinds == {"Submit", "Resume", "ResumeRollback", "DriveContinue", "Fail"}
Outcomes == {"None", "AlreadyRunning", "NotResumable", "AppendFailed", "ResumedOk", "DriveFailed"}
OpKinds == {"Journal", "RuntimeEvent"}
OpEvents == JournalKinds \cup RuntimeEventKinds

JournalEvent == [kind: JournalKinds, run: RunIds]
OpEvent == [kind: OpKinds, event: OpEvents, run: RunIds]

TypeOK ==
    /\ runtimeState \in [RunIds -> States]
    /\ journal \in Seq(JournalEvent)
    /\ opLog \in Seq(OpEvent)
    /\ lastOutcome \in [RunIds -> Outcomes]
    /\ driveFailures \subseteq RunIds
    /\ Len(journal) <= MaxJournalLength
    /\ Len(opLog) <= MaxOpLogLength

Init ==
    /\ runtimeState = [r \in RunIds |-> "Initial"]
    /\ journal = <<>>
    /\ opLog = <<>>
    /\ lastOutcome = [r \in RunIds |-> "None"]
    /\ driveFailures = {}

CanAppendJournal == Len(journal) < MaxJournalLength
CanAppendOp(n) == Len(opLog) + n <= MaxOpLogLength

AppendJournal(kind, r) == Append(journal, [kind |-> kind, run |-> r])
AppendOp(kind, event, r) == Append(opLog, [kind |-> kind, event |-> event, run |-> r])

HasJournal(kind, r) ==
    \E i \in DOMAIN journal :
        /\ journal[i].kind = kind
        /\ journal[i].run = r

HasResumedJournal(r) == HasJournal("Resumed", r)

StartRun(r) ==
    /\ CanAppendJournal
    /\ CanAppendOp(2)
    /\ runtimeState[r] = "Initial"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Running"]
    /\ journal' = AppendJournal("Started", r)
    /\ opLog' = AppendOp("Journal", "Started", r)
    /\ lastOutcome' = [lastOutcome EXCEPT ![r] = "None"]
    /\ UNCHANGED driveFailures

Suspend(r) ==
    /\ runtimeState[r] = "Running"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Resumable"]
    /\ lastOutcome' = [lastOutcome EXCEPT ![r] = "None"]
    /\ UNCHANGED <<journal, opLog, driveFailures>>

ResumeAlreadyRunning(r) ==
    /\ runtimeState[r] = "Running"
    /\ lastOutcome' = [lastOutcome EXCEPT ![r] = "AlreadyRunning"]
    /\ UNCHANGED <<runtimeState, journal, opLog, driveFailures>>

ResumeNotResumable(r) ==
    /\ runtimeState[r] \in {"Initial", "Resuming", "Failed"}
    /\ lastOutcome' = [lastOutcome EXCEPT ![r] = "NotResumable"]
    /\ UNCHANGED <<runtimeState, journal, opLog, driveFailures>>

AppendResumedEventOk(r) ==
    /\ CanAppendJournal
    /\ CanAppendOp(2)
    /\ runtimeState[r] = "Resumable"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Resuming"]
    /\ journal' = AppendJournal("Resumed", r)
    /\ opLog' = Append(AppendOp("RuntimeEvent", "Resume", r),
                       [kind |-> "Journal", event |-> "Resumed", run |-> r])
    /\ lastOutcome' = [lastOutcome EXCEPT ![r] = "None"]
    /\ UNCHANGED driveFailures

AppendResumedEventFail(r) ==
    /\ CanAppendOp(2)
    /\ runtimeState[r] = "Resumable"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Resumable"]
    /\ opLog' = Append(AppendOp("RuntimeEvent", "Resume", r),
                       [kind |-> "RuntimeEvent", event |-> "ResumeRollback", run |-> r])
    /\ lastOutcome' = [lastOutcome EXCEPT ![r] = "AppendFailed"]
    /\ UNCHANGED <<journal, driveFailures>>

ResumeDriveContinue(r) ==
    /\ CanAppendOp(1)
    /\ runtimeState[r] = "Resuming"
    /\ HasResumedJournal(r)
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Running"]
    /\ opLog' = AppendOp("RuntimeEvent", "DriveContinue", r)
    /\ lastOutcome' = [lastOutcome EXCEPT ![r] = "ResumedOk"]
    /\ UNCHANGED <<journal, driveFailures>>

ResumeDriveFailureRollback(r) ==
    /\ CanAppendOp(1)
    /\ runtimeState[r] = "Resuming"
    /\ HasResumedJournal(r)
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Resumable"]
    /\ opLog' = AppendOp("RuntimeEvent", "ResumeRollback", r)
    /\ lastOutcome' = [lastOutcome EXCEPT ![r] = "DriveFailed"]
    /\ driveFailures' = driveFailures \cup {r}
    /\ UNCHANGED journal

FailRun(r) ==
    /\ CanAppendJournal
    /\ CanAppendOp(2)
    /\ runtimeState[r] = "Running"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Failed"]
    /\ journal' = AppendJournal("Failed", r)
    /\ opLog' = Append(AppendOp("RuntimeEvent", "Fail", r),
                       [kind |-> "Journal", event |-> "Failed", run |-> r])
    /\ lastOutcome' = [lastOutcome EXCEPT ![r] = "None"]
    /\ UNCHANGED driveFailures

Next ==
    \/ \E r \in RunIds : StartRun(r)
    \/ \E r \in RunIds : Suspend(r)
    \/ \E r \in RunIds : ResumeAlreadyRunning(r)
    \/ \E r \in RunIds : ResumeNotResumable(r)
    \/ \E r \in RunIds : AppendResumedEventOk(r)
    \/ \E r \in RunIds : AppendResumedEventFail(r)
    \/ \E r \in RunIds : ResumeDriveContinue(r)
    \/ \E r \in RunIds : ResumeDriveFailureRollback(r)
    \/ \E r \in RunIds : FailRun(r)
    \/ /\ \/ Len(journal) = MaxJournalLength
          \/ Len(opLog) >= MaxOpLogLength - 1
          \/ \A r \in RunIds : runtimeState[r] = "Failed"
       /\ UNCHANGED vars

Spec == Init /\ [][Next]_vars

ResumingImpliesResumedJournal ==
    \A r \in RunIds : runtimeState[r] = "Resuming" => HasResumedJournal(r)

SuccessfulResumeHasResumedJournal ==
    \A r \in RunIds : lastOutcome[r] = "ResumedOk" => HasResumedJournal(r)

DriveFailureLeavesResumedJournal ==
    \A r \in RunIds :
        lastOutcome[r] = "DriveFailed" =>
            /\ runtimeState[r] = "Resumable"
            /\ HasResumedJournal(r)

RecordedDriveFailuresHaveResumedJournal ==
    \A r \in driveFailures : HasResumedJournal(r)

AppendFailureRollsBackToResumable ==
    \A r \in RunIds : lastOutcome[r] = "AppendFailed" => runtimeState[r] = "Resumable"

DriveContinueAfterResumedAppend ==
    \A i \in DOMAIN opLog :
        /\ opLog[i].kind = "RuntimeEvent"
        /\ opLog[i].event = "DriveContinue"
        => \E j \in 1..(i - 1) :
            /\ opLog[j].kind = "Journal"
            /\ opLog[j].event = "Resumed"
            /\ opLog[j].run = opLog[i].run

StateConstraint == Len(journal) <= MaxJournalLength /\ Len(opLog) <= MaxOpLogLength

THEOREM Spec => []TypeOK
THEOREM Spec => []ResumingImpliesResumedJournal
THEOREM Spec => []SuccessfulResumeHasResumedJournal
THEOREM Spec => []DriveFailureLeavesResumedJournal
THEOREM Spec => []RecordedDriveFailuresHaveResumedJournal
THEOREM Spec => []AppendFailureRollsBackToResumable
THEOREM Spec => []DriveContinueAfterResumedAppend

====
