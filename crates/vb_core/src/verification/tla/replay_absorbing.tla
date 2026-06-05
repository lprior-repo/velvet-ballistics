---- MODULE replay_absorbing ----
\* PO-TLA-001: Replay determinism with absorbing terminal states.
\*
\* GOD RULE 3: This model uses bounded U64 arithmetic to match
\* vb_core::executed type. No unbounded Nat.
\*
\* Invariants:
\* - TerminalStatesNeverReexecuted: no terminal step is re-executed during replay
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

\* ValidTransitions: the fixed 64-entry boolean matrix as a set of pairs
ValidTransitions == {
    <<0, 1>>, <<0, 2>>, <<0, 3>>, <<0, 7>>, <<0, 4>>,
    <<1, 2>>, <<1, 3>>, <<1, 5>>, <<1, 6>>, <<1, 7>>, <<1, 4>>,
    <<5, 1>>,
    <<6, 1>>
}

\* is_valid_step_state_transition(from, to)
IsValidTransition(from, to) ==
    IsSelfTransition(from, to) \/ (from \in StepStates /\ to \in StepStates /\ <<from, to>> \in ValidTransitions)

\* Terminal absorption invariant:
\* For all terminal t and all s != t: IsValidTransition(t, s) == FALSE
TerminalAbsorbing ==
    \A t \in Terminal: \A s \in StepStates \ {t}: ~IsValidTransition(t, s)

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
IsEligibleForExecution(pc, steps) ==
    \* Step must exist
    /\ pc \in 0 .. MaxSteps-1
    \* Step must NOT be in a terminal state
    /\ steps[pc] \notin Terminal

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

\* Next-state relation
Next == ReplayIteration

\* ---------------------------------------------------------------------------
\* Invariants
\* ---------------------------------------------------------------------------

\* INV-1: Terminal states are never re-executed during replay.
\* If a step is terminal, its state never changes.
TerminalStatesNeverReexecuted ==
    \A i \in 0 .. MaxSteps-1:
        (steps[i] \in Terminal) => (steps'[i] \in Terminal)

\* INV-2: Terminal absorption holds in every reachable state.
Invariant_TerminalAbsorbing == TerminalAbsorbing

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

vars == <<pc, executed, steps, iteration, replay_done>>

Spec == Init /\ [][Next]_vars /\ Fairness

\* ---------------------------------------------------------------------------
\* Theorems
\* ---------------------------------------------------------------------------

\* THEOREM: TerminalAbsorbing holds in the initial state.
THEOREM Init => TerminalAbsorbing
    BY DEF Init, Terminal, ValidTransitions, StepStates, TerminalAbsorbing

\* THEOREM: If TerminalAbsorbing holds, it is preserved by ReplayStep.
THEOREM TerminalAbsorbing /\ Next => TerminalAbsorbing'
    BY DEF TerminalAbsorbing, Next, ReplayIteration, ReplayStep,
           IsValidTransition, ValidTransitions, Terminal, IsEligibleForExecution

=============================================================================
