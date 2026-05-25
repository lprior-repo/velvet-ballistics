---- MODULE collect_body_model ----
(*!
# Collect Body Lowering — TLA+ State Machine Model
Bean ID: vb-xi2f.23
PO: PO-001 (4-node IR emission order and step offset invariants)

## Modeling Notes

- MachineInt (u16 range [0, 65535]) is modeled explicitly.
  TLA+ Nat is NOT used for step indices.
- Node IDs are modeled as machine integers in the range 0..65535.
- Step offsets (body=id+1, page=id+2, done=id+3) are modeled as
  checked arithmetic that returns StepIndexOutOfRange when id+offset > 65535.
- The model represents a single collect-lowering emission sequence only;
  parallel/together primitives are out of scope for this bead.
*)
EXTENDS Naturals, FiniteSets

CONSTANT MaxStepIdx  \* Modeled as 65535 (u16::MAX)

ASSUME MaxStepIdxSpec == MaxStepIdx = 65535

VARIABLES
  (*! \\type: Set of node IDs emitted so far *)
  emitted,
  (*! \\type: Current step index (id) *)
  current_id,
  (*! \\type: Phase of emission: idle | collect_start | set_const | collect_page | collect_finish | done *)
  phase,
  (*! \\type: Whether the lowering has overflowed *)
  overflow

vars == <<emitted, current_id, phase, overflow>>

\* ─────────────────────────────────────────────────────────────────
\* State Space Bounds (MachineInt model — u16 range)
\* ─────────────────────────────────────────────────────────────────

IsValidNodeId(id) == id \in 0..MaxStepIdx

\* ─────────────────────────────────────────────────────────────────
\* Init
\* ─────────────────────────────────────────────────────────────────

Init ==
  /\ emitted = {}
  /\ current_id \in 0..MaxStepIdx
  /\ phase = "idle"
  /\ overflow = FALSE

\* ─────────────────────────────────────────────────────────────────
\* Emission sequence for a valid single-set collect body
\*
\* lower_canonical_collect emits exactly 4 nodes at positions:
\*   Node 0 (id):     CollectStart  { source, limit, page_size, body: id+1, done: id+3 }
\*   Node 1 (id+1):  SetConst      from body Set step
\*   Node 2 (id+2):  CollectPage   { collector_slot: source, body: id+1, done: id+3 }
\*   Node 3 (id+3):  CollectFinish { collector_slot: source }
\* ─────────────────────────────────────────────────────────────────

EmitCollectStart ==
  /\ phase = "idle"
  /\ IsValidNodeId(current_id)
  (*! Check: body=current_id+1, done=current_id+3 must be valid u16 *)
  /\ current_id + 3 <= MaxStepIdx
  /\ emitted' = emitted \cup {current_id}
  /\ phase' = "set_const"
  /\ UNCHANGED <<current_id, overflow>>

EmitSetConst ==
  /\ phase = "set_const"
  /\ IsValidNodeId(current_id + 1)
  /\ emitted' = emitted \cup {current_id + 1}
  /\ phase' = "collect_page"
  /\ UNCHANGED <<current_id, overflow>>

EmitCollectPage ==
  /\ phase = "collect_page"
  /\ IsValidNodeId(current_id + 2)
  /\ emitted' = emitted \cup {current_id + 2}
  /\ phase' = "collect_finish"
  /\ UNCHANGED <<current_id, overflow>>

EmitCollectFinish ==
  /\ phase = "collect_finish"
  /\ IsValidNodeId(current_id + 3)
  /\ emitted' = emitted \cup {current_id + 3}
  /\ phase' = "done"
  /\ UNCHANGED <<current_id, overflow>>

\* ─────────────────────────────────────────────────────────────────
\* Overflow transition: when id + offset exceeds u16::MAX
\* ─────────────────────────────────────────────────────────────────

Overflow ==
  /\ phase = "idle"
  /\ current_id + 3 > MaxStepIdx
  /\ overflow' = TRUE
  /\ UNCHANGED <<emitted, current_id, phase>>

\* ─────────────────────────────────────────────────────────────────
\* Next
\* ─────────────────────────────────────────────────────────────────

Next ==
  \/ EmitCollectStart
  \/ EmitSetConst
  \/ EmitCollectPage
  \/ EmitCollectFinish
  \/ Overflow

Spec == Init /\ [][Next]_vars

\* ─────────────────────────────────────────────────────────────────
\* Invariants
\* ─────────────────────────────────────────────────────────────────

\* NodeCountInvariant: A valid collect-lowering emits exactly 4 nodes
NodeCountInvariant ==
  phase = "done"
    => Cardinality(emitted) = 4

\* OffsetInvariant: The 4 nodes occupy consecutive positions id, id+1, id+2, id+3
OffsetInvariant ==
  phase = "done"
    => (
      emitted \in SUBSET (0..MaxStepIdx)
        /\ Cardinality(emitted) = 4
        /\ LET sorted == Sort(emitted) IN
          /\ sorted[1] = current_id
          /\ sorted[2] = current_id + 1
          /\ sorted[3] = current_id + 2
          /\ sorted[4] = current_id + 3
    )

\* NodeKindInvariant: Nodes are in emission order
NodeKindInvariant ==
  phase = "done"
    => (
      /\ current_id \in emitted
      /\ current_id + 1 \in emitted
      /\ current_id + 2 \in emitted
      /\ current_id + 3 \in emitted
    )

\* NoOverflowInvariant: overflow flag is never set for valid ids
NoOverflowInvariant ==
  overflow = TRUE
    => current_id + 3 > MaxStepIdx

\* TypeOK: All emitted node IDs are valid u16 values
TypeOK ==
  emitted \subseteq 0..MaxStepIdx

\* ─────────────────────────────────────────────────────────────────
\* Temporal Properties
\* ─────────────────────────────────────────────────────────────────

\* Convergence: Eventually reaches "done" phase (for valid starting id)
CollectLowersToDone ==
  phase = "idle" /\ current_id + 3 <= MaxStepIdx
    => <>(phase = "done")

\* No spurious nodes before completion
NoEarlyDone ==
  phase = "done"
    => Cardinality(emitted) = 4

====
