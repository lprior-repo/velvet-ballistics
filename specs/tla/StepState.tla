(* StepState.tla
 *
 * Per-step execution state machine from crates/vb_core/src/frame.rs.
 *
 * Valid transitions (matching Rust is_valid_step_state_transition()):
 *   Pending -> Running, Succeeded, Failed, Cancelled, Skipped
 *   Running  -> Succeeded, Failed, Waiting, Asking, Cancelled, Skipped
 *   Waiting | Asking -> Running
 *   state == next  (idempotent re-mark, always valid)
 *
 * All terminal states (Succeeded, Failed, Cancelled, Skipped) are absorbing;
 * no terminal state transitions back to Running. Loop body re-entry uses
 * the explicit Succeeded->Pending admission path in
 * RunFrame::mark_pending before mark_running.
 *)

---- MODULE StepState ----

EXTENDS Integers, FiniteSets, TLC

CONSTANT StepId

VARIABLES
    step_state

StateNames == {"Pending", "Running", "Succeeded", "Failed",
               "Skipped", "Waiting", "Asking", "Cancelled"}

TerminalStates == {"Succeeded", "Failed", "Cancelled", "Skipped"}

ValidNext(source) ==
    CASE source = "Pending"   -> {"Running", "Succeeded", "Failed", "Cancelled", "Skipped"}
      [] source = "Running"  -> {"Succeeded", "Failed", "Waiting", "Asking", "Cancelled", "Skipped"}
      [] source = "Waiting"  -> {"Running"}
      [] source = "Asking"   -> {"Running"}
      [] OTHER              -> {}

IsValidTransition(source, next) ==
    \/ source = next
    \/ next \in ValidNext(source)

TypeInvariant ==
    \A step \in StepId : step_state[step] \in StateNames

\* All terminal states (including Succeeded) are fully absorbing:
\* they block every outward non-self transition. Loop re-entry uses
\* the explicit Succeeded->Pending admission path; no direct
\* Succeeded->Running edge is admitted.
TerminalStateBlocksOutwardTransitions ==
    \A step \in StepId :
        step_state[step] \in TerminalStates
            => \A next \in StateNames :
                IsValidTransition(step_state[step], next)
                    <=> (next = step_state[step])

Init ==
    step_state = [step \in StepId |-> "Pending"]

Tick(step) ==
    /\ step_state' = step_state

Transition(step, next) ==
    /\ IsValidTransition(step_state[step], next)
    /\ step_state' = [step_state EXCEPT ![step] = next]

Next ==
    \E step \in StepId :
        \/ Tick(step)
        \/ Transition(step, "Running")
        \/ Transition(step, "Succeeded")
        \/ Transition(step, "Failed")
        \/ Transition(step, "Cancelled")
        \/ Transition(step, "Skipped")
        \/ Transition(step, "Waiting")
        \/ Transition(step, "Asking")

Spec == Init /\ [][Next]_step_state

THEOREM Spec => []TypeInvariant
THEOREM Spec => []TerminalStateBlocksOutwardTransitions

====
