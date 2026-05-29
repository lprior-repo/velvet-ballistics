---- MODULE vb_7m21_006 ----
EXTENDS Naturals, TLC

(* PO-vb-7m21-026: divergent duplicate events are rejected; identical duplicates are explicit idempotent success. *)
VARIABLES phase, existing, incoming, sameDigest, outcome

Outcomes == {"Unknown", "Accepted", "DuplicateEvent", "IdempotentDuplicate"}
Phases == {"Start", "Checked", "Done"}
EventIds == 0..3

Init ==
  /\ phase = "Start"
  /\ existing \in EventIds
  /\ incoming \in EventIds
  /\ sameDigest \in BOOLEAN
  /\ outcome = "Unknown"

Check ==
  /\ phase = "Start"
  /\ phase' = "Checked"
  /\ UNCHANGED <<existing, incoming, sameDigest>>
  /\ outcome' = IF existing = incoming /\ sameDigest THEN "IdempotentDuplicate"
                ELSE IF existing = incoming /\ ~sameDigest THEN "DuplicateEvent"
                ELSE "Accepted"

Finish == /\ phase = "Checked" /\ phase' = "Done" /\ UNCHANGED <<existing, incoming, sameDigest, outcome>>
Done == /\ phase = "Done" /\ UNCHANGED <<phase, existing, incoming, sameDigest, outcome>>
Next == Check \/ Finish \/ Done
Spec == Init /\ [][Next]_<<phase, existing, incoming, sameDigest, outcome>>

TypeOK == /\ phase \in Phases /\ existing \in EventIds /\ incoming \in EventIds /\ sameDigest \in BOOLEAN /\ outcome \in Outcomes
DivergentDuplicateRejected == (phase \in {"Checked", "Done"} /\ existing = incoming /\ ~sameDigest) => outcome = "DuplicateEvent"
IdenticalDuplicateExplicit == (phase \in {"Checked", "Done"} /\ existing = incoming /\ sameDigest) => outcome = "IdempotentDuplicate"
====
