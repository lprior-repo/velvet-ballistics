bead_id: vb-tw3b
phase: 7

# Test plan

- TP-001: Run `generated_expression_primitives_match_interpreter_finish` for binary/operator generated-vs-interpreter result parity.
- TP-002: Run `expression_generated_parity` filter for append/merge value-order and taint parity.
- TP-003: Run `post_011_generated_finished_value_taint_and_journal_match_ir_for_expression` for finished output taint+journal parity.
- TP-004: Run `generated_drive_error_covers_all_step_error_paths` for typed error variant coverage, including division-by-zero, integer overflow, and stack errors.

STATUS: APPROVED
