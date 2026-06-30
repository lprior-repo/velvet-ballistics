---- MODULE RunLifecycle ----
EXTENDS Naturals, FiniteSets, Sequences, TLC

\* Obligation: PO-003
\* Requirement: TLA-WF-003 (POST-002)
\* Model: Run lifecycle with terminal states — no commands after terminal.
\* Bounds: MaxSteps <= 5

CONSTANTS MAX_STEPS

ASSUME MAX_STEPS \in Nat \ {0}
ASSUME MAX_STEPS <= 5

VARIABLES
    run_state,      \* Current state of the run
    step_count,     \* Number of steps executed
    terminal_reached, \* Whether terminal state has been reached
    last_event,     \* Last event recorded
    prev_terminal   \* TRUE if terminal was reached in previous state

TerminalStates == {"succeeded", "failed", "skipped", "cancelled"}
MutableStates == {"queued", "running", "await_action", "await_ask"}
AllStates == MutableStates \cup TerminalStates
Events == {"none", "submit", "step", "action_suspend", "action_complete", "ask_suspend", "ask_complete", "succeed", "fail", "skip", "cancel"}

vars == <<run_state, step_count, terminal_reached, last_event, prev_terminal>>

TypeOK ==
    /\ run_state \in AllStates
    /\ step_count \in Nat
    /\ terminal_reached \in BOOLEAN
    /\ last_event \in Events
    /\ prev_terminal \in BOOLEAN

Init ==
    /\ run_state = "queued"   \* Start in queued so Tick enables
    /\ step_count = 0
    /\ terminal_reached = FALSE
    /\ last_event = "none"
    /\ prev_terminal = FALSE

CanStep == step_count < MAX_STEPS

Tick ==
    /\ run_state = "queued"
    /\ run_state' = "running"
    /\ UNCHANGED <<step_count, terminal_reached, last_event, prev_terminal>>

Step ==
    /\ CanStep
    /\ run_state = "running"
    /\ run_state' = "running"
    /\ step_count' = step_count + 1
    /\ terminal_reached' = FALSE
    /\ last_event' = "step"
    /\ prev_terminal' = FALSE

SuspendAction ==
    /\ run_state = "running"
    /\ run_state' = "await_action"
    /\ UNCHANGED <<step_count, terminal_reached, last_event, prev_terminal>>

CompleteAction ==
    /\ run_state = "await_action"
    /\ run_state' = "running"
    /\ UNCHANGED <<step_count, terminal_reached, last_event, prev_terminal>>

SuspendAsk ==
    /\ run_state = "running"
    /\ run_state' = "await_ask"
    /\ UNCHANGED <<step_count, terminal_reached, last_event, prev_terminal>>

CompleteAsk ==
    /\ run_state = "await_ask"
    /\ run_state' = "running"
    /\ UNCHANGED <<step_count, terminal_reached, last_event, prev_terminal>>

Terminal(state) ==
    /\ terminal_reached = FALSE
    /\ run_state' = state
    /\ terminal_reached' = TRUE
    /\ prev_terminal' = TRUE   \* Mark that we just entered terminal
    /\ UNCHANGED <<step_count, last_event>>

Succeed ==
    /\ run_state \in {"running", "await_action", "await_ask"}
    /\ Terminal("succeeded")

Fail ==
    /\ run_state \in {"running", "await_action", "await_ask"}
    /\ Terminal("failed")

Skip ==
    /\ run_state \in {"running", "await_action", "await_ask"}
    /\ Terminal("skipped")

Cancel ==
    /\ run_state \in {"running", "await_action", "await_ask"}
    /\ Terminal("cancelled")

\* Stutter when terminal to avoid deadlock; clears prev_terminal flag and last_event
Stutter ==
    /\ terminal_reached = TRUE
    /\ UNCHANGED <<run_state, step_count, terminal_reached>>
    /\ prev_terminal' = FALSE
    /\ last_event' = "none"  \* Clear to indicate no new event in terminal state

Progress ==
    \/ Tick
    \/ Step
    \/ SuspendAction
    \/ CompleteAction
    \/ SuspendAsk
    \/ CompleteAsk
    \/ Succeed
    \/ Fail
    \/ Skip
    \/ Cancel
    \/ Stutter

Spec == Init /\ [][Progress]_vars

\* Invariant: TerminalUniqueness — terminal state reached at most once
TerminalUniqueness ==
    terminal_reached => step_count <= MAX_STEPS

\* Invariant: NoCommandAfterTerminal — no state-changing events after terminal
\* We only check last_event when prev_terminal = FALSE, meaning we're in a
\* terminal state that was entered at least one step ago (or never entered).
NoCommandAfterTerminal ==
    terminal_reached =>
        /\ run_state \in TerminalStates
        /\ (prev_terminal = FALSE => last_event \notin {"step", "action_suspend", "ask_suspend"})

===============================================================================
