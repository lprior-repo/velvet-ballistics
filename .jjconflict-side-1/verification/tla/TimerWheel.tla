---- MODULE TimerWheel ----
EXTENDS Naturals, FiniteSets, Sequences

\* Obligations: PO-001, PO-002, PO-003, PO-004, PO-005, PO-006.
\* Bounded timer-wheel scheduling model for vb-f7k6.
\* Deadlines are encoded finite hardware/runtime values 0..MAX_TIME.
\* CheckedAdd is the only deadline constructor; modular wrap is not modeled.

CONSTANTS RUNS, KINDS, MAX_TIME, MAX_DURATION, MAX_GENERATION

VARIABLES runState, runIndex, deadlineIndex, generation, lastOutcome,
          firedEvents, deliveredEvents, rejectedEvents, coverage

vars == <<runState, runIndex, deadlineIndex, generation, lastOutcome,
          firedEvents, deliveredEvents, rejectedEvents, coverage>>

TIMES == 0..MAX_TIME
DURATIONS == 0..MAX_DURATION
GENERATIONS == 0..MAX_GENERATION

MutableStates == {"Active"}
TerminalStates == {"SuspendedOverflow", "Cancelled", "Shutdown", "Completed", "Failed"}
RunStates == MutableStates \cup TerminalStates

RejectReasons == {"Captured", "WrongGeneration", "WrongDeadline", "WrongKind"}

CoverageItems == {
  "ValidDelivery", "StaleAfterCancel", "StaleAfterReplace",
  "WrongGeneration", "WrongDeadline", "WrongKind", "TerminalRejected"
}

Outcomes == {
  "Init", "Inserted", "Replaced", "Cancelled", "CancelAbsent",
  "FiredDue", "ValidTimerFire", "InvalidTimerFire",
  "DeadlineOverflow", "RunNotTimerMutable", "LifecycleTerminal", "Idle"
}

TimerEntries == [run: RUNS, deadline: TIMES, kind: KINDS, gen: GENERATIONS]
FireEvents == [run: RUNS, deadline: TIMES, kind: KINDS, gen: GENERATIONS,
               firedAt: TIMES, reason: RejectReasons]

ActiveEntries == UNION {runIndex[r] : r \in RUNS}

EntryForRun(e, r) == e \in TimerEntries /\ e.run = r

EventEntry(ev) == [run |-> ev.run, deadline |-> ev.deadline, kind |-> ev.kind, gen |-> ev.gen]

CheckedAddOk(now, duration) ==
  /\ now \in TIMES
  /\ duration \in DURATIONS
  /\ duration <= MAX_TIME - now

CheckedDeadline(now, duration) == now + duration

NextGen(g) == g + 1

