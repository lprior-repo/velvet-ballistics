---- MODULE admission_header_before_ack ----
EXTENDS TLC

CONSTANTS ErrorCodes, NoCode

VARIABLES state, code, ack, persisted, live_state, duplicate_run

vars == <<state, code, ack, persisted, live_state, duplicate_run>>

States == {"Pending", "Persisted", "Rejected", "Acked"}
CodeDomain == ErrorCodes \cup {NoCode}

Init ==
    /\ state = "Pending"
    /\ code \in CodeDomain
    /\ ack = FALSE
    /\ persisted = FALSE
    /\ live_state = FALSE
    /\ duplicate_run \in BOOLEAN

TypeOK ==
    /\ ErrorCodes # {}
    /\ state \in States
    /\ code \in CodeDomain
    /\ ack \in BOOLEAN
    /\ persisted \in BOOLEAN
    /\ live_state \in BOOLEAN
    /\ duplicate_run \in BOOLEAN

AdmissionReject ==
    /\ state = "Pending"
    /\ code \in ErrorCodes \/ duplicate_run
    /\ state' = "Rejected"
    /\ ack' = FALSE
    /\ persisted' = FALSE
    /\ live_state' = FALSE
    /\ UNCHANGED <<code, duplicate_run>>

StorageFail ==
    /\ state = "Pending"
    /\ code \in ErrorCodes
    /\ state' = "Rejected"
    /\ ack' = FALSE
    /\ persisted' = FALSE
    /\ live_state' = FALSE
    /\ UNCHANGED <<code, duplicate_run>>

PersistHeader ==
    /\ state = "Pending"
    /\ code = NoCode
    /\ ~duplicate_run
    /\ state' = "Persisted"
    /\ persisted' = TRUE
    /\ ack' = FALSE
    /\ live_state' = FALSE
    /\ UNCHANGED <<code, duplicate_run>>

Ack ==
    /\ state = "Persisted"
    /\ code = NoCode
    /\ state' = "Acked"
    /\ ack' = TRUE
    /\ persisted' = TRUE
    /\ live_state' = TRUE
    /\ UNCHANGED <<code, duplicate_run>>

TerminalStutter ==
    /\ state \in {"Rejected", "Acked"}
    /\ UNCHANGED vars

Next ==
    \/ AdmissionReject
    \/ StorageFail
    \/ PersistHeader
    \/ Ack
    \/ TerminalStutter

FailurePreventsAck ==
    code \in ErrorCodes => /\ ack = FALSE
                          /\ live_state = FALSE
                          /\ state # "Acked"

DuplicateRejectsNoLiveState ==
    duplicate_run => /\ ack = FALSE
                     /\ live_state = FALSE
                     /\ state # "Acked"

AckRequiresPersistence ==
    ack => /\ persisted
           /\ state = "Acked"

LiveStateRequiresPersistence ==
    live_state => /\ persisted
                  /\ ack
                  /\ state = "Acked"

NoLiveStateBeforeDurableAdmission ==
    ~persisted => /\ ~ack
                  /\ ~live_state

FailureEventuallyRejected ==
    (state = "Pending" /\ (code \in ErrorCodes \/ duplicate_run)) ~> state = "Rejected"

SuccessEventuallyAcked ==
    (state = "Pending" /\ code = NoCode /\ ~duplicate_run) ~> state = "Acked"

Spec ==
    /\ Init
    /\ [][Next]_vars
    /\ WF_vars(AdmissionReject)
    /\ WF_vars(StorageFail)
    /\ WF_vars(PersistHeader)
    /\ WF_vars(Ack)

(* Theorems: all invariants hold over all behaviors *)
THEOREM Spec => []TypeOK
THEOREM Spec => []FailurePreventsAck
THEOREM Spec => []DuplicateRejectsNoLiveState
THEOREM Spec => []AckRequiresPersistence
THEOREM Spec => []LiveStateRequiresPersistence
THEOREM Spec => []NoLiveStateBeforeDurableAdmission

====
