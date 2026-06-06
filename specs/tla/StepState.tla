(* StepState.tla
 *
 * Per-step execution state machine from crates/vb_core/src/frame.rs:394-431.
 *
 * Valid transitions (matching Rust validate_transition()):
 *   Pending -> Running, Succeeded, Failed, Cancelled, Skipped
 *   Running  -> Succeeded, Failed, Waiting, Asking, Cancelled, Skipped
 *   Waiting | Asking -> Running
 *   Succeeded -> Running  (loop body re-entry; only outward transition
 *                          admitted for a terminal state)
 *   state == next  (idempotent re-mark, always valid)
 *   Terminal states (Failed, Cancelled, Skipped) block all outward transitions
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
      [] source = "Succeeded"-> {"Running"}
      [] OTHER              -> {}

IsValidTransition(source, next) ==
    \/ source = next
    \/ next \in ValidNext(source)

TypeInvariant ==
    \A step \in StepId : step_state[step] \in StateNames

\* Non-Succeeded terminal states block every outward non-self transition.
\* Succeeded is the partial exception: it admits a single outward edge to
\* Running for loop body re-entry (see production frame.rs validate_transition).
TerminalStateBlocksOutwardTransitions ==
    \A step \in StepId :
        step_state[step] \in TerminalStates
            => \A next \in StateNames :
                IsValidTransition(step_state[step], next)
                    <=> (next = step_state[step])
                        \/ (step_state[step] = "Succeeded" /\ next = "Running")

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
