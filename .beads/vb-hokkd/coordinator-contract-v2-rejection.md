# Coordinator source audit of contract V2

Candidate V2 remains unaccepted. All eleven source SHA256 entries verified against current files; this is useful source inventory, not acceptance of the prose or signatures.

Actual RegisteredActionContext.output is ActionOutputReady with value, taint, output_slot and encoded_len; there is no output.encoded field. registered_action_context computes postcard bytes locally only for encoded_len, then drops them. New extraction must explicitly carry or serialize the actual validated SlotValue and actual read taint. Do not use a nonexistent field.

Actual per-run policy pin is supplied by admission child vb-h7rex. Re-loading an artifact at dispatch and turning its policy_digest into the admission pin ignores the retained pin and is not accepted. New registered context must carry the existing admitted pin, reject Unverified for new live authority, and preserve independent artifact identity checks.

Actual RuntimeJournal::append_sequenced requires BOTH event and EventSeq. Actual Shard::append_journal_event allocates/advances per-run sequence. Contract self.journal_event_for(run) does not exist. Proposed configure/activate APIs returning () cannot report durable storage failures and must not acknowledge failed writes.

DecisionRecord has nine top-level fields; it does not have SourceSpanId and does not have eleven fields. Source map must distinguish proposed APIs from existing signatures. Full tag38 storage candidate is c74c3102; V7/V9 await final canonical formatter/review, not already accepted on main.

Emergency control scope must be durable and stable across object recreation. Runtime-instance-only wording lacks a persistent identity. A dedicated Fjall journal-store scoped monotonic marker is tracked by vb-ct31x. It has no clear API and cannot be replaced by a mutable boolean/config setter. Configuration need not silently restore Allow on restart; default Unconfigured denies until an explicit trusted configuration. Durable policy decisions remain per-run facts, separate from the store-scoped deny marker.

Historical reconciliation must preserve catalog.require_receipt_recovery and actual durable adapter receipts, and must never create an effect. Existing catalog/grant/input validation stays in force. This audit does not close live dispatch, durable controls or full grant lifecycle.
