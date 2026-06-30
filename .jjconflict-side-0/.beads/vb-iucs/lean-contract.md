# Lean Contract Note: vb-iucs

No Lean proof is claimed by this recovery.

The relevant recovered evidence set is Kani, Verus, and TLA+ only:

- Gate 8 accessor behavior: Kani.
- StepState runtime parity: Kani plus Verus mirror.
- BudgetArithmetic overflow/underflow: TLA+ TLC.

Any Lean obligation beyond this scope remains outside `vb-iucs` and must be filed as separate follow-up work if required.
