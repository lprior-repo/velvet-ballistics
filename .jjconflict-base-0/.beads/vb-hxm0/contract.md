bead_id: vb-hxm0
phase: 3
attempt: 1-of-7

STATUS: APPROVED

Requirements:
REQ-1 The system shall expose a master acceptance catalog as executable data reachable through a public Rust API.
REQ-2 Each scenario shall include scenario id, Given, When, Then, public surface, fixture, expected outcome/error, durability profile, related bead, and test target.
REQ-3 The catalog gate shall reject empty catalogs, missing Given/When/Then, missing exact assertions, missing test targets, private/helper primary surfaces, shared fixtures, and duplicate ids.
REQ-4 Existing coverage shall map to concrete test files; missing coverage shall map to follow-up beads.

Invariants:
- Fixtures are isolated per scenario.
- No private helper is the primary behavior surface.
- At least one exact outcome or exact error is required.
