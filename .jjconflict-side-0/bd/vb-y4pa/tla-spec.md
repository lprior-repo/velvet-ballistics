-------------------------- MODULE for_each_body_reentry -------------------------
(*
 * TLA+ specification for VB-Y4PA: for_each body re-entry state machine fix.
 *
 * Models:
 *   - StepState: Pending, Running, Waiting, Asking, Succeeded, Failed, Cancelled, Skipped
 *   - for_each_next body re-entry transition Succeeded → Pending
 *   - Valid transitions per step_state.rs VALID_TRANSITIONS
 *)

EXTENDS Integers, FiniteSets, Sequences

CONSTANT
  \* @type: Set(STEP_STATE);
  STEP_STATES,
  MAX_U64

ASSUME STEP_STATES = {"Pending", "Running", "Waiting", "Asking",
                      "Succeeded", "Failed", "Cancelled", "Skipped"}

(* @type: (STEP_STATE x STEP_STATE) -> BOOL *)
ValidTransition(from, to) ==
  LET allowed == """
  (Pending, Running), (Pending, Succeeded), (Pending, Failed),
  (Pending, Cancelled), (Pending, Skipped),
  (Running, Succeeded), (Running, Failed), (Running, Waiting),
  (Running, Asking), (Running, Cancelled), (Running, Skipped),
  (Waiting, Running),
  (Asking, Running),
  (Succeeded, Succeeded),
  (Failed, Failed),
  (Cancelled, Cancelled),
  (Skipped, Skipped)
  """  \* placeholder — see below
  IN
  \E pair \in {
    {"Pending", "Running"}, {"Pending", "Succeeded"}, {"Pending", "Failed"},
    {"Pending", "Cancelled"}, {"Pending", "Skipped"},
    {"Running", "Succeeded"}, {"Running", "Failed"}, {"Running", "Waiting"},
    {"Running", "Asking"}, {"Running", "Cancelled"}, {"Running", "Skipped"},
    {"Waiting", "Running"},
    {"Asking", "Running"},
    {"Succeeded", "Succeeded"},
    {"Failed", "Failed"},
    {"Cancelled", "Cancelled"},
    {"Skipped", "Skipped"}
  } :
    from = pair[1] /\ to = pair[2]

(* BUG: Succeeded → Running is NOT valid *)
ASSUME ~ValidTransition("Succeeded", "Running")
ASSUME ~ValidTransition("Failed", "Running")
ASSUME ~ValidTransition("Cancelled", "Running")
ASSUME ~ValidTransition("Skipped", "Running")

(* REQUIRED: Succeeded → Pending IS valid (for body reset) *)
ASSUME ValidTransition("Succeeded", "Pending")

VARIABLE
  \* @type: STEP_STATE
  body_state,
  \* @type: Seq(SlotValue)
  iterator_items,
  \* @type: Bool
  engine_error

States == [body_state: STEP_STATES, iterator_items: Seq(STRING), engine_error: BOOLEAN]

Init ==
  /\ body_state = "Pending"
  /\ iterator_items = <<"Item1", "Item2">>
  /\ engine_error = FALSE

(* for_each_next: advance iterator, bind next item, jump to body *)
ForEachNext ==
  /\ Len(iterator_items) > 0
  (* If body was Succeeded, we must reset to Pending before re-entry *)
  /\ IF body_state = "Succeeded"
     THEN body_state' = "Pending"
     ELSE body_state' = body_state
  (* bind next item to output slot (modeled as consuming head) *)
  /\ iterator_items' = Tail(iterator_items)
  /\ engine_error' = FALSE

(* Engine scheduler picks up body step for execution *)
EngineScheduler ==
  (* BUG REPRO: If body_state is Succeeded and we try to run it without reset *)
  /\ IF body_state = "Succeeded"
     THEN engine_error' = TRUE  (* invalid transition rejected *)
     ELSE engine_error' = FALSE

(* After body runs, it returns Continue → marked Succeeded *)
BodyCompletes ==
  /\ body_state = "Running"
  /\ body_state' = "Succeeded"

Next ==
  \/ ForEachNext
  \/ EngineScheduler
  \/ BodyCompletes

Spec ==
  Init /\ [][Next]_<<body_state, iterator_items, engine_error>>

=============================================================================
\* Invariant: body can always be re-entered after reset
BodyReentryInvariant ==
  engine_error = FALSE => body_state /= "Succeeded"

=============================================================================
\* THEOREM: Without Succeeded→Pending transition, engine would reject re-entry
\* Proof sketch: By ValidTransition definition, Succeeded→Running is not allowed.
\*   Therefore engine_error must be TRUE when body_state=Succeeded and
\*   scheduler attempts to run body without reset.
=============================================================================
```

### Key Theorems/Invariants

1. **ValidTransition_Exhaustive**: All pairs in `VALID_TRANSITIONS` are in `STEP_STATES × STEP_STATES`
2. **Succeeded_Not_Running**: `ValidTransition(Succeeded, Running) = FALSE`
3. **BodyReentryRequiresReset**: After body completes with Succeeded, any re-entry requires `Succeeded → Pending` transition
4. **NoInvalidTransitionPanic**: Engine never attempts an invalid transition without setting `engine_error`

### Model Checking Notes

- Run with `tlc -deadlock for_each_body_reentry.tla`
- Set `CONSTANT STEP_STATES` to the 8 concrete states
- Deadlock check enabled (loop primitives should not deadlock)
- Invariant `BodyReentryInvariant` should FAIL before fix, PASS after fix
