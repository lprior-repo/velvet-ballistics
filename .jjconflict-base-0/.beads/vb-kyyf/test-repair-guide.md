# vb-kyyf State 9 Test Repair Guide — BDD-KYYF-002 Cap-Unblock

STATUS: APPROVED

No repair needed for the owner-authorized BDD-KYYF-002 CLI hardening sublane.

If this regresses, restore these mandatory properties before resubmission:
1. Close/drop/reopen the persisted store before CLI reads.
2. Run `replay`, `events`, and `inspect` twice through the public CLI.
3. Capture and compare exact `status_code`, `stdout`, and `stderr` reports.
4. Reject locked-writer text, `writer_lock_held`, and `events=0`.
5. Assert scenario id, command name, run id, evidence path, digest marker, `events=4`, seq `0..3`, and terminal/status facts.
