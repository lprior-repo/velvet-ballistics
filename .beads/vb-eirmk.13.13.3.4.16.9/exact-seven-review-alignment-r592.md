# Exact seven-case review alignment

Independent TEST report artifact-dispatch-implementation-test-r525.json requires the seven original case identities and their exact historical inputs to remain in place. Candidate 051031438d672b0406bdc5167cde5ebe2828a095 preserved the relevant domains through a case-map but moved two original inputs and renamed three reviewed identities; this successor corrects that mismatch in only compiled_ir_tests.rs and compiled_ir_version_tests.rs.

Restore v1_outer_version_1_2_0_accepted with its valid raw fixture and held1.2.0, asserting exact ArtifactVersionIncompatible held1.2.0/required2.0.0/Rejected. Keep the separate current2 raw positive. Historical names are retained as registered case identities, not statements that outer1 is currently authorized.

Keep version_matrix_matching_v1 at its original held1.0.0 with valid raw parts and exact held1.0.0/required2.0.0/Rejected. Restore version_matrix_same_major_1_2_0_accepted and version_matrix_v1_patch_accepted identities with their original held1.2.0/raw0 and held1.0.1/raw1 inputs, respectively, and exact typed rejection fields.

Retain outer_inner_disagreement_v1_with_v2_prefix with exact01ff5642 bytes and held1.0.0, asserting outer incompatibility before body parsing. Restore outer_inner_disagreement_v2_with_v1_bytes identity with its original raw parts and outer2, asserting full successful decode, canonical digest/parts and absent binding through the existing independent raw roundtrip helper.

For explicit_classifier_keeps_legacy_wrapper, change only the original stale CURRENT_IR_VERSION expected literal to2.0.0 while retaining held1.2.0 and exact wrapper-versus-explicit-current equality. All production authorization, compatibility, writer bytes, flags, schemas, public APIs and protected files remain unchanged.

Preserve candidate051's submission conflict repair, legitimate corrupt-escape no-write controls, separate raw/bound positives, and all original Fjall/owned-input scenarios. This is a source-review alignment only; blockerT2 remains unqualified until actual parent scoped codec/admission/public recovery and unchanged Fjall/owned scenarios execute against the imported source.
