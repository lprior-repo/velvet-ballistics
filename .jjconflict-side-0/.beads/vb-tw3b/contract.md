bead_id: vb-tw3b
phase: 3

# Contract

REQ-001: Expression evaluation in bytecode/interpreter and generated Rust shall produce identical terminal `SlotValue` results for supported expression primitives.
REQ-002: Expression evaluation in both modes shall preserve taint on produced values.
REQ-003: Division-by-zero, integer overflow, and expression stack errors shall be surfaced as typed errors, not panics.
REQ-004: Closure is evidence-only: no dependency or runtime behavior changes are required if current merged tests prove the above scope.

STATUS: APPROVED
