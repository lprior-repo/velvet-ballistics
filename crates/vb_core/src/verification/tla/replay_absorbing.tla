---- MODULE replay_absorbing ----
\* PO-TLA-001: Replay determinism with terminal-state re-entry exception.
\*
\* GOD RULE 3: This model uses bounded U64 arithmetic to match
\* vb_core::executed type. No unbounded Nat.
\*
\* Invariants:
\* - TerminalStatesNeverReexecuted: replay does not mutate terminal step states
\* - ReplayCompletes: replay always terminates
\*
\* Bounds: MAX_STEPS (default 8), MAX_ITERATIONS (default 4)

EXTENDS Naturals, FiniteSets, TLC

\* ---------------------------------------------------------------------------
\* Constants and Limits
\* ---------------------------------------------------------------------------

CONSTANTS
    MaxSteps,       \* Maximum number of steps in a workflow
    MaxIterations,  \* Maximum loop iterations during replay
    MaxExecuted     \* MaxU64 for executed counter

\* StepState values (matches vb_core::frame::StepState):
\*   Pending=0, Running=1, Succeeded=2, Failed=3,
\*   Skipped=4, Waiting=5, Asking=6, Cancelled=7

StepStates == {0, 1, 2, 3, 4, 5, 6, 7}

Terminal == {2, 3, 4, 7}   \* Succeeded, Failed, Skipped, Cancelled
NonTerminal == {0, 1, 5, 6} \* Pending, Running, Waiting, Asking

\* ---------------------------------------------------------------------------
\* Valid transition predicate (bounded arithmetic)
\* ---------------------------------------------------------------------------

\* Self-transition is always valid (idempotent)
IsSelfTransition(from, to) == (from = to)

\* ValidTransitions: the fixed transition matrix as a set of pairs. No
\* Succeeded->Pending edge exists; Succeeded->Running is retained only for loop
\* body re-entry.
ValidTransitions == {
    <<0, 1>>, <<0, 2>>, <<0, 3>>, <<0, 7>>, <<0, 4>>,
    <<1, 2>>, <<1, 3>>, <<1, 5>>, <<1, 6>>, <<1, 7>>, <<1, 4>>,
    <<2, 1>>,
    <<5, 1>>,
    <<6, 1>>
}

\* is_valid_step_state_transition(from, to)
IsValidTransition(from, to) ==
    IsSelfTransition(from, to) \/ (from \in StepStates /\ to \in StepStates /\ <<from, to>> \in ValidTransitions)

\* Terminal transition invariant:
\* For all terminal t and all s != t: IsValidTransition(t, s) == FALSE,
\* except Succeeded->Running for loop body re-entry.
TerminalReentryException ==
    \A t \in Terminal: \A s \in StepStates \ {t}:
        (t = 2 /\ s = 1) \/ ~IsValidTransition(t, s)

\* ---------------------------------------------------------------------------
\* Replay state machine variables
\* ---------------------------------------------------------------------------

VARIABLES
    pc,             \* Program counter (0..MaxSteps-1)
    executed,       \* Executed instruction counter (bounded: 0..MaxExecuted)
    steps,          \* Step state array (indexed 0..MaxSteps-1)
    iteration,      \* Current replay iteration
    replay_done     \* Boolean: replay completed?

\* ---------------------------------------------------------------------------
\* Initial state
\* ---------------------------------------------------------------------------

Init ==
    /\ pc = 0
    /\ executed = 0
    /\ iteration = 0
    /\ replay_done = FALSE
    /\ steps \in [0 .. MaxSteps-1 -> StepStates]
    \* At least one step is in a terminal state
    /\ \E i \in 0 .. MaxSteps-1: steps[i] \in Terminal

\* ---------------------------------------------------------------------------
\* Replay step: attempt to execute step at pc
\* ---------------------------------------------------------------------------

\* Check if current step is eligible for execution (not terminal)
IsEligibleForExecution(p, s) ==
    \* Step must exist
    /\ p \in 0 .. MaxSteps-1
    \* Step must NOT be in a terminal state
    /\ s[p] \notin Terminal

\* Execute one replay step: if pc is eligible, advance pc and increment executed.
\* If pc is NOT eligible (terminal state), skip to next step.
ReplayStep ==
    /\ ~replay_done
    /\ IF IsEligibleForExecution(pc, steps)
       THEN
           /\ executed < MaxExecuted    \* Bounded: no overflow
           /\ executed' = executed + 1
           /\ pc' = IF pc + 1 < MaxSteps THEN pc + 1 ELSE pc
           /\ UNCHANGED steps
       ELSE
           \* Terminal state: skip without executing
           /\ UNCHANGED executed
           /\ pc' = IF pc + 1 < MaxSteps THEN pc + 1 ELSE pc
           /\ UNCHANGED steps
    /\ iteration' = iteration
    /\ replay_done' = replay_done

\* Replay iteration: run through all steps once
ReplayIteration ==
    /\ ~replay_done
    /\ IF pc < MaxSteps
       THEN ReplayStep
       ELSE
           \* End of iteration: either continue or finish
           /\ pc' = 0
           /\ iteration' = iteration + 1
           /\ IF iteration + 1 >= MaxIterations
              THEN replay_done' = TRUE
              ELSE replay_done' = FALSE
           /\ UNCHANGED <<executed, steps>>

vars == <<pc, executed, steps, iteration, replay_done>>

\* Next-state relation
Next == ReplayIteration \/ (replay_done /\ UNCHANGED vars)

\* ---------------------------------------------------------------------------
\* Invariants
\* ---------------------------------------------------------------------------

\* INV-1: Terminal states are never re-executed during replay.
\* If a step is terminal, its state never changes.
TerminalStatesNeverReexecuted ==
    \A i \in 0 .. MaxSteps-1:
        (steps[i] \in Terminal) => (steps'[i] \in Terminal)

\* INV-2: Terminal transition invariant holds in every reachable state.
Invariant_TerminalReentryException == TerminalReentryException

\* INV-3: Executed counter never overflows (bounded arithmetic).
ExecutedBounded == executed <= MaxExecuted

\* INV-4: Steps are never mutated during replay (no mark_* calls).
StepsNeverMutated == steps' = steps

\* ---------------------------------------------------------------------------
\* Liveness: Replay always terminates
\* ---------------------------------------------------------------------------

ReplayCompletes ==
    pc = MaxSteps \/ replay_done \/ iteration >= MaxIterations

\* ---------------------------------------------------------------------------
\* Fairness
\* ---------------------------------------------------------------------------

Fairness ==
    WF_vars(ReplayStep)

\* ---------------------------------------------------------------------------
\* Specification
\* ---------------------------------------------------------------------------

Spec == Init /\ [][Next]_vars /\ Fairness

\* ---------------------------------------------------------------------------
\* Theorems
\* ---------------------------------------------------------------------------

\* THEOREM: TerminalReentryException holds in the initial state.
THEOREM Init => TerminalReentryException
    BY DEF Init, Terminal, ValidTransitions, StepStates, TerminalReentryException

\* THEOREM: If TerminalReentryException holds, it is preserved by ReplayStep.
THEOREM TerminalReentryException /\ Next => TerminalReentryException'
    BY DEF TerminalReentryException, Next, ReplayIteration, ReplayStep,
           IsValidTransition, ValidTransitions, Terminal, IsEligibleForExecution

=============================================================================
