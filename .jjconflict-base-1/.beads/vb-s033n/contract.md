**Contract vb-s033n: Validate Runtime Inputs Before Journal Admission**

**Domain:** cmd_submit input validation ordering.

**Boundary:** Validate after YAML compilation, before Fjall journal open and any DB write.

**Error:** On DecodeFailed, excess-slot, or kind mismatch from map_runtime_inputs, print InputMappingError Display via write_failure_message; return ValidationFailed.

**Acceptance:**
- Valid scalar positive input accepted with existing outputshape
- Legacy empty bytes and encoded empty values remain compatible
- Validation failures produce no journal entries, no DB writes
- Fresh DB: validation failure leaves DB absent
- Existing DB: validation failure leaves DB unchanged
- Parent durable submit requirements (persistent inputs, owned handle data, atomic recoverable admission) remain unsolved

**No new source or test behavior beyond input validation ordering.**