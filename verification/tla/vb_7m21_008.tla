---- MODULE vb_7m21_008 ----
EXTENDS Naturals, TLC

(* PO-vb-7m21-035: missing declared keyspace/manifest parity is an exact typed outcome. *)
VARIABLES phase, declared, present, outcome

Outcomes == {"Unknown", "Accepted", "MissingManifestKeyspace"}
Phases == {"Start", "Checked", "Done"}
Keyspaces == 0..3

Init ==
  /\ phase = "Start"
  /\ declared \in SUBSET Keyspaces
  /\ present \in SUBSET Keyspaces
  /\ outcome = "Unknown"

Missing == declared \ present

Check ==
  /\ phase = "Start"
  /\ phase' = "Checked"
  /\ UNCHANGED <<declared, present>>
  /\ outcome' = IF Missing = {} THEN "Accepted" ELSE "MissingManifestKeyspace"

Finish == /\ phase = "Checked" /\ phase' = "Done" /\ UNCHANGED <<declared, present, outcome>>
Done == /\ phase = "Done" /\ UNCHANGED <<phase, declared, present, outcome>>
Next == Check \/ Finish \/ Done
Spec == Init /\ [][Next]_<<phase, declared, present, outcome>>

TypeOK == /\ phase \in Phases /\ declared \in SUBSET Keyspaces /\ present \in SUBSET Keyspaces /\ outcome \in Outcomes
MissingManifestRejected == (phase \in {"Checked", "Done"} /\ Missing # {}) => outcome = "MissingManifestKeyspace"
NoAdHocOutcome == phase \in {"Start", "Checked", "Done"} => outcome \in Outcomes
====
