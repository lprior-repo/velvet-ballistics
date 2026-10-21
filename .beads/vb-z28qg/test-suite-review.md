# Independent atomic admission test review

STATUS: REJECTED for full bead acceptance; C6 recovery integration is pending.

Exact candidate9f0557e59ab55203cc94196d67395c3bb3efe92f; reviewed against its revision5 contract. Reviewer uses unique Git/JJ workspace and random live bead vb-z28qg; no production/source repair authored here.

## Required integration oracle (C6)

`crates/workspace_tests/tests/cli/submit_retained_inputs.rs:42` manually reads the retained record, validates it, creates a core RunFrame, seeds inputs and calls run_until_blocked. This is real production engine execution and a useful storage/CLI test, but it bypasses Runtime::recover_product/recover_and_resume. Runtime currently never reads run_inputs or validates its committed input_digest receipt. ContractC6 explicitly requires the recovery owner to supply this path; the evidence README honestly states it is missing. This is a pending cross-stack dependency, not a newly introduced admission-storage failure.

Before closing vb-ezymz, add an actual public Runtime/Fjall recovery test after a separate CLI submit process exits and original files are deleted. Journaled/Strict must consume the exact retained value and reach the correct terminal result through the actual runtime path. Missing record, replaced input digest, wrong run/artifact and LegacyUnbound receipt for a newly retained submission must reject before effects/journal mutation. None makes no durable-recovery claim and remains separately tested by direct numeric execution.

Disposition: blocker until runtime integration and this oracle are published and independently tested. No invented waiver or self-approved completion.

## Independent executions

All three full named commands exit0 with real subprocess return codes: storage inputs13passed/1599filtered; storage codec174passed/1438filtered; CLI boundary6passed and retainedexecution2passed. Total195passed, zero failures or ignored. Property cases are not counted as additional tests. Complete stdout/stderr and argv are in evidence/results.json and 0.log/1.log/2.log. No broadCI/proof or hardware-crash guarantee claimed.

## Covered behavior and limits

The tests bind typed values/slots/types, canonical order independence, raw source and artifact identity, corrupt/rehashed records versus committed receipts, new36/legacy24/ordinal1 decoding, precommit atomic failure and successful retry, collision nonreplacement, and exact CLI rejection before database creation. Close/reopen execution uses actual persisted numeric IR after deleting both input/source files. Atomic storage commit guarantee delegates to Fjall; the injected precommit failure is not a simulated power loss. The separately rejected artifact948f dependency must be replaced by independently accepted57f repairs and integrated evidence before delivery.
