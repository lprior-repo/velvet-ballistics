# Review scope and disposition

The local 5090 independently reviewed local 3090 candidate
`55c79301e5da189bab96adfeb7f97c46cf60f923` and actually reran all ten
finish-digest tests successfully. Both scoped reviews approved the repair.
The author's full compiler library log records 489 passing, zero ignored.

Acceptance is limited to restoring executable digest regression properties
and exact typed slot-boundary tests. There is no compiler production change.
The source digest is tested through the actual compiler API.

Calibration of wording in the historical header and review:

- Proptest is finite randomized regression evidence, not exhaustive proof
  or formal proof-obligation closure.
- The legacy `finish_position_change_changes_digest` name tests step-ID
  sensitivity; it does not test rearranging source positions. The contract
  wording is clarified accordingly, without changing behavior or assertions.
- The invalid generator is the half-open range `[1024, 65535)`, including
  65534. Its samples do not exhaust every member of that range.
- Mutation statements are reviewer thought experiments, not mutation-tool
  execution evidence. No coverage percentage or full-v1 acceptance follows.

Compiler/runtime integration compilation is a separate fixture repair
`vb-bwdf0`. Full canonical CI remains open under `vb-xx331`.