RemoveRun(idx, r) ==
  [d \in TIMES |-> {e \in idx[d] : e.run # r}]

AddEntry(idx, e) ==
  [d \in TIMES |-> IF d = e.deadline THEN idx[d] \cup {e} ELSE idx[d]]

RemoveEntry(idx, e) ==
  [d \in TIMES |-> IF d = e.deadline THEN idx[d] \ {e} ELSE idx[d]]

NoTimerForRun(r) == runIndex[r] = {}
HasTimerForRun(r) == runIndex[r] # {}

TypeOK ==
  /\ RUNS # {}
  /\ KINDS # {}
  /\ GENERATIONS # {}
  /\ MAX_TIME \in Nat
  /\ MAX_DURATION \in Nat
  /\ MAX_GENERATION \in Nat \ {0}
  /\ runState \in [RUNS -> RunStates]
  /\ generation \in [RUNS -> GENERATIONS]
  /\ runIndex \in [RUNS -> SUBSET TimerEntries]
  /\ Cardinality(firedEvents) <= 1
  /\ Cardinality(deliveredEvents) <= 1
  /\ Cardinality(rejectedEvents) <= 1
  /\ \A r \in RUNS:
      /\ Cardinality(runIndex[r]) <= 1
      /\ \A e \in runIndex[r]: EntryForRun(e, r)
  /\ deadlineIndex \in [TIMES -> SUBSET TimerEntries]
  /\ \A d \in TIMES: \A e \in deadlineIndex[d]: e.deadline = d
  /\ firedEvents \subseteq FireEvents
  /\ deliveredEvents \subseteq FireEvents
  /\ rejectedEvents \subseteq FireEvents
  /\ lastOutcome \in Outcomes
  /\ coverage \subseteq CoverageItems

BiIndexConsistent ==
  /\ \A e \in TimerEntries: e \in ActiveEntries <=> e \in deadlineIndex[e.deadline]
  /\ \A r \in RUNS: Cardinality({e \in ActiveEntries : e.run = r}) <= 1

NoDeadlineWrap ==
  /\ \A e \in ActiveEntries: e.deadline \in TIMES
  /\ \A ev \in firedEvents \cup deliveredEvents \cup rejectedEvents:
      /\ ev.deadline \in TIMES
      /\ ev.firedAt \in TIMES

OneActiveTimerPerRun ==
  \A r \in RUNS: Cardinality(runIndex[r]) <= 1

CancelRemovesAllIndexes ==
  \A r \in RUNS:
    runState[r] \in TerminalStates
      => /\ runIndex[r] = {}
         /\ \A d \in TIMES: \A e \in deadlineIndex[d]: e.run # r

ReplaceRemovesOldGeneration ==
  \A r \in RUNS:
    \A e \in ActiveEntries:
      e.run = r => e.gen = generation[r]

DueOnlyFires ==
  \A ev \in firedEvents \cup deliveredEvents \cup rejectedEvents: ev.deadline <= ev.firedAt

FireRemovesReturned ==
  \A ev \in deliveredEvents: EventEntry(ev) \notin ActiveEntries

StaleFireNoMutation ==
  \A ev \in rejectedEvents: EventEntry(ev) \notin ActiveEntries \/ runState[ev.run] \in TerminalStates

TerminalNoTimerMutation == CancelRemovesAllIndexes

DueExists == \E e \in ActiveEntries: e.deadline <= MAX_TIME

NoResurrectionAlways == []TerminalNoTimerMutation
DueTimerEventuallyFireable == [](DueExists => <>(firedEvents # {} \/ ~DueExists))
OverflowEventuallySuspended ==
  [](lastOutcome = "DeadlineOverflow" => \E r \in RUNS: runState[r] = "SuspendedOverflow")
CoverageEventuallyComplete == <> (CoverageItems \subseteq coverage)

\* PO-005/PO-006 mechanical coverage probes.  Each `Missing*`
\* predicate is intended to be configured as an invariant in a dedicated
\* coverage CFG; TLC must violate it with a concrete trace that reaches
\* the corresponding coverage item.  This is existential reachability
\* evidence and is separate from the all-behaviors safety/liveness model.
MissingValidDelivery == "ValidDelivery" \notin coverage
MissingStaleAfterCancel == "StaleAfterCancel" \notin coverage
MissingStaleAfterReplace == "StaleAfterReplace" \notin coverage
MissingWrongGeneration == "WrongGeneration" \notin coverage
MissingWrongDeadline == "WrongDeadline" \notin coverage
MissingWrongKind == "WrongKind" \notin coverage
MissingTerminalRejected == "TerminalRejected" \notin coverage

Init ==
  /\ runState = [r \in RUNS |-> "Active"]
  /\ runIndex = [r \in RUNS |-> {}]
  /\ deadlineIndex = [d \in TIMES |-> {}]
  /\ generation = [r \in RUNS |-> 0]
  /\ lastOutcome = "Init"
  /\ firedEvents = {}
  /\ deliveredEvents = {}
  /\ rejectedEvents = {}
  /\ coverage = {}

InsertOrReplace(r, now, duration, kind) ==
  /\ r \in RUNS
  /\ kind \in KINDS
  /\ runState[r] = "Active"
  /\ CheckedAddOk(now, duration)
  /\ generation[r] < MAX_GENERATION
  /\ LET old == runIndex[r] IN
     LET ng == NextGen(generation[r]) IN
     LET e == [run |-> r, deadline |-> CheckedDeadline(now, duration), kind |-> kind, gen |-> ng] IN
       /\ runIndex' = [runIndex EXCEPT ![r] = {e}]
       /\ deadlineIndex' = AddEntry(RemoveRun(deadlineIndex, r), e)
       /\ generation' = [generation EXCEPT ![r] = ng]
       /\ lastOutcome' = IF old = {} THEN "Inserted" ELSE "Replaced"
       /\ UNCHANGED <<runState, firedEvents, deliveredEvents, rejectedEvents, coverage>>

InsertTimer ==
  \E r \in RUNS, now \in TIMES, duration \in DURATIONS, kind \in KINDS:
    InsertOrReplace(r, now, duration, kind)

OverflowInsert ==
  \E r \in RUNS, now \in TIMES, duration \in DURATIONS, kind \in KINDS:
    /\ kind \in KINDS
    /\ runState[r] = "Active"
    /\ duration > MAX_TIME - now
    /\ runState' = [runState EXCEPT ![r] = "SuspendedOverflow"]
    /\ runIndex' = [runIndex EXCEPT ![r] = {}]
    /\ deadlineIndex' = RemoveRun(deadlineIndex, r)
    /\ lastOutcome' = "DeadlineOverflow"
    /\ UNCHANGED <<generation, firedEvents, deliveredEvents, rejectedEvents, coverage>>

RejectedLifecycleInsert ==
  \E r \in RUNS, now \in TIMES, duration \in DURATIONS, kind \in KINDS:
    /\ kind \in KINDS
    /\ runState[r] \in TerminalStates
    /\ lastOutcome' = "RunNotTimerMutable"
    /\ UNCHANGED <<runState, runIndex, deadlineIndex, generation, firedEvents, deliveredEvents, rejectedEvents, coverage>>

CancelTimer ==
  \E r \in RUNS:
    /\ runState[r] = "Active"
    /\ runIndex' = [runIndex EXCEPT ![r] = {}]
    /\ deadlineIndex' = RemoveRun(deadlineIndex, r)
    /\ lastOutcome' = IF runIndex[r] = {} THEN "CancelAbsent" ELSE "Cancelled"
    /\ UNCHANGED <<runState, generation, firedEvents, deliveredEvents, rejectedEvents, coverage>>

CaptureDueTimerFired ==
  \E now \in TIMES, e \in ActiveEntries:
    /\ firedEvents = {}
    /\ e.deadline <= now
    /\ firedEvents' = firedEvents \cup {[run |-> e.run, deadline |-> e.deadline,
          kind |-> e.kind, gen |-> e.gen, firedAt |-> now, reason |-> "Captured"]}
    /\ lastOutcome' = "FiredDue"
    /\ UNCHANGED <<runState, runIndex, deadlineIndex, generation, deliveredEvents, rejectedEvents, coverage>>

InjectWrongGenerationFire ==
  \E now \in TIMES, e \in ActiveEntries, g \in GENERATIONS:
    /\ firedEvents = {}
    /\ e.deadline <= now
    /\ g < e.gen
    /\ firedEvents' = firedEvents \cup {[run |-> e.run, deadline |-> e.deadline,
          kind |-> e.kind, gen |-> g, firedAt |-> now, reason |-> "WrongGeneration"]}
    /\ lastOutcome' = "FiredDue"
    /\ UNCHANGED <<runState, runIndex, deadlineIndex, generation, deliveredEvents, rejectedEvents, coverage>>

