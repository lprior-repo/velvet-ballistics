# Terminal summary identity

The existing expected-terminal summary query compares the entire `RecoveryTerminalState`, including the Finished result slot. Matching payloads succeed; differing payloads, terminal classes or absent terminal state return the existing TerminalStateMismatch with payload-distinguishing diagnostic strings. Existing no-data and journal-read errors retain precedence. No new live recovery authority, API, schema, codec, dependencies or persisted format.

Public regression scenarios use real durable RunAccepted/RunFinished events for a summary-only query, positive and negative expectations before and after reopen, exact error fields, preserved encoded journal bytes, no-terminal and missing-run controls. MASTER §40/§67 and reconciliation bead .34 govern this narrow repair; the broader checkpoint authority owner remains open.
