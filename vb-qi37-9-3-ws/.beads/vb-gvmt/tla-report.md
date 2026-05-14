# TLA+ Evidence: vb-gvmt

## Command

```bash
tlc -config ".beads/vb-gvmt/specs/GeneratedParity.cfg" ".beads/vb-gvmt/specs/GeneratedParity.tla"
```

## Result

- Status: PASS
- Tool: TLC2 Version 2.19
- Observed evidence: `Model checking completed. No error has been found.`
- State count: 17 states generated, 13 distinct states, 0 states left on queue
- Search depth: 4
- Temporal checks: 3 temporal property branches checked

## Scope

The model covers generated lifecycle phases for deterministic slot write, valid Do suspension/resume, valid Ask suspension/resume, terminal finish, budget/capacity error transitions, journal order invariants, trace parity abstraction, and terminal/error stuttering.

Invalid-resume no-mutation and concrete journal no-drop behavior are not claimed from this TLA+ model revision; those obligations are covered by Verus/Kani/Rust tests and recorded separately.

The executable cfg intentionally omits the module's trivial traceability placeholders (`SlotTaintParallel`, `JournalAppendOnly`, `NoMutationOnInvalidResume`, `NoDropOnJournalFull`) so the TLC pass is not misrepresented as proof of those obligations.
