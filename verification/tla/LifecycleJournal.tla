---- MODULE LifecycleJournal ----
EXTENDS Naturals, Sequences, FiniteSets

\* Obligations: VB-REPLAY-001, VB-REPLAY-002, VB-REPLAY-003.

CONSTANTS ActionIds, MaxSeq

VARIABLES journal, dispatched, replayPointer, replayed, inFlight

vars == <<journal, dispatched, replayPointer, replayed, inFlight>>

TypeOK ==
  /\ journal \in Seq([seq: 1..MaxSeq, action: ActionIds])
  /\ dispatched \subseteq ActionIds
  /\ replayPointer \in 1..(Len(journal) + 1)
  /\ replayed \subseteq ActionIds
  /\ inFlight \subseteq ActionIds

Entry(seq, action) == [seq |-> seq, action |-> action]

Seqs(s) == {s[i].seq : i \in 1..Len(s)}

Actions(s) == {s[i].action : i \in 1..Len(s)}

StrictlyIncreasing(s) ==
  \A i, j \in 1..Len(s) : i < j => s[i].seq < s[j].seq

JournalValidity ==
  \A i \in 1..Len(journal) :
    /\ journal[i].action \in ActionIds
    /\ journal[i].seq \in 1..MaxSeq

MonotonicSequence == StrictlyIncreasing(journal)

ReplayOrderPreserved ==
  /\ replayPointer \in 1..(Len(journal) + 1)
  /\ \A i \in 1..Len(journal) : i < replayPointer => journal[i].action \in replayed
  /\ \A i \in 1..Len(journal) : i >= replayPointer => ~(journal[i].action \in replayed)

ReplayNoDuplicate == Cardinality(replayed) = Cardinality(Actions(journal) \cap replayed)

NoOrphanDispatch == dispatched \subseteq Actions(journal)

NoOrphanReplay == replayed \subseteq Actions(journal)

Init ==
  /\ journal = <<>>
  /\ dispatched = {}
  /\ replayPointer = 1
  /\ replayed = {}
  /\ inFlight = {}

WriteJournal ==
  /\ Len(journal) < MaxSeq
  /\ \E action \in ActionIds \ Actions(journal) :
      journal' = Append(journal, Entry(Len(journal) + 1, action))
  /\ UNCHANGED <<dispatched, replayPointer, replayed, inFlight>>

DispatchAction ==
  /\ \E i \in 1..Len(journal) :
      /\ journal[i].action \notin dispatched
      /\ dispatched' = dispatched \cup {journal[i].action}
  /\ UNCHANGED <<journal, replayPointer, replayed, inFlight>>

ReplayEntry ==
  /\ replayPointer <= Len(journal)
  /\ replayed' = replayed \cup {journal[replayPointer].action}
  /\ replayPointer' = replayPointer + 1
  /\ UNCHANGED <<journal, dispatched, inFlight>>

AdvancePointer ==
  /\ replayPointer <= Len(journal)
  /\ journal[replayPointer].action \in replayed
  /\ replayPointer' = replayPointer + 1
  /\ UNCHANGED <<journal, dispatched, replayed, inFlight>>

CompleteAction == UNCHANGED vars

Next == WriteJournal \/ DispatchAction \/ ReplayEntry \/ AdvancePointer \/ CompleteAction

Spec == Init /\ [][Next]_vars /\ WF_vars(WriteJournal) /\ WF_vars(ReplayEntry)

EventuallyAllJournaled == <>[](Len(journal) = Cardinality(ActionIds))
EventuallyReplayComplete == <>[](replayPointer = Len(journal) + 1)

====
