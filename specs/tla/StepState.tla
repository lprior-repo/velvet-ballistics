(* StepState.tla
 *
 * Per-step execution state machine from crates/vb_core/src/frame.rs:72
 * (is_valid_step_state_transition; Succeeded -> Pending at :91).
 *
 * Valid transitions (matching Rust validate_transition()):
 *   Pending -> Running, Succeeded, Failed, Cancelled, Skipped
 *   Running  -> Succeeded, Failed, Waiting, Asking, Cancelled, Skipped
 *   Waiting | Asking -> Running
 *   Succeeded -> Pending (bounded loop body re-entry only), Succeeded
 *   state == next  (idempotent re-mark, always valid)
 *   Terminal states (Succeeded, Failed, Cancelled, Skipped) block outward
 *   transitions, except Succeeded -> Pending (MASTER.md §45).
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
    CASE source = "Pending"  -> {"Running", "Succeeded", "Failed", "Cancelled", "Skipped"}
      [] source = "Running"  -> {"Succeeded", "Failed", "Waiting", "Asking", "Cancelled", "Skipped"}
      [] source = "Waiting"  -> {"Running"}
      [] source = "Asking"  -> {"Running"}
      [] source = "Succeeded" -> {"Pending"}
      [] OTHER              -> {}

IsValidTransition(source, next) ==
    \/ source = next
    \/ next \in ValidNext(source)

TypeInvariant ==
    \A step \in StepId : step_state[step] \in StateNames

TerminalStateBlocksOutwardTransitions ==
    \A step \in StepId :
        step_state[step] \in TerminalStates
            => \A next \in StateNames :
                IsValidTransition(step_state[step], next) <=> (next = step_state[step] \/ (step_state[step] = "Succeeded" /\ next = "Pending"))

\* Mirrors Verus proof_succeeded_reentry_only_to_pending
\* (verification/verus/step_state_machine.rs:473): from Succeeded, the
\* only outward transition is back to Pending.
SucceededReentryOnlyToPending ==
    \A step \in StepId :
        step_state[step] = "Succeeded"
            => \A next \in StateNames :
                IsValidTransition("Succeeded", next) <=> (next = "Succeeded" \/ next = "Pending")

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
        \/ Transition(step, "Pending")

Spec == Init /\ [][Next]_step_state

THEOREM Spec => []TypeInvariant
THEOREM Spec => []TerminalStateBlocksOutwardTransitions
THEOREM Spec => []SucceededReentryOnlyToPending

====
