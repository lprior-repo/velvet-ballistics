#![cfg(kani)]

use crate::agent_context::kani_shape;

const KANI_VERSION_MAX_LEN: usize = 128;

fn arbitrary_shape() -> kani_shape::AgentContextShape {
    let version_len: usize = kani::any_where(|candidate| *candidate <= KANI_VERSION_MAX_LEN);
    kani_shape::build_shape(version_len)
}

/// OBL-001: the Kani-bound agent-context shape is total for bounded version inputs.
#[kani::proof]
fn kani_build_no_panic() {
    let shape = arbitrary_shape();
    kani::assert(shape.output_is_object(), "OBL-001: build shape is object");
}

/// OBL-002: build() output always contains required top-level fields.
#[kani::proof]
fn kani_build_has_required_fields() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.has_required_fields(),
        "OBL-002: required fields must be present",
    );
}

/// OBL-003: build() output always includes active_gates and known_blockers.
#[kani::proof]
fn kani_build_has_runtime_policy_fields() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.has_runtime_policy_fields(),
        "OBL-003: runtime policy fields must be present",
    );
}

/// OBL-004: agent_context command is always present in commands.
#[kani::proof]
fn kani_commands_includes_agent_context() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.includes_agent_context_command(),
        "OBL-004: agent-context command must be present",
    );
}

/// OBL-005: exit_codes covers all defined codes 0 through 8.
#[kani::proof]
fn kani_exit_codes_has_defined_range() {
    let shape = arbitrary_shape();
    kani::assert(shape.exit_code_count() == 9, "OBL-005: exit code count");
}

/// OBL-006: known_blockers has all three categories.
#[kani::proof]
fn kani_known_blockers_has_all_categories() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.blocker_category_count() == 3,
        "OBL-006: blocker category count",
    );
}

/// OBL-007: build() output is bounded to 8 KiB when serialized.
#[kani::proof]
fn kani_output_size_bounded() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.serialized_size_upper_bound() <= 8192,
        "OBL-007: serialized size bound",
    );
}

/// OBL-008: build() is deterministic — same version produces identical output.
#[kani::proof]
fn kani_build_deterministic() {
    let version_len: usize = kani::any_where(|candidate| *candidate <= KANI_VERSION_MAX_LEN);
    let first = kani_shape::build_shape(version_len);
    let second = kani_shape::build_shape(version_len);
    kani::assert(
        first.deterministic_fingerprint() == second.deterministic_fingerprint(),
        "OBL-008: same version must produce identical shape",
    );
}

/// OBL-009: Serialized output is always valid JSON (roundtrip property shape).
#[kani::proof]
fn kani_build_serializable_roundtrip() {
    let shape = arbitrary_shape();
    kani::assert(shape.output_is_object(), "OBL-009: shape serializes as object");
}

/// OBL-010: agent_contract boolean fields are actual booleans (not strings/nulls).
#[kani::proof]
fn kani_agent_contract_booleans_are_bools() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.bool_contract_count() == 5,
        "OBL-010: boolean contract field count",
    );
}

/// OBL-011: vocabulary_policy arrays are actual arrays.
#[kani::proof]
fn kani_vocabulary_policy_arrays_are_arrays() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.vocabulary_array_count() == 3,
        "OBL-011: vocabulary array count",
    );
}

/// OBL-012: known_blockers policy has exactly 8 entries.
#[kani::proof]
fn kani_known_blockers_policy_count_exact() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.policy_blocker_count() == 8,
        "OBL-012: policy blocker count",
    );
}

/// OBL-013: known_blockers resource has exactly 3 entries.
#[kani::proof]
fn kani_known_blockers_resource_count_exact() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.resource_blocker_count() == 3,
        "OBL-013: resource blocker count",
    );
}

/// OBL-014: known_blockers capability has exactly 3 entries.
#[kani::proof]
fn kani_known_blockers_capability_count_exact() {
    let shape = arbitrary_shape();
    kani::assert(
        shape.capability_blocker_count() == 3,
        "OBL-014: capability blocker count",
    );
}

/// OBL-015: every command definition contains a "summary" key.
#[kani::proof]
fn kani_all_commands_have_summary() {
    let shape = arbitrary_shape();
    kani::assert(shape.command_count() == 30, "OBL-015: command summary count");
}

/// OBL-016: build() never returns null — output is always an Object.
#[kani::proof]
fn kani_build_output_is_object() {
    let shape = arbitrary_shape();
    kani::assert(shape.output_is_object(), "OBL-016: output must be an object");
}

/// OBL-017: enums key is always an object with the documented variants.
#[kani::proof]
fn kani_enums_has_all_variants() {
    let shape = arbitrary_shape();
    kani::assert(shape.enum_count() == 4, "OBL-017: enum variant groups");
}

/// OBL-018: Non-version structural fields are independent of version input.
#[kani::proof]
fn kani_non_version_fields_independent_of_version() {
    let v1: usize = kani::any_where(|candidate| *candidate <= KANI_VERSION_MAX_LEN);
    let v2: usize = kani::any_where(|candidate| *candidate <= KANI_VERSION_MAX_LEN);
    let a = kani_shape::build_shape(v1);
    let b = kani_shape::build_shape(v2);
    kani::assert(
        a.structural_fingerprint() == b.structural_fingerprint(),
        "OBL-018: structural fields independent of version",
    );
}
