# Independent ABI schedule regression review

STATUS: APPROVED (schedule boundary scope only)

Exact source candidate `0547383f200ea1f08fbaa4f263ddc295b8f61713`. The new public Runtime/Fjall test schedules an actual action, reads the persisted ActionScheduledTicket, checks its action and exact contract ABI digest, and separately rejects a zero digest. The pre-existing full recovery tests were not weakened. This test intentionally stops at scheduling; pending-action restoration and retained-input recovery remain separate incomplete work.

Independent positive execution passes one named test. A controlled source mutation changes only the scheduled digest expression back to zero: the same test then fails with actual cargo exit101 and unequal digest values. Source is restored byte for byte in finally and the same test passes again. The restored runtime library passes1914 tests, zero ignored, and both affected files pass exact-file rustfmt check. Complete logs, argv, true returncodes and restoration hash are archived in evidence/.

No open finding within this scheduling regression scope; it catches deletion of the digest wiring. This does not claim all20 recovery tests pass, missing-contract authority coverage is independently checked by the code reviewer, and vb-netau remains open until its full recovery contract is resolved.
