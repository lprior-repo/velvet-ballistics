---- MODULE admission_header_before_ack ----
EXTENDS TLC

CONSTANTS ErrorCodes, NoCode

VARIABLES state, code, ack

vars == <<state, code, ack>>

States == {"Pending", "Rejected", "Acked"}
CodeDomain == ErrorCodes \cup {NoCode}

Init ==
    /\ state = "Pending"
    /\ code \in ErrorCodes
    /\ ack = FALSE

TypeOK ==
    /\ ErrorCodes # {}
    /\ state \in States
    /\ code \in CodeDomain
    /\ ack \in BOOLEAN

AdmissionReject ==
    /\ state = "Pending"
    /\ code \in ErrorCodes
    /\ state' = "Rejected"
    /\ ack' = FALSE
    /\ UNCHANGED code

StorageFail ==
    /\ state = "Pending"
    /\ code \in ErrorCodes
    /\ state' = "Rejected"
    /\ ack' = FALSE
    /\ UNCHANGED code

Ack ==
    /\ state = "Pending"
    /\ code = NoCode
    /\ state' = "Acked"
    /\ ack' = TRUE
    /\ UNCHANGED code

TerminalStutter ==
    /\ state \in {"Rejected", "Acked"}
    /\ UNCHANGED vars

Next ==
    \/ AdmissionReject
    \/ StorageFail
    \/ Ack
    \/ TerminalStutter

FailurePreventsAck ==
    code \in ErrorCodes => /\ ack = FALSE
                          /\ state # "Acked"

FailureEventuallyRejected ==
    state = "Pending" ~> state = "Rejected"

Spec ==
    /\ Init
    /\ [][Next]_vars
    /\ WF_vars(AdmissionReject)
    /\ WF_vars(StorageFail)

====
