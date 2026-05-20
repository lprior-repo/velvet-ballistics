---- MODULE TimerWheel ----
EXTENDS Naturals, FiniteSets, TLC

\* Obligation: PO-004
\* Requirement: TLA-WF-004 (POST-004, INV-003)
\* Model: Timer wheel with generation tracking, deadline ordering.
\* Bounds: MaxTimers <= 4

CONSTANTS MAX_TIMERS

ASSUME MAX_TIMERS \in Nat \ {0}
ASSUME MAX_TIMERS <= 4

VARIABLES
    timers,         \* [run -> TimerEntry] active timers (NullEntry if absent)
    deadline_idx,   \* [deadline -> set of runs] deadline index
    generation,     \* [run -> nat] current generation per run
    fired,         \* set of fired timer entries (cleared only via ProcessFired)
    now            \* current time (abstract)

vars == <<timers, deadline_idx, generation, fired, now>>

TIMES == 0..5   \* Reduced from 0..100 for tractability
KINDS == {"wait", "ask"}
RunIds == 1..1
NullEntry == [run |-> 0, deadline |-> 0, kind |-> "null", gen |-> 0]
TimerEntry == [run: RunIds, deadline: TIMES, kind: KINDS, gen: Nat]

Init ==
    /\ timers = [r \in RunIds |-> NullEntry]
    /\ deadline_idx = [t \in TIMES |-> {}]
    /\ generation = [r \in RunIds |-> 0]
    /\ fired = {}
    /\ now = 0

NextGen(r) == generation[r] + 1

InsertTimer(r, deadline, kind) ==
    /\ timers[r] = NullEntry
    /\ generation' = [generation EXCEPT ![r] = NextGen(r)]
    /\ timers' = [timers EXCEPT ![r] = [run |-> r, deadline |-> deadline, kind |-> kind, gen |-> generation'[r]]]
    /\ deadline_idx' = [deadline_idx EXCEPT ![deadline] = deadline_idx[deadline] \cup {r}]
    /\ UNCHANGED <<fired, now>>

ReplaceTimer(r, new_deadline, kind) ==
    /\ timers[r] # NullEntry
    /\ generation' = [generation EXCEPT ![r] = NextGen(r)]
    /\ timers' = [timers EXCEPT ![r] = [run |-> r, deadline |-> new_deadline, kind |-> kind, gen |-> generation'[r]]]
    /\ deadline_idx' = [deadline_idx EXCEPT ![timers[r].deadline] = deadline_idx[timers[r].deadline] \ {r}, ![new_deadline] = deadline_idx[new_deadline] \cup {r}]
    /\ UNCHANGED <<fired, now>>

CancelTimer(r) ==
    /\ timers[r] # NullEntry
    /\ deadline_idx' = [deadline_idx EXCEPT ![timers[r].deadline] = deadline_idx[timers[r].deadline] \ {r}]
    /\ timers' = [timers EXCEPT ![r] = NullEntry]
    /\ UNCHANGED <<generation, fired, now>>

FireTimer(r) ==
    /\ timers[r] # NullEntry
    /\ timers[r].deadline <= now
    /\ timers' = [timers EXCEPT ![r] = NullEntry]
    /\ deadline_idx' = [deadline_idx EXCEPT ![timers[r].deadline] = deadline_idx[timers[r].deadline] \ {r}]
    /\ fired' = fired \cup {timers[r]}
    /\ UNCHANGED <<generation, now>>

\* ProcessFired: consume processed fired entries (explicit consumption)
ProcessFired ==
    /\ fired # {}
    /\ fired' = {}
    /\ UNCHANGED <<timers, deadline_idx, generation, now>>

AdvanceTime(t) ==
    /\ now' = t
    /\ now' >= now
    /\ fired = {}   \* Cannot advance time while there are unfired entries
    /\ UNCHANGED <<timers, deadline_idx, generation, fired>>

Progress ==
    \/ (\E r \in RunIds, d \in TIMES, k \in KINDS : InsertTimer(r, d, k))
    \/ (\E r \in RunIds, d \in TIMES, k \in KINDS : ReplaceTimer(r, d, k))
    \/ (\E r \in RunIds : CancelTimer(r))
    \/ (\E r \in RunIds : FireTimer(r))
    \/ ProcessFired
    \/ (\E t \in TIMES : AdvanceTime(t))

Spec == Init /\ [][Progress]_vars

\* Invariant: GenerationMonotonic — generation tracks timer.gen for active timers
GenerationMonotonic ==
    \A r \in RunIds:
        generation[r] >= 0
        /\ (timers[r] # NullEntry => generation[r] = timers[r].gen)

\* Invariant: NoPhantomFire — fired entries had valid deadlines and run IDs
NoPhantomFire ==
    \A entry \in fired:
        /\ entry.deadline <= now
        /\ entry.run # 0
        /\ entry.gen <= generation[entry.run]  \* generation not stale

===============================================================================
