---- MODULE vb_7m21_004 ----
EXTENDS Naturals, TLC

(* PO-vb-7m21-016: missing side-index event cannot be accepted silently. *)
VARIABLES phase, eventPresent, sideIndexPresent, outcome

Outcomes == {"Unknown", "Accepted", "IndexParityMismatch"}
Phases == {"Start", "Checked", "Done"}

Init ==
  /\ phase = "Start"
  /\ eventPresent \in BOOLEAN
  /\ sideIndexPresent \in BOOLEAN
  /\ outcome = "Unknown"

Check ==
  /\ phase = "Start"
  /\ phase' = "Checked"
  /\ UNCHANGED <<eventPresent, sideIndexPresent>>
  /\ outcome' = IF eventPresent /\ ~sideIndexPresent
                THEN "IndexParityMismatch"
                ELSE "Accepted"

Finish ==
  /\ phase = "Checked"
  /\ phase' = "Done"
  /\ UNCHANGED <<eventPresent, sideIndexPresent, outcome>>

Done ==
  /\ phase = "Done"
  /\ UNCHANGED <<phase, eventPresent, sideIndexPresent, outcome>>

Next == Check \/ Finish \/ Done
Spec == Init /\ [][Next]_<<phase, eventPresent, sideIndexPresent, outcome>>

TypeOK ==
  /\ phase \in Phases
  /\ eventPresent \in BOOLEAN
  /\ sideIndexPresent \in BOOLEAN
  /\ outcome \in Outcomes

MissingIndexRejected ==
  (phase \in {"Checked", "Done"} /\ eventPresent /\ ~sideIndexPresent) => outcome = "IndexParityMismatch"

NoSilentAcceptance ==
  (phase \in {"Checked", "Done"} /\ outcome = "Accepted") => ~(eventPresent /\ ~sideIndexPresent)
====
