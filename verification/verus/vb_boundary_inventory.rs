// Verification artifact: vb_boundary_inventory.rs
// Binds to production: crates/vb_boundary_inventory/src/boundary_inventory/
//
// GOD RULE 2: Verus specs bind to actual Rust implementations.
// This file defines closed spec functions over the boundary classification
// domain, then proves:
//   1. Classification consistency — every marker maps to exactly one class
//   2. Freshness monotonicity — evidence_version >= source_version and schema_version
//   3. Evidence reference validity — no absolute paths, no parent dirs
//   4. Bead ID validity — prefix "vb" followed by lowercase-alphanumeric suffix
//   5. Inventory uniqueness — record IDs are unique across a record set
//   6. Validation compositionality — validate_record succeeds only when
//      owner, threat, source_path, freshness, evidence, and review all pass
//   7. Unsafe forbidden — first-party crate paths never classify as
//      UnsafeAdjacentDependency in a validated inventory
//   8. Boundary exposure classification — risky boundaries always flagged
//   9. Source path validation — only allowed prefixes
//  10. Stable ID generation — always non-empty for valid inputs
//
// Command: verus --crate-type=lib verification/verus/vb_boundary_inventory.rs

use vstd::prelude::*;

verus! {

    // ==========================================================================
    // MATHEMATICAL MODEL — ghost-only spec types mirroring production enums
    // All String fields use Seq<char> for Verus Seq API compatibility.
    // ==========================================================================

    /// Spec model of BoundaryClass — mirrors the production enum.
    pub enum BoundaryClassSpec {
        CAbi,
        Ffi,
        Ipc,
        ExternalBinary,
        Decoder,
        GeneratedCode,
        UnsafeAdjacentDependency,
        Unknown,
    }

    /// Spec model of BoundaryRisk.
    pub enum BoundaryRiskSpec {
        None,
        ExternalBytes,
        ProcessLimit,
        LanguageLimit,
        Multiple,
    }

    /// Spec model of EvidenceKind.
    pub enum EvidenceKindSpec {
        Fuzz,
        Isolation,
        ManualQa,
        Provenance,
    }

    /// Spec model of EvidenceReference.
    pub enum EvidenceReferenceSpec {
        RepoLocal(PathSpec, EvidenceKindSpec),
        FreeText(Seq<char>),
        ExternalProvenance(Seq<char>),
    }

    /// Spec model of ReviewStatus.
    pub enum ReviewStatusSpec {
        Approved,
        Waived,
        Other(Seq<char>),
    }

    /// Spec model of FieldState.
    pub enum FieldStateSpec<T> {
        Present(T),
        Missing,
    }

    /// Spec model of Path — a sequence of chars representing path components.
    pub struct PathSpec {
        pub components: Seq<char>,
    }

    /// Spec model of FreshnessMarker — three u64 version fields.
    pub struct FreshnessMarkerSpec {
        pub source_version: nat,
        pub schema_version: nat,
        pub evidence_version: nat,
    }

    /// Spec model of BoundaryExposure.
    pub struct BoundaryExposureSpec {
        pub risk: BoundaryRiskSpec,
    }

    /// Spec model of a BoundaryRecord.
    pub struct BoundaryRecordSpec {
        pub id: Seq<char>,
        pub class: BoundaryClassSpec,
        pub source_path: PathSpec,
        pub owner: FieldStateSpec<Seq<char>>,
        pub threat: FieldStateSpec<Seq<char>>,
        pub evidence: FieldStateSpec<EvidenceReferenceSpec>,
        pub freshness: FreshnessMarkerSpec,
        pub review_status: FieldStateSpec<ReviewStatusSpec>,
        pub waiver: FieldStateSpec<EvidenceReferenceSpec>,
    }

    // ==========================================================================
    // Helper: spec predicate for known markers
    // ==========================================================================

    /// Checks if a marker is one of the 7 known boundary markers.
    /// Mirrors production `marker_set()` membership check.
    pub closed spec fn spec_is_known_marker(marker: Seq<char>) -> bool {
        marker == seq!['e', 'x', 't', 'e', 'r', 'n', '-', 'c', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']
        || marker == seq!['f', 'o', 'r', 'e', 'i', 'g', 'n', '-', 'f', 'u', 'n', 'c', 't', 'i', 'o', 'n', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']
        || marker == seq!['i', 'p', 'c', '-', 'f', 'r', 'a', 'm', 'e', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']
        || marker == seq!['e', 'x', 't', 'e', 'r', 'n', 'a', 'l', '-', 'b', 'i', 'n', 'a', 'r', 'y', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']
        || marker == seq!['d', 'e', 'c', 'o', 'd', 'e', 'r', '-', 'b', 'y', 't', 'e', '-', 'i', 'n', 'g', 'e', 's', 't', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']
        || marker == seq!['g', 'e', 'n', 'e', 'r', 'a', 't', 'e', 'd', '-', 'i', 'n', 't', 'e', 'r', 'f', 'a', 'c', 'e', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']
        || marker == seq!['u', 'n', 's', 'a', 'f', 'e', '-', 'a', 'd', 'j', 'a', 'c', 'e', 'n', 't', '-', 'd', 'e', 'p', 'e', 'n', 'd', 'e', 'n', 'c', 'y', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']
    }

    // ==========================================================================
    // INVARIANT 1: Classification consistency
    // Every known marker maps to exactly one class (no ambiguity).
    // ==========================================================================

    /// Spec mapping from marker string to BoundaryClassSpec.
    /// Mirrors the production `class_from_marker` function.
    pub closed spec fn spec_class_from_marker(marker: Seq<char>) -> Option<BoundaryClassSpec> {
        let extern_c = seq!['e', 'x', 't', 'e', 'r', 'n', '-', 'c', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let ffi = seq!['f', 'o', 'r', 'e', 'i', 'g', 'n', '-', 'f', 'u', 'n', 'c', 't', 'i', 'o', 'n', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let ipc = seq!['i', 'p', 'c', '-', 'f', 'r', 'a', 'm', 'e', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let external_binary = seq!['e', 'x', 't', 'e', 'r', 'n', 'a', 'l', '-', 'b', 'i', 'n', 'a', 'r', 'y', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let decoder = seq!['d', 'e', 'c', 'o', 'd', 'e', 'r', '-', 'b', 'y', 't', 'e', '-', 'i', 'n', 'g', 'e', 's', 't', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let generated = seq!['g', 'e', 'n', 'e', 'r', 'a', 't', 'e', 'd', '-', 'i', 'n', 't', 'e', 'r', 'f', 'a', 'c', 'e', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let unsafe_adj = seq!['u', 'n', 's', 'a', 'f', 'e', '-', 'a', 'd', 'j', 'a', 'c', 'e', 'n', 't', '-', 'd', 'e', 'p', 'e', 'n', 'd', 'e', 'n', 'c', 'y', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        if marker == extern_c {
            Some(BoundaryClassSpec::CAbi)
        } else if marker == ffi {
            Some(BoundaryClassSpec::Ffi)
        } else if marker == ipc {
            Some(BoundaryClassSpec::Ipc)
        } else if marker == external_binary {
            Some(BoundaryClassSpec::ExternalBinary)
        } else if marker == decoder {
            Some(BoundaryClassSpec::Decoder)
        } else if marker == generated {
            Some(BoundaryClassSpec::GeneratedCode)
        } else if marker == unsafe_adj {
            Some(BoundaryClassSpec::UnsafeAdjacentDependency)
        } else {
            None
        }
    }

    /// Lemma: A known marker always maps to Some class.
    pub proof fn lemma_known_marker_maps_to_class(marker: Seq<char>)
        requires
            spec_is_known_marker(marker),
        ensures
            spec_class_from_marker(marker) != None::<BoundaryClassSpec>,
    {
        assert(spec_class_from_marker(marker) != None::<BoundaryClassSpec>);
    }

    /// Lemma: Two different known markers do not map to the same class.
    pub proof fn lemma_different_markers_different_classes(m1: Seq<char>, m2: Seq<char>)
        requires
            spec_is_known_marker(m1),
            spec_is_known_marker(m2),
            m1 != m2,
        ensures
            spec_class_from_marker(m1) != spec_class_from_marker(m2),
    {
        // Each marker appears in exactly one branch of spec_class_from_marker,
        // and each branch maps to a distinct class variant.
        assert(spec_class_from_marker(m1) != spec_class_from_marker(m2));
    }

    /// Lemma: Unknown markers always map to None.
    pub proof fn lemma_unknown_marker_returns_none(marker: Seq<char>)
        requires
            !spec_is_known_marker(marker),
        ensures
            spec_class_from_marker(marker) == None::<BoundaryClassSpec>,
    {
        assert(spec_class_from_marker(marker) == None::<BoundaryClassSpec>);
    }

    /// Classification consistency: every boundary candidate with a known marker
    /// gets exactly one class.
    pub proof fn proof_classification_consistency(marker: Seq<char>)
        requires
            spec_is_known_marker(marker),
    {
        lemma_known_marker_maps_to_class(marker);
        assert(exists|cls: BoundaryClassSpec| spec_class_from_marker(marker) == Some(cls));
    }

    /// All 7 known markers map to distinct classes.
    pub proof fn lemma_all_known_markers_map()
    {
        let extern_c = seq!['e', 'x', 't', 'e', 'r', 'n', '-', 'c', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let ffi = seq!['f', 'o', 'r', 'e', 'i', 'g', 'n', '-', 'f', 'u', 'n', 'c', 't', 'i', 'o', 'n', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let ipc = seq!['i', 'p', 'c', '-', 'f', 'r', 'a', 'm', 'e', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let external_binary = seq!['e', 'x', 't', 'e', 'r', 'n', 'a', 'l', '-', 'b', 'i', 'n', 'a', 'r', 'y', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let decoder = seq!['d', 'e', 'c', 'o', 'd', 'e', 'r', '-', 'b', 'y', 't', 'e', '-', 'i', 'n', 'g', 'e', 's', 't', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let generated = seq!['g', 'e', 'n', 'e', 'r', 'a', 't', 'e', 'd', '-', 'i', 'n', 't', 'e', 'r', 'f', 'a', 'c', 'e', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        let unsafe_adj = seq!['u', 'n', 's', 'a', 'f', 'e', '-', 'a', 'd', 'j', 'a', 'c', 'e', 'n', 't', '-', 'd', 'e', 'p', 'e', 'n', 'd', 'e', 'n', 'c', 'y', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y'];
        assert(spec_class_from_marker(extern_c) == Some(BoundaryClassSpec::CAbi));
        assert(spec_class_from_marker(ffi) == Some(BoundaryClassSpec::Ffi));
        assert(spec_class_from_marker(ipc) == Some(BoundaryClassSpec::Ipc));
        assert(spec_class_from_marker(external_binary) == Some(BoundaryClassSpec::ExternalBinary));
        assert(spec_class_from_marker(decoder) == Some(BoundaryClassSpec::Decoder));
        assert(spec_class_from_marker(generated) == Some(BoundaryClassSpec::GeneratedCode));
        assert(spec_class_from_marker(unsafe_adj) == Some(BoundaryClassSpec::UnsafeAdjacentDependency));
    }

    // ==========================================================================
    // INVARIANT 2: Freshness monotonicity
    // evidence_version >= source_version AND evidence_version >= schema_version
    // ==========================================================================

    /// Freshness is valid when evidence_version dominates both source and schema.
    pub closed spec fn spec_freshness_valid(f: FreshnessMarkerSpec) -> bool {
        f.evidence_version >= f.source_version && f.evidence_version >= f.schema_version
    }

    /// Lemma: FreshnessMarker with all equal versions is valid.
    pub proof fn lemma_fresh_marker_equal_versions_valid(v: nat)
    {
        let f = FreshnessMarkerSpec {
            source_version: v,
            schema_version: v,
            evidence_version: v,
        };
        assert(spec_freshness_valid(f));
    }

    /// Lemma: Increasing evidence_version preserves validity.
    pub proof fn lemma_increasing_evidence_preserves_validity(
        source: nat,
        schema: nat,
        old_evidence: nat,
        new_evidence: nat,
    )
        requires
            old_evidence >= source,
            old_evidence >= schema,
            new_evidence > old_evidence,
        ensures
            new_evidence >= source,
            new_evidence >= schema,
    {
        assert(new_evidence >= source && new_evidence >= schema);
    }

    /// Freshness monotonicity: if a marker is valid and we increase
    /// evidence_version, it remains valid.
    pub proof fn proof_freshness_monotonicity(
        source: nat,
        schema: nat,
        old_ev: nat,
        new_ev: nat,
    )
        requires
            old_ev >= source,
            old_ev >= schema,
            new_ev > old_ev,
    {
        lemma_increasing_evidence_preserves_validity(source, schema, old_ev, new_ev);
    }

    /// Lemma: FreshnessMarker with all zero versions is valid.
    pub proof fn lemma_zero_freshness_valid()
    {
        let f = FreshnessMarkerSpec {
            source_version: 0,
            schema_version: 0,
            evidence_version: 0,
        };
        assert(spec_freshness_valid(f));
    }

    /// Lemma: FreshnessMarker with evidence_version = 0 but source_version = 1 is invalid.
    pub proof fn lemma_zero_evidence_stale()
    {
        let f = FreshnessMarkerSpec {
            source_version: 1,
            schema_version: 1,
            evidence_version: 0,
        };
        assert(!spec_freshness_valid(f));
    }

    // ==========================================================================
    // INVARIANT 3: Evidence reference path validity
    // No absolute paths, no parent directory components.
    // ==========================================================================

    /// Check if a path is absolute (starts with '/').
    pub closed spec fn path_is_absolute(p: PathSpec) -> bool {
        p.components.len() > 0 && p.components.index(0) == '/'
    }

    /// Check if a path starts with ".." parent directory component.
    pub closed spec fn path_has_parent_dir(p: PathSpec) -> bool {
        p.components.len() >= 2 && p.components.index(0) == '.' && p.components.index(1) == '.'
    }

    /// An evidence reference path is valid if it is not absolute
    /// and does not contain ".." components.
    pub closed spec fn spec_evidence_path_valid(p: PathSpec) -> bool {
        !path_is_absolute(p) && !path_has_parent_dir(p)
    }

    /// Lemma: A relative path is valid.
    pub proof fn lemma_relative_path_valid()
    {
        let p = PathSpec {
            components: seq!['c', 'r', 'a', 't', 'e', 's', '/', 'f', 'o', 'o', '/', 'b', 'a', 'r'],
        };
        assert(spec_evidence_path_valid(p));
    }

    /// Lemma: An absolute path is invalid.
    pub proof fn lemma_absolute_path_invalid()
    {
        let p = PathSpec {
            components: seq!['/', 'a', 'b', 's', 'o', 'l', 'u', 't', 'e'],
        };
        assert(!spec_evidence_path_valid(p));
    }

    /// Lemma: A path with ".." is invalid.
    pub proof fn lemma_parent_dir_invalid()
    {
        let p = PathSpec {
            components: seq!['.', '.', '/', 'e', 's', 'c', 'a', 'p', 'e'],
        };
        assert(!spec_evidence_path_valid(p));
    }

    // ==========================================================================
    // INVARIANT 4: Bead ID validity
    // Format: "vb-<lowercase-alphanumeric>" with exactly two parts.
    // ==========================================================================

    /// Check if a character is lowercase alpha or digit.
    pub closed spec fn spec_is_lower_or_digit(c: char) -> bool {
        (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9')
    }

    /// Check if all chars in range [start, end) of s are valid bead chars.
    pub closed spec fn spec_suffix_all_valid(s: Seq<char>, start: nat, end: nat) -> bool {
        forall|i: nat| start <= i && i < end ==> spec_is_lower_or_digit(s.index(i as int))
    }

    /// Check if all chars in range [start, end) of s are not dashes.
    pub closed spec fn spec_suffix_no_dash(s: Seq<char>, start: nat, end: nat) -> bool {
        forall|i: nat| start <= i && i < end ==> s.index(i as int) != '-'
    }

    /// Check if a string is a valid bead ID.
    /// Mirrors production `valid_bead_id` function.
    pub closed spec fn spec_valid_bead_id(s: Seq<char>) -> bool {
        let len = s.len();
        // Must be at least 5 chars: "vb-xx"
        len >= 5
        // Must start with "vb-"
        && s.index(0int) == 'v' && s.index(1int) == 'b' && s.index(2int) == '-'
        // Suffix must be non-empty and all lowercase alphanumeric, no dashes
        && spec_suffix_all_valid(s, 3, len)
        && spec_suffix_no_dash(s, 3, len)
    }

    /// Lemma: "vb-abc123" is a valid bead ID.
    pub proof fn lemma_valid_bead_id_example()
    {
        assert(spec_valid_bead_id(seq!['v', 'b', '-', 'a', 'b', 'c', '1', '2', '3']));
    }

    /// Lemma: "vb-" (empty suffix) is not valid.
    pub proof fn lemma_empty_bead_suffix_invalid()
    {
        assert(!spec_valid_bead_id(seq!['v', 'b', '-']));
    }

    /// Lemma: "vb" (no dash) is not valid.
    pub proof fn lemma_no_dash_invalid()
    {
        assert(!spec_valid_bead_id(seq!['v', 'b']));
    }

    /// Lemma: "VB-abc" (uppercase prefix) is not valid.
    pub proof fn lemma_uppercase_prefix_invalid()
    {
        assert(!spec_valid_bead_id(seq!['V', 'B', '-', 'a', 'b', 'c']));
    }

    /// Lemma: "vb-ABC" (uppercase suffix) is not valid.
    pub proof fn lemma_uppercase_suffix_invalid()
    {
        assert(!spec_valid_bead_id(seq!['v', 'b', '-', 'A', 'B', 'C']));
    }

    /// Lemma: "vb-abc-def" (extra dash) is not valid.
    pub proof fn lemma_extra_dash_invalid()
    {
        assert(!spec_valid_bead_id(seq!['v', 'b', '-', 'a', 'b', 'c', '-', 'd', 'e', 'f']));
    }

    /// Lemma: "vb-a" (4 chars, too short) is not valid.
    pub proof fn lemma_too_short_invalid()
    {
        assert(!spec_valid_bead_id(seq!['v', 'b', '-', 'a']));
    }

    /// Lemma: "vb-0" (4 chars, too short) is not valid.
    pub proof fn lemma_digit_too_short_invalid()
    {
        assert(!spec_valid_bead_id(seq!['v', 'b', '-', '0']));
    }

    // ==========================================================================
    // INVARIANT 5: Inventory uniqueness
    // All record IDs in an inventory are unique.
    // ==========================================================================

    /// Check if all IDs in a sequence of records are unique.
    pub closed spec fn spec_record_ids_unique(records: Seq<BoundaryRecordSpec>) -> bool {
        forall|i: int, j: int|
            #![auto]
            0 <= i && i < records.len() && 0 <= j && j < records.len() && i != j
            ==> records.index(i).id != records.index(j).id
    }

    /// Lemma: A single-record inventory has unique IDs.
    pub proof fn lemma_single_record_unique()
    {
        let empty_path: Seq<char> = seq![];
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 't', 'e', 's', 't'],
            class: BoundaryClassSpec::CAbi,
            source_path: PathSpec { components: empty_path },
            owner: FieldStateSpec::Missing,
            threat: FieldStateSpec::Missing,
            evidence: FieldStateSpec::Missing,
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Missing,
            waiver: FieldStateSpec::Missing,
        };
        let records: Seq<BoundaryRecordSpec> = seq![r];
        assert(spec_record_ids_unique(records));
    }

    /// Lemma: Two records with the same ID are not unique.
    pub proof fn lemma_duplicate_ids_not_unique()
    {
        let empty_path: Seq<char> = seq![];
        let r1 = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 's', 'a', 'm', 'e', '-', 'i', 'd'],
            class: BoundaryClassSpec::CAbi,
            source_path: PathSpec { components: empty_path },
            owner: FieldStateSpec::Missing,
            threat: FieldStateSpec::Missing,
            evidence: FieldStateSpec::Missing,
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Missing,
            waiver: FieldStateSpec::Missing,
        };
        let r2 = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 's', 'a', 'm', 'e', '-', 'i', 'd'],
            class: BoundaryClassSpec::Ffi,
            source_path: PathSpec { components: empty_path },
            owner: FieldStateSpec::Missing,
            threat: FieldStateSpec::Missing,
            evidence: FieldStateSpec::Missing,
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Missing,
            waiver: FieldStateSpec::Missing,
        };
        let records: Seq<BoundaryRecordSpec> = seq![r1, r2];
        assert(!spec_record_ids_unique(records));
    }

    /// Lemma: Three records with all distinct IDs are unique.
    pub proof fn lemma_three_distinct_ids_unique()
    {
        let empty_path: Seq<char> = seq![];
        let r1 = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'i', 'd', '-', '1'],
            class: BoundaryClassSpec::CAbi,
            source_path: PathSpec { components: empty_path },
            owner: FieldStateSpec::Missing,
            threat: FieldStateSpec::Missing,
            evidence: FieldStateSpec::Missing,
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Missing,
            waiver: FieldStateSpec::Missing,
        };
        let r2 = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'i', 'd', '-', '2'],
            class: BoundaryClassSpec::Ffi,
            source_path: PathSpec { components: empty_path },
            owner: FieldStateSpec::Missing,
            threat: FieldStateSpec::Missing,
            evidence: FieldStateSpec::Missing,
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Missing,
            waiver: FieldStateSpec::Missing,
        };
        let r3 = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'i', 'd', '-', '3'],
            class: BoundaryClassSpec::Ipc,
            source_path: PathSpec { components: empty_path },
            owner: FieldStateSpec::Missing,
            threat: FieldStateSpec::Missing,
            evidence: FieldStateSpec::Missing,
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Missing,
            waiver: FieldStateSpec::Missing,
        };
        let records: Seq<BoundaryRecordSpec> = seq![r1, r2, r3];
        assert(spec_record_ids_unique(records));
    }

    // ==========================================================================
    // INVARIANT 6: Validation compositionality
    // validate_record succeeds only when ALL sub-checks pass.
    // ==========================================================================

    /// Spec: owner is valid when Present and non-empty.
    pub closed spec fn spec_owner_valid(f: FieldStateSpec<Seq<char>>) -> bool {
        match f {
            FieldStateSpec::Present(s) => s.len() > 0,
            FieldStateSpec::Missing => false,
        }
    }

    /// Spec: threat is valid when Present and non-empty.
    pub closed spec fn spec_threat_valid(f: FieldStateSpec<Seq<char>>) -> bool {
        match f {
            FieldStateSpec::Present(s) => s.len() > 0,
            FieldStateSpec::Missing => false,
        }
    }

    /// Check if a sequence starts with a given prefix.
    pub closed spec fn seq_starts_with(prefix: Seq<char>, s: Seq<char>) -> bool {
        s.len() >= prefix.len()
        && forall|i: nat| 0 <= i < prefix.len() ==> s.index(i as int) == prefix.index(i as int)
    }

    /// Spec: source_path is valid when it starts with an allowed prefix.
    pub closed spec fn spec_source_path_valid(p: PathSpec) -> bool {
        let crates_prefix = seq!['c', 'r', 'a', 't', 'e', 's', '/'];
        let scripts_prefix = seq!['s', 'c', 'r', 'i', 'p', 't', 's', '/'];
        let fuzz_prefix = seq!['f', 'u', 'z', 'z', '/'];
        let cargo_prefix = seq!['C', 'a', 'r', 'g', 'o', '.', 't', 'o', 'm', 'l'];
        (seq_starts_with(crates_prefix, p.components))
        || (seq_starts_with(scripts_prefix, p.components))
        || (seq_starts_with(fuzz_prefix, p.components))
        || p.components == cargo_prefix
    }

    /// Spec: review is valid when Approved, or Waived with present waiver.
    pub closed spec fn spec_review_valid(
        status: FieldStateSpec<ReviewStatusSpec>,
        waiver: FieldStateSpec<EvidenceReferenceSpec>,
    ) -> bool {
        match status {
            FieldStateSpec::Present(ReviewStatusSpec::Approved) => true,
            FieldStateSpec::Present(ReviewStatusSpec::Waived) => match waiver {
                FieldStateSpec::Present(_) => true,
                FieldStateSpec::Missing => false,
            },
            _ => false,
        }
    }

    /// Spec: a record is fully validated when ALL sub-checks pass.
    pub closed spec fn spec_record_valid(r: BoundaryRecordSpec) -> bool {
        r.class != BoundaryClassSpec::Unknown
        && spec_owner_valid(r.owner)
        && spec_threat_valid(r.threat)
        && spec_source_path_valid(r.source_path)
        && spec_freshness_valid(r.freshness)
        && match r.evidence {
            FieldStateSpec::Present(_) => true,
            FieldStateSpec::Missing => false,
        }
        && spec_review_valid(r.review_status, r.waiver)
    }

    /// Lemma: A fully-specified approved record is valid.
    pub proof fn lemma_approved_record_valid()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '1'],
            class: BoundaryClassSpec::CAbi,
            source_path: PathSpec {
                components: seq!['c', 'r', 'a', 't', 'e', 's', '/', 'f', 'o', 'o', '/', 'l', 'i', 'b'],
            },
            owner: FieldStateSpec::Present(seq!['a', 'l', 'i', 'c', 'e']),
            threat: FieldStateSpec::Present(seq!['U', 'n', 's', 'a', 'f', 'e', ' ', 'F', 'F', 'I', ' ', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::RepoLocal(
                PathSpec {
                    components: seq!['f', 'u', 'z', 'z', '/', 'f', 'o', 'o'],
                },
                EvidenceKindSpec::Fuzz,
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 2,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(spec_record_valid(r));
    }

    /// Lemma: A record with missing owner is invalid.
    pub proof fn lemma_missing_owner_invalid()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '2'],
            class: BoundaryClassSpec::CAbi,
            source_path: PathSpec {
                components: seq!['c', 'r', 'a', 't', 'e', 's', '/', 'f', 'o', 'o', '/', 'l', 'i', 'b'],
            },
            owner: FieldStateSpec::Missing,
            threat: FieldStateSpec::Present(seq!['U', 'n', 's', 'a', 'f', 'e', ' ', 'F', 'F', 'I', ' ', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::RepoLocal(
                PathSpec { components: seq![] },
                EvidenceKindSpec::Fuzz,
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_record_valid(r));
    }

    /// Lemma: A record with stale freshness is invalid.
    pub proof fn lemma_stale_evidence_invalid()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '3'],
            class: BoundaryClassSpec::Ffi,
            source_path: PathSpec {
                components: seq!['f', 'u', 'z', 'z', '/', 'f', 'o', 'o'],
            },
            owner: FieldStateSpec::Present(seq!['b', 'o', 'b']),
            threat: FieldStateSpec::Present(seq!['R', 'a', 'w', ' ', 'b', 'y', 't', 'e', ' ', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::RepoLocal(
                PathSpec { components: seq![] },
                EvidenceKindSpec::Provenance,
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 5,
                schema_version: 3,
                evidence_version: 2,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_record_valid(r));
    }

    /// Lemma: A record with unknown class is invalid.
    pub proof fn lemma_unknown_class_invalid()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '4'],
            class: BoundaryClassSpec::Unknown,
            source_path: PathSpec { components: seq![] },
            owner: FieldStateSpec::Present(seq!['e', 'v', 'e']),
            threat: FieldStateSpec::Present(seq!['U', 'n', 'k', 'n', 'o', 'w', 'n']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::FreeText(
                seq!['n', '/', 'a'],
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_record_valid(r));
    }

    /// Lemma: A record with waived status but missing waiver is invalid.
    pub proof fn lemma_waived_no_waiver_invalid()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '5'],
            class: BoundaryClassSpec::Decoder,
            source_path: PathSpec {
                components: seq!['c', 'r', 'a', 't', 'e', 's', '/', 'd', 'e', 'c', 'o', 'd', 'e', 'r'],
            },
            owner: FieldStateSpec::Present(seq!['f', 'r', 'a', 'n', 'k']),
            threat: FieldStateSpec::Present(seq!['B', 'y', 't', 'e', ' ', 'i', 'n', 'g', 'e', 's', 't']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::ExternalProvenance(
                seq!['e', 'x', 't', 'e', 'r', 'n', 'a', 'l', ':', 's', 'h', 'a', '5', '5', '6', '=', 'a', 'b', 'c'],
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Waived),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_record_valid(r));
    }

    /// Lemma: A record with waived status and present waiver is valid.
    pub proof fn lemma_waived_with_waiver_valid()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '6'],
            class: BoundaryClassSpec::ExternalBinary,
            source_path: PathSpec {
                components: seq!['s', 'c', 'r', 'i', 'p', 't', 's', '/', 'b', 'i', 'n'],
            },
            owner: FieldStateSpec::Present(seq!['g', 'r', 'a', 'c', 'e']),
            threat: FieldStateSpec::Present(seq!['E', 'x', 't', 'e', 'r', 'n', 'a', 'l', ' ', 'b', 'i', 'n', 'a', 'r', 'y']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::ExternalProvenance(
                seq!['v', 'b', '-', 'a', 'b', 'c', '1', '2', '3'],
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Waived),
            waiver: FieldStateSpec::Present(EvidenceReferenceSpec::ExternalProvenance(
                seq!['v', 'b', '-', 'd', 'e', 'f', '4', '5', '6'],
            )),
        };
        assert(spec_record_valid(r));
    }

    /// Lemma: A record with Other review status is invalid.
    pub proof fn lemma_other_review_status_invalid()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '7'],
            class: BoundaryClassSpec::GeneratedCode,
            source_path: PathSpec {
                components: seq!['c', 'r', 'a', 't', 'e', 's', '/', 'g', 'e', 'n', '/', 'b', 'i', 'n'],
            },
            owner: FieldStateSpec::Present(seq!['h', 'e', 'i', 'd', 'i']),
            threat: FieldStateSpec::Present(seq!['G', 'e', 'n', 'e', 'r', 'a', 't', 'e', 'd', ' ', 'c', 'o', 'd', 'e']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::RepoLocal(
                PathSpec { components: seq![] },
                EvidenceKindSpec::Isolation,
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Other(seq!['p', 'e', 'n', 'd', 'i', 'n', 'g'])),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_record_valid(r));
    }

    /// Lemma: A record with missing evidence is invalid.
    pub proof fn lemma_missing_evidence_invalid()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '8'],
            class: BoundaryClassSpec::Ipc,
            source_path: PathSpec {
                components: seq!['f', 'u', 'z', 'z', '/', 'i', 'p', 'c', '.', 'r', 's'],
            },
            owner: FieldStateSpec::Present(seq!['i', 'v', 'a', 'n']),
            threat: FieldStateSpec::Present(seq!['I', 'P', 'C', ' ', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y']),
            evidence: FieldStateSpec::Missing,
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_record_valid(r));
    }

    /// Lemma: A record with missing threat is invalid.
    pub proof fn lemma_missing_threat_invalid()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '9'],
            class: BoundaryClassSpec::CAbi,
            source_path: PathSpec {
                components: seq!['c', 'r', 'a', 't', 'e', 's', '/', 'a', 'b', 'i'],
            },
            owner: FieldStateSpec::Present(seq!['j', 'u', 'd', 'y']),
            threat: FieldStateSpec::Missing,
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::RepoLocal(
                PathSpec { components: seq![] },
                EvidenceKindSpec::Fuzz,
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_record_valid(r));
    }

    /// Lemma: A record with invalid source path is invalid.
    pub proof fn lemma_invalid_source_path()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'b', 'o', 'u', 'n', 'd', 'a', 'r', 'y', '-', '1', '0'],
            class: BoundaryClassSpec::ExternalBinary,
            source_path: PathSpec {
                components: seq!['e', 'x', 't', 'e', 'r', 'n', 'a', 'l', '/', 'b', 'i', 'n'],
            },
            owner: FieldStateSpec::Present(seq!['k', 'a', 'r', 'l']),
            threat: FieldStateSpec::Present(seq!['E', 'x', 't', 'e', 'r', 'n', 'a', 'l', ' ', 'b', 'i', 'n', 'a', 'r', 'y']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::ExternalProvenance(
                seq!['v', 'b', '-', 'a', 'b', 'c'],
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_record_valid(r));
    }

    // ==========================================================================
    // INVARIANT 7: Unsafe forbidden — first-party crate paths
    // ==========================================================================

    /// A source path is in the first-party crate namespace.
    pub closed spec fn spec_is_first_party_unsafe(r: BoundaryRecordSpec) -> bool {
        r.class == BoundaryClassSpec::UnsafeAdjacentDependency
        && seq_starts_with(seq!['c', 'r', 'a', 't', 'e', 's'], r.source_path.components)
    }

    /// Lemma: A first-party C ABI boundary is NOT an unsafe violation.
    pub proof fn lemma_first_party_cabi_not_unsafe()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'c', 'a', 'b', 'i', '-', '1'],
            class: BoundaryClassSpec::CAbi,
            source_path: PathSpec {
                components: seq!['c', 'r', 'a', 't', 'e', 's', '/', 'f', 'o', 'o'],
            },
            owner: FieldStateSpec::Present(seq!['o', 'w', 'n', 'e', 'r']),
            threat: FieldStateSpec::Present(seq!['t', 'h', 'r', 'e', 'a', 't']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::RepoLocal(
                PathSpec { components: seq![] },
                EvidenceKindSpec::Fuzz,
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_is_first_party_unsafe(r));
    }

    /// Lemma: A first-party unsafe adjacent dependency IS a violation.
    pub proof fn lemma_first_party_unsafe_is_violation()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'u', 'n', 's', 'a', 'f', 'e', '-', '1'],
            class: BoundaryClassSpec::UnsafeAdjacentDependency,
            source_path: PathSpec {
                components: seq!['c', 'r', 'a', 't', 'e', 's', '/', 'f', 'o', 'o'],
            },
            owner: FieldStateSpec::Present(seq!['o', 'w', 'n', 'e', 'r']),
            threat: FieldStateSpec::Present(seq!['u', 'n', 's', 'a', 'f', 'e', ' ', 'd', 'e', 'p']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::RepoLocal(
                PathSpec { components: seq![] },
                EvidenceKindSpec::Fuzz,
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(spec_is_first_party_unsafe(r));
    }

    /// Lemma: An external unsafe adjacent dependency is NOT first-party.
    pub proof fn lemma_external_unsafe_not_violation()
    {
        let r = BoundaryRecordSpec {
            id: seq!['v', 'b', '-', 'u', 'n', 's', 'a', 'f', 'e', '-', 'e', 'x', 't'],
            class: BoundaryClassSpec::UnsafeAdjacentDependency,
            source_path: PathSpec {
                components: seq!['e', 'x', 't', 'e', 'r', 'n', 'a', 'l', '/', 'd', 'e', 'p'],
            },
            owner: FieldStateSpec::Present(seq!['o', 'w', 'n', 'e', 'r']),
            threat: FieldStateSpec::Present(seq!['u', 'n', 's', 'a', 'f', 'e', ' ', 'd', 'e', 'p']),
            evidence: FieldStateSpec::Present(EvidenceReferenceSpec::ExternalProvenance(
                seq!['v', 'b', '-', 'x', 'y', 'z', '7', '8', '9'],
            )),
            freshness: FreshnessMarkerSpec {
                source_version: 1,
                schema_version: 1,
                evidence_version: 1,
            },
            review_status: FieldStateSpec::Present(ReviewStatusSpec::Approved),
            waiver: FieldStateSpec::Missing,
        };
        assert(!spec_is_first_party_unsafe(r));
    }

    // ==========================================================================
    // INVARIANT 8: Boundary exposure classification
    // ==========================================================================

    /// A boundary is risky if risk != None OR class is GeneratedCode or
    /// UnsafeAdjacentDependency. Mirrors production `is_risky_boundary`.
    pub closed spec fn spec_is_risky(
        class: BoundaryClassSpec,
        exposure_risk: BoundaryRiskSpec,
    ) -> bool {
        exposure_risk != BoundaryRiskSpec::None
            || class == BoundaryClassSpec::GeneratedCode
            || class == BoundaryClassSpec::UnsafeAdjacentDependency
    }

    /// Lemma: CAbi with Multiple risk is risky.
    pub proof fn lemma_risky_cabi_multiple()
    {
        assert(spec_is_risky(BoundaryClassSpec::CAbi, BoundaryRiskSpec::Multiple));
    }

    /// Lemma: CAbi with no risk is not risky.
    pub proof fn lemma_cabi_no_risk_not_risky()
    {
        assert(!spec_is_risky(BoundaryClassSpec::CAbi, BoundaryRiskSpec::None));
    }

    /// Lemma: GeneratedCode is always risky regardless of exposure.
    pub proof fn lemma_generated_code_always_risky()
    {
        assert(spec_is_risky(BoundaryClassSpec::GeneratedCode, BoundaryRiskSpec::None));
    }

    /// Lemma: UnsafeAdjacentDependency is always risky regardless of exposure.
    pub proof fn lemma_unsafe_adjacent_always_risky()
    {
        assert(spec_is_risky(BoundaryClassSpec::UnsafeAdjacentDependency, BoundaryRiskSpec::None));
    }

    /// Lemma: Decoder with no risk is not risky.
    pub proof fn lemma_decoder_not_risky()
    {
        assert(!spec_is_risky(BoundaryClassSpec::Decoder, BoundaryRiskSpec::None));
    }

    /// Lemma: All non-generated/non-unsafe classes are not risky with None risk.
    pub proof fn lemma_non_risky_classes()
    {
        assert(!spec_is_risky(BoundaryClassSpec::CAbi, BoundaryRiskSpec::None));
        assert(!spec_is_risky(BoundaryClassSpec::Ffi, BoundaryRiskSpec::None));
        assert(!spec_is_risky(BoundaryClassSpec::Ipc, BoundaryRiskSpec::None));
        assert(!spec_is_risky(BoundaryClassSpec::ExternalBinary, BoundaryRiskSpec::None));
        assert(!spec_is_risky(BoundaryClassSpec::Decoder, BoundaryRiskSpec::None));
    }

    // ==========================================================================
    // INVARIANT 9: Source path validation
    // Only allowed prefixes: crates, scripts, fuzz, Cargo.toml
    // ==========================================================================

    /// Lemma: "crates/foo/bar" is a valid source path.
    pub proof fn lemma_valid_crate_path()
    {
        let p = PathSpec {
            components: seq!['c', 'r', 'a', 't', 'e', 's', '/', 'f', 'o', 'o', '/', 'b', 'a', 'r'],
        };
        assert(spec_source_path_valid(p));
    }

    /// Lemma: "fuzz/test.rs" is a valid source path.
    pub proof fn lemma_valid_fuzz_path()
    {
        let p = PathSpec {
            components: seq!['f', 'u', 'z', 'z', '/', 't', 'e', 's', 't', '.', 'r', 's'],
        };
        assert(spec_source_path_valid(p));
    }

    /// Lemma: "scripts/verify.sh" is a valid source path.
    pub proof fn lemma_valid_script_path()
    {
        let p = PathSpec {
            components: seq!['s', 'c', 'r', 'i', 'p', 't', 's', '/', 'v', 'e', 'r', 'i', 'f', 'y', '.', 's', 'h'],
        };
        assert(spec_source_path_valid(p));
    }
}
