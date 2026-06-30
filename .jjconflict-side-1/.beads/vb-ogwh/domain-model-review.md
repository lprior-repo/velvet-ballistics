bead_id: vb-ogwh
phase: 3
updated_at: 2026-05-17T22:26:00Z

# Domain Model Review

The existing `ShardDirective` enum is adequate. The illegal state was an action-ordering bug in the Shutdown branch: a drain helper that requires a queued shutdown command was called without the command.
