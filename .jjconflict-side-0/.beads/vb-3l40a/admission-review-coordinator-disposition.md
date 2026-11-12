# Coordinator disposition

Independent local 3090 suite review rejected candidate `1b0fe1f8` after
actually executing 114 passing lifecycle tests. All listed test gaps remain
required repairs; no owner waiver was granted.

The code review approval is not accepted as complete: it claims global
duplicate/name/index precedence, but `validate_contract_ids` returns the
first per-entry index error before inspecting later duplicate IDs. The
addendum and implementation must agree and multi-error tests must establish
the documented precedence. The code review also confused fixture API changes
with the separate existing cfg(test) production replacements (`vb-8iilq`).

Next repair preserves both real with-contract ingress checks and exact
pending-ticket authority. Positive tests must prove RunFinished and actual
output values; negative tests must prove the full journal is unchanged.