InjectWrongDeadlineFire ==
  \E now \in TIMES, e \in ActiveEntries, d \in TIMES:
    /\ firedEvents = {}
    /\ e.deadline <= now
    /\ d # e.deadline
    /\ d <= now
    /\ firedEvents' = firedEvents \cup {[run |-> e.run, deadline |-> d,
          kind |-> e.kind, gen |-> e.gen, firedAt |-> now, reason |-> "WrongDeadline"]}
    /\ lastOutcome' = "FiredDue"
    /\ UNCHANGED <<runState, runIndex, deadlineIndex, generation, deliveredEvents, rejectedEvents, coverage>>

InjectWrongKindFire ==
  \E now \in TIMES, e \in ActiveEntries, k \in KINDS:
    /\ firedEvents = {}
    /\ e.deadline <= now
    /\ k # e.kind
    /\ firedEvents' = firedEvents \cup {[run |-> e.run, deadline |-> e.deadline,
          kind |-> k, gen |-> e.gen, firedAt |-> now, reason |-> "WrongKind"]}
    /\ lastOutcome' = "FiredDue"
    /\ UNCHANGED <<runState, runIndex, deadlineIndex, generation, deliveredEvents, rejectedEvents, coverage>>

RejectCoverage(ev) ==
  IF runState[ev.run] \in TerminalStates THEN "TerminalRejected"
  ELSE IF ev.reason = "WrongGeneration" THEN "WrongGeneration"
  ELSE IF ev.reason = "WrongDeadline" THEN "WrongDeadline"
  ELSE IF ev.reason = "WrongKind" THEN "WrongKind"
  ELSE IF HasTimerForRun(ev.run) THEN "StaleAfterReplace"
  ELSE "StaleAfterCancel"

