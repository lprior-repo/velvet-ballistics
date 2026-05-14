---- MODULE ResumeStateMachine ----
EXTENDS Naturals, Sequences, FiniteSets

CONSTANTS RunIds, MaxJournalLength

VARIABLES runtimeState, journal, pending, resumed

vars == <<runtimeState, journal, pending, resumed>>

States == {"Initial", "Running", "Resumable", "Resuming", "Failed"}

TypeOK ==
    /\ runtimeState \in [RunIds -> States]
    /\ journal \in Seq([kind: {"Started", "Resumed", "Failed"}, run: RunIds])
    /\ pending \subseteq RunIds
    /\ resumed \subseteq RunIds
    /\ Len(journal) <= MaxJournalLength

Init ==
    /\ runtimeState = [r \in RunIds |-> "Initial"]
    /\ journal = <<>>
    /\ pending = {}
    /\ resumed = {}

CanAppend == Len(journal) < MaxJournalLength

AppendEvent(kind, r) == Append(journal, [kind |-> kind, run |-> r])

StartRun(r) ==
    /\ CanAppend
    /\ runtimeState[r] = "Initial"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Running"]
    /\ journal' = AppendEvent("Started", r)
    /\ UNCHANGED <<pending, resumed>>

Suspend(r) ==
    /\ runtimeState[r] = "Running"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Resumable"]
    /\ UNCHANGED <<journal, pending, resumed>>

BeginResume(r) ==
    /\ runtimeState[r] = "Resumable"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Resuming"]
    /\ pending' = pending \cup {r}
    /\ UNCHANGED <<journal, resumed>>

CompleteResume(r) ==
    /\ CanAppend
    /\ r \in pending
    /\ runtimeState[r] = "Resuming"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Running"]
    /\ journal' = AppendEvent("Resumed", r)
    /\ pending' = pending \ {r}
    /\ resumed' = resumed \cup {r}

FailResume(r) ==
    /\ CanAppend
    /\ r \in pending
    /\ runtimeState[r] = "Resuming"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Failed"]
    /\ journal' = AppendEvent("Failed", r)
    /\ pending' = pending \ {r}
    /\ UNCHANGED resumed

FailRun(r) ==
    /\ CanAppend
    /\ runtimeState[r] = "Running"
    /\ runtimeState' = [runtimeState EXCEPT ![r] = "Failed"]
    /\ journal' = AppendEvent("Failed", r)
    /\ UNCHANGED <<pending, resumed>>

Stutter == UNCHANGED vars

Next ==
    \/ \E r \in RunIds: StartRun(r)
    \/ \E r \in RunIds: Suspend(r)
    \/ \E r \in RunIds: BeginResume(r)
    \/ \E r \in RunIds: CompleteResume(r)
    \/ \E r \in RunIds: FailResume(r)
    \/ \E r \in RunIds: FailRun(r)
    \/ Stutter

Spec == Init /\ [][Next]_vars

NoDoubleRunning == \A r \in RunIds: runtimeState[r] = "Running" => r \notin pending

FailedNotResumable == \A r \in RunIds: runtimeState[r] = "Failed" => ~ENABLED BeginResume(r)

JournalAppendBeforeSuccess ==
    \A r \in resumed:
        \E i \in DOMAIN journal:
            /\ journal[i].kind = "Resumed"
            /\ journal[i].run = r

JournalImmutable == Len(journal) <= MaxJournalLength

ValidTransition == TypeOK /\ NoDoubleRunning /\ FailedNotResumable

====
