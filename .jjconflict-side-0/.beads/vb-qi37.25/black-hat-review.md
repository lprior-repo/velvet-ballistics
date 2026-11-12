bead_id: vb-qi37.25
bead_title: quality: Workspace assertion sharpness and spelling gates
phase: 12
updated_at: 2026-05-18T14:40:00Z
attempt: 1-of-7
STATUS: APPROVED

Black-hat verdict: APPROVED.

Contract parity:
- R1 workspace membership exactness is enforced by check-workspace-assertions.py and negative fixture tests.
- R2 package/crate names are exact, including expected/actual failure text.
- R3 binary name set is exactly ["velvet-ballistics"].
- R4 feature gate checks reject exact drift and forbidden feature names.
- R5 forbidden dependency checks cover direct, package alias, and path alias cases.
- R6 canonical spelling tests reject invalid velvet-ballistics outside exact allowlist and reject broad substring allowlists.

Mutation resistance:
- New tests assert exact stderr or exact NamingScanError/NamingFinding values, not broad success substrings.
- Existing vb_37lc canonical spelling suite still passes (76 tests).

Execution evidence consumed:
- machine-gate-report.md STATUS: PASS.
- formal-verification-report.md STATUS: APPROVED.
- rtk moon ci PASS, 23 completed.

No defects requiring reroute were found.