DeliverTimerFired ==
  \E ev \in firedEvents:
    LET current == EventEntry(ev) IN
      /\ IF current \in ActiveEntries /\ runState[ev.run] = "Active"
            THEN /\ runIndex' = [runIndex EXCEPT ![ev.run] = {}]
                 /\ deadlineIndex' = RemoveRun(deadlineIndex, ev.run)
                 /\ deliveredEvents' = {ev}
                 /\ rejectedEvents' = rejectedEvents
                 /\ coverage' = coverage \cup {"ValidDelivery"}
                 /\ lastOutcome' = "ValidTimerFire"
            ELSE /\ UNCHANGED <<runIndex, deadlineIndex, deliveredEvents>>
                 /\ rejectedEvents' = {ev}
                 /\ coverage' = coverage \cup {RejectCoverage(ev)}
                 /\ lastOutcome' = "InvalidTimerFire"
      /\ firedEvents' = firedEvents \ {ev}
      /\ UNCHANGED <<runState, generation>>

ShutdownRun ==
  \E r \in RUNS:
    /\ runState[r] = "Active"
    /\ runState' = [runState EXCEPT ![r] = "Shutdown"]
    /\ runIndex' = [runIndex EXCEPT ![r] = {}]
    /\ deadlineIndex' = RemoveRun(deadlineIndex, r)
    /\ lastOutcome' = "LifecycleTerminal"
    /\ UNCHANGED <<generation, firedEvents, deliveredEvents, rejectedEvents, coverage>>

CompleteRun ==
  \E r \in RUNS:
    /\ runState[r] = "Active"
    /\ runState' = [runState EXCEPT ![r] = "Completed"]
    /\ runIndex' = [runIndex EXCEPT ![r] = {}]
    /\ deadlineIndex' = RemoveRun(deadlineIndex, r)
    /\ lastOutcome' = "LifecycleTerminal"
    /\ UNCHANGED <<generation, firedEvents, deliveredEvents, rejectedEvents, coverage>>

FailRun ==
  \E r \in RUNS:
    /\ runState[r] = "Active"
    /\ runState' = [runState EXCEPT ![r] = "Failed"]
    /\ runIndex' = [runIndex EXCEPT ![r] = {}]
    /\ deadlineIndex' = RemoveRun(deadlineIndex, r)
    /\ lastOutcome' = "LifecycleTerminal"
    /\ UNCHANGED <<generation, firedEvents, deliveredEvents, rejectedEvents, coverage>>

Idle ==
  /\ lastOutcome' = "Idle"
  /\ UNCHANGED <<runState, runIndex, deadlineIndex, generation, firedEvents,
                deliveredEvents, rejectedEvents, coverage>>

Next ==
  InsertTimer \/ OverflowInsert \/ RejectedLifecycleInsert \/ CancelTimer \/
  CaptureDueTimerFired \/ InjectWrongGenerationFire \/ InjectWrongDeadlineFire \/
  InjectWrongKindFire \/ DeliverTimerFired \/ ShutdownRun \/ CompleteRun \/ FailRun \/ Idle

Spec ==
  /\ Init
  /\ [][Next]_vars
  /\ WF_vars(CaptureDueTimerFired)
  /\ WF_vars(DeliverTimerFired)

====
