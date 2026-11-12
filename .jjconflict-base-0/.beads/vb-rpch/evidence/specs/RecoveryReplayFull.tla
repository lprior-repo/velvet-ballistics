------------------------- MODULE RecoveryReplayFull -------------------------
EXTENDS Integers, FiniteSets, Sequences, TLC

CONSTANTS
    RUN_ID,
    MAX_STEPS,
    MAX_ACTIONS,
    MAX_EVENTS,
    MAXSEQ

VARIABLES
    events,
    replayed,
    tracker,
    step_counter,
    divergence_detected,
    scheduled_actions,
    completed_actions,
    failed_actions,
    terminal_event,
    is_terminal,
    snapshot_seq,
    tail_events,
    max_step_seen

EventType == {
    "RunAccepted",
    "RunAdmission",
    "StepStarted",
    "StepSucceeded",
    "StepFailed",
    "ActionScheduled",
    "ActionCompleted",
    "ActionFailed",
    "SlotWritten",
    "WaitScheduled",
    "AskScheduled",
    "RunFinished",
    "RunCancelled",
    "RunFailed"
}

JournalEventRec == [
    type: EventType,
    step: 0..MAX_STEPS,
    action: 0..MAX_ACTIONS,
    seq: 0..MAXSEQ,
    attempt: 1..3
]

StepOrderInvariant ==
    \A i \in 1..(Len(events)-1):
        /\ events[i].type \in {"StepStarted", "StepSucceeded", "StepFailed", "ActionScheduled", "ActionCompleted", "ActionFailed"}
        /\ events[i+1].type \in {"StepStarted", "StepSucceeded", "StepFailed", "ActionScheduled", "ActionCompleted", "ActionFailed"}
        => events[i].step <= events[i+1].step

NoDivergenceInvariant ==
    divergence_detected = FALSE

NoDoubleScheduling ==
    \A i \in 1..Len(events):
        LET e == events[i]
        IN e.type = "ActionScheduled" =>
            \A j \in 1..(i-1):
                events[j].type /= "ActionScheduled" \/
                events[j].action /= e.action \/
                events[j].step /= e.step

IsTerminalEvent(e) ==
    e.type \in {"RunFinished", "RunCancelled", "RunFailed"}

ActionSafety ==
    completed_actions \cap failed_actions = {}

Init ==
    /\ events = <<>>
    /\ replayed = <<>>
    /\ tracker = [completed |-> {}, failed |-> {}]
    /\ step_counter = 0
    /\ divergence_detected = FALSE
    /\ scheduled_actions = {}
    /\ completed_actions = {}
    /\ failed_actions = {}
    /\ terminal_event = [type |-> "RunAccepted", step |-> 0, action |-> 0, seq |-> 0, attempt |-> 1]
    /\ is_terminal = FALSE
    /\ snapshot_seq = 0
    /\ tail_events = <<>>
    /\ max_step_seen = 0

Next ==
    \E e \in JournalEventRec:
        /\ events' = Append(events, e)
        /\ IF e.type \in {"StepStarted", "StepSucceeded", "StepFailed", "ActionScheduled", "ActionCompleted", "ActionFailed"} THEN
               /\ e.step >= max_step_seen
               /\ max_step_seen' = e.step
           ELSE
               /\ max_step_seen' = max_step_seen
        /\ IF e.type = "ActionScheduled" THEN
               /\ e.action \notin scheduled_actions
               /\ scheduled_actions' = scheduled_actions \cup {e.action}
               /\ completed_actions' = completed_actions
               /\ failed_actions' = failed_actions
               /\ divergence_detected' = FALSE
               /\ step_counter' = step_counter
               /\ terminal_event' = terminal_event
               /\ is_terminal' = is_terminal
           ELSE IF e.type = "ActionCompleted" THEN
               /\ e.action \in scheduled_actions
               /\ completed_actions' = completed_actions \cup {e.action}
               /\ scheduled_actions' = scheduled_actions
               /\ failed_actions' = failed_actions
               /\ divergence_detected' = FALSE
               /\ step_counter' = step_counter
               /\ terminal_event' = terminal_event
               /\ is_terminal' = is_terminal
           ELSE IF e.type = "ActionFailed" THEN
               /\ e.action \in scheduled_actions
               /\ failed_actions' = failed_actions \cup {e.action}
               /\ scheduled_actions' = scheduled_actions
               /\ completed_actions' = completed_actions
               /\ divergence_detected' = FALSE
               /\ step_counter' = step_counter
               /\ terminal_event' = terminal_event
               /\ is_terminal' = is_terminal
           ELSE IF e.type = "StepStarted" THEN
               /\ step_counter' = step_counter + 1
               /\ scheduled_actions' = scheduled_actions
               /\ completed_actions' = completed_actions
               /\ failed_actions' = failed_actions
               /\ divergence_detected' = FALSE
               /\ terminal_event' = terminal_event
               /\ is_terminal' = is_terminal
           ELSE IF e.type = "RunFinished" \/ e.type = "RunCancelled" \/ e.type = "RunFailed" THEN
               /\ terminal_event' = e
               /\ is_terminal' = TRUE
               /\ scheduled_actions' = scheduled_actions
               /\ completed_actions' = completed_actions
               /\ failed_actions' = failed_actions
               /\ divergence_detected' = FALSE
               /\ step_counter' = step_counter
           ELSE
               /\ scheduled_actions' = scheduled_actions
               /\ completed_actions' = completed_actions
               /\ failed_actions' = failed_actions
               /\ divergence_detected' = FALSE
               /\ step_counter' = step_counter
               /\ terminal_event' = terminal_event
               /\ is_terminal' = is_terminal
        /\ replayed' = Append(replayed, e)
        /\ tracker' = tracker
        /\ snapshot_seq' = snapshot_seq
        /\ tail_events' = tail_events

Spec == Init /\ [][Next]_<<events, replayed, tracker, step_counter, divergence_detected, scheduled_actions, completed_actions, failed_actions, terminal_event, is_terminal, snapshot_seq, tail_events, max_step_seen>>

=============================================================================
