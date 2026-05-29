---- MODULE vb_7m21_005 ----
EXTENDS Naturals, TLC

(* PO-vb-7m21-021: bounded u64 sequence gaps and overflow become typed outcomes. *)
CONSTANT MAX_U64
VARIABLES phase, expected, actual, outcome

Outcomes == {"Unknown", "Accepted", "SequenceGap", "SequenceOverflow"}
Phases == {"Start", "Checked", "Done"}
Seqs == 0..MAX_U64

Init ==
  /\ phase = "Start"
  /\ expected \in Seqs
  /\ actual \in Seqs
  /\ outcome = "Unknown"

Check ==
  /\ phase = "Start"
  /\ phase' = "Checked"
  /\ UNCHANGED <<expected, actual>>
  /\ outcome' = IF expected = MAX_U64 THEN "SequenceOverflow"
                ELSE IF actual = expected THEN "Accepted"
                ELSE "SequenceGap"

Finish == /\ phase = "Checked" /\ phase' = "Done" /\ UNCHANGED <<expected, actual, outcome>>
Done == /\ phase = "Done" /\ UNCHANGED <<phase, expected, actual, outcome>>
Next == Check \/ Finish \/ Done
Spec == Init /\ [][Next]_<<phase, expected, actual, outcome>>

TypeOK == /\ phase \in Phases /\ expected \in Seqs /\ actual \in Seqs /\ outcome \in Outcomes
GapRejected == (phase \in {"Checked", "Done"} /\ expected # MAX_U64 /\ actual # expected) => outcome = "SequenceGap"
OverflowRejected == (phase \in {"Checked", "Done"} /\ expected = MAX_U64) => outcome = "SequenceOverflow"
====
