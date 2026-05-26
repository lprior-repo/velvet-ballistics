bead_id: vb-qi37.25
phase: 3
STATUS: APPROVED

Requirements:
R1 workspace.members equals the canonical member set; missing/extra entries fail.
R2 member package.name values match exact canonical crate/package names.
R3 product Cargo binary set is exactly ["velvet-ballistics"].
R4 checked feature sets for vb_core, vb_validate, vb_ui_snapshot are exact; forbidden legacy/json feature names fail.
R5 boundary crates reject UI and runtime-format dependencies including package/path aliases.
R6 canonical spelling scan rejects velvet-ballistics, velvet_ballistics, and velvet-ballistics/v1 outside exact allowlist/migration references.
