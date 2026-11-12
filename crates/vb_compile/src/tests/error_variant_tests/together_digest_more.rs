
/// Unit test: canonical_primitive_name(Together) directly returns "together".
///
/// This is the direct assertion that the fix is in place. Previous tests
/// verified this indirectly through the compile path; this test calls
/// the function directly on a programmatically constructed Together.
#[test]
fn canonical_primitive_name_together_returns_together_direct() {
    use crate::mod_compile_lowering::canonical_primitive_name;
    use crate::{StepPrimitive, TogetherBranch};

    let together = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label: "main".into(),
            steps: vec![],
        }],
    };
    let name = canonical_primitive_name(&together);
    assert_eq!(
        name, "together",
        "canonical_primitive_name(Together) must return 'together', not 'parallel'"
    );
}

// =========================================================================
// GAP-2: Exhaustive unit test for canonical_primitive_name all 12+1 variants
// =========================================================================

/// Unit test: canonical_primitive_name returns the correct static string
/// for every known StepPrimitive variant.
///
/// Each variant is programmatically constructed and the exact expected
/// name string is asserted. This is a table-driven rollout of the match
/// arms in part_05.rs lines 99-113.
#[test]
fn canonical_primitive_name_returns_correct_names_for_all_variants() {
    use crate::mod_compile_lowering::canonical_primitive_name;
    use crate::{ChooseBranch, ScalarValue, StepPrimitive, TogetherBranch};

    let cases: Vec<(StepPrimitive, &'static str)> = vec![
        (
            StepPrimitive::Set {
                output: "x".into(),
                value: "1".into(),
            },
            "set",
        ),
        (
            StepPrimitive::Save {
                value: ScalarValue::String("x".into()),
            },
            "save",
        ),
        (
            StepPrimitive::Do {
                action: "act".into(),
                input: "in".into(),
            },
            "do",
        ),
        (
            StepPrimitive::Choose {
                branches: vec![ChooseBranch {
                    when: "true".into(),
                    steps: vec![],
                }],
                otherwise: None.into(),
            },
            "choose",
        ),
        (
            StepPrimitive::ForEach {
                variable: "item".into(),
                input: "items".into(),
                at_once: None,
                body: vec![],
            },
            "for_each",
        ),
        (
            StepPrimitive::Together {
                branches: vec![TogetherBranch {
                    label: "a".into(),
                    steps: vec![],
                }],
            },
            "together",
        ),
        (
            StepPrimitive::Collect {
                variable: "p".into(),
                source: "src".into(),
                pages: None,
                items: None,
                body: vec![],
            },
            "collect",
        ),
        (
            StepPrimitive::Aggregate {
                variable: "acc".into(),
                input: "list".into(),
                initial: "0".into(),
                body: vec![],
            },
            "reduce",
        ),
        (
            StepPrimitive::Repeat {
                max_attempts: 3,
                body: vec![],
            },
            "repeat",
        ),
        (
            StepPrimitive::Wait {
                event: None,
                timeout: None,
            },
            "wait",
        ),
        (
            StepPrimitive::Ask {
                prompt: "?".into(),
                timeout: None,
            },
            "ask",
        ),
        (
            StepPrimitive::Finish {
                result: ScalarValue::Integer(0),
            },
            "finish",
        ),
    ];

    for (primitive, expected_name) in &cases {
        let name = canonical_primitive_name(primitive);
        assert_eq!(
            name, *expected_name,
            "canonical_primitive_name({expected_name}) should return \"{expected_name}\""
        );
    }
}

// =========================================================================
// GAP-3: Single-branch together produces deterministic digest
// =========================================================================

/// Unit test: a Together step with exactly one branch compiles successfully
/// and produces a deterministic, non-zero digest.
///
/// The proptest strategies in together_digest_sensitivity.rs only generate
/// 2+ branches. This test covers the edge case of a single branch.
#[test]
fn test_single_branch_together_produces_deterministic_digest() {
    let yaml = "\
version: velvet-ballistics/v1
name: single-branch-together
when:
  manual: {}
steps:
  - id: fanout
    together:
      branches:
        - label: sole-branch
          steps:
            - id: set-val
              set:
                output: \"x\"
                value: \"1\"
  - id: done
    finish:
      result: 0
";

    let source = crate::parse_workflow_source(yaml).expect("single-branch parse");
    let workflow = crate::compile_source(&source).expect("single-branch compile");

    // Non-zero
    let digest = workflow.digest();
    let is_all_zero = digest.as_bytes().iter().all(|&b| b == 0);
    assert!(
        !is_all_zero,
        "single-branch together digest must not be all zeros"
    );

    // Deterministic
    let source2 = crate::parse_workflow_source(yaml).expect("parse 2");
    let workflow2 = crate::compile_source(&source2).expect("compile 2");
    assert_eq!(
        digest,
        workflow2.digest(),
        "single-branch together digest must be deterministic"
    );
}

// =========================================================================
// GAP-4: Many-branch together (10+ branches) — unit test
// =========================================================================

/// Unit test: a Together step with many branches (10) compiles without panic
/// and produces a deterministic, non-zero digest.
///
/// Proptest coverage is added in together_digest_sensitivity.rs for
/// branch count variation (1..=20). This unit test provides a fixed
/// large-count validation that can be checked quickly.
#[test]
fn test_many_branch_together_produces_deterministic_digest() {
    // Build YAML with 10 branches programmatically
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: many-branch-test\nwhen:\n  manual: {}\nsteps:\n  - id: fanout\n    together:\n      branches:\n",
    );
    for i in 0..10 {
        yaml.push_str(&format!(
            "        - label: branch-{i}\n          steps:\n            - id: set-{i}\n              set:\n                output: \"o{i}\"\n                value: \"{i}\"\n"
        ));
    }
    yaml.push_str("  - id: done\n    finish:\n      result: 0\n");

    let source = crate::parse_workflow_source(&yaml).expect("many-branch parse");
    let workflow = crate::compile_source(&source).expect("many-branch compile");

    let digest = workflow.digest();
    let is_all_zero = digest.as_bytes().iter().all(|&b| b == 0);
    assert!(
        !is_all_zero,
        "many-branch (10) together digest must not be all zeros"
    );

    // Deterministic
    let source2 = crate::parse_workflow_source(&yaml).expect("parse 2");
    let workflow2 = crate::compile_source(&source2).expect("compile 2");
    assert_eq!(
        digest,
        workflow2.digest(),
        "many-branch together digest must be deterministic"
    );
}

// =========================================================================
// GAP-5: Empty sub-steps within branch — programmatic test
// =========================================================================

/// Unit test: a Together branch with zero sub-steps (`steps: []`) produces
/// a deterministic, non-zero digest when processed by digest_step_primitive.
///
/// Validation currently rejects YAML with empty branch steps, but this
/// test exercises the digest function directly to prove it handles the
/// degenerate case safely. The branch label is hashed; the inner sub-step
/// loop executes zero times.
#[test]
fn test_empty_sub_steps_within_together_branch_produces_deterministic_digest() {
    use crate::mod_compile_lowering::digest_step_primitive;
    use crate::{StepPrimitive, TogetherBranch};
    use vb_core::WorkflowDigest;

    let together = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label: "empty-branch".into(),
            steps: vec![], // empty — inner loop runs zero times
        }],
    };

    let mut hasher1 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher1, &together).expect("test: branch count within u16");
    let digest1 = WorkflowDigest::from_bytes(hasher1.finalize().into());

    let is_all_zero = digest1.as_bytes().iter().all(|&b| b == 0);
    assert!(
        !is_all_zero,
        "digest for empty branch must not be all zeros"
    );

    let mut hasher2 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher2, &together).expect("test: branch count within u16");
    let digest2 = WorkflowDigest::from_bytes(hasher2.finalize().into());
    assert_eq!(
        digest1, digest2,
        "empty-branch digest must be deterministic"
    );
}

// =========================================================================
// GAP-6: Zero-branch Together — programmatic test
// =========================================================================

/// Unit test: a Together step with zero branches (branches: vec![])
/// produces a deterministic, non-zero digest when processed by
/// digest_step_primitive.
///
/// The function hashes "together" + 0u16 (branch count), then the
/// branch iteration loop runs zero times. This ensures no panic
/// on the zero-branch edge case.
#[test]
fn test_zero_branch_together_produces_deterministic_digest() {
    use crate::StepPrimitive;
    use crate::mod_compile_lowering::digest_step_primitive;
    use vb_core::WorkflowDigest;

    let together = StepPrimitive::Together {
        branches: vec![], // zero branches
    };

    let mut hasher1 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher1, &together).expect("test: branch count within u16");
    let digest1 = WorkflowDigest::from_bytes(hasher1.finalize().into());

    let is_all_zero = digest1.as_bytes().iter().all(|&b| b == 0);
    assert!(
        !is_all_zero,
        "digest for zero-branch together must not be all zeros"
    );

    let mut hasher2 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher2, &together).expect("test: branch count within u16");
    let digest2 = WorkflowDigest::from_bytes(hasher2.finalize().into());
    assert_eq!(digest1, digest2, "zero-branch digest must be deterministic");

    // Zero-branch and single-branch should differ (branch count changed)
    let single = StepPrimitive::Together {
        branches: vec![crate::TogetherBranch {
            label: "solo".into(),
            steps: vec![],
        }],
    };
    let mut hasher3 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher3, &single).expect("test: branch count within u16");
    let digest3 = WorkflowDigest::from_bytes(hasher3.finalize().into());
    assert_ne!(
        digest1, digest3,
        "zero-branch and single-branch digests must differ (branch count changed)"
    );
}

// =========================================================================
// GAP-7: digest_sub_step with non-Together primitive (ForEach)
// =========================================================================

/// Unit test: digest_sub_step correctly processes a sub-step with a ForEach
/// primitive when called through digest_step_primitive.
///
/// The compile pipeline rejects non-Set primitives inside Together branches
/// (emit_single_body_set enforces Set-only), but digest_step_primitive is
/// a pure hashing function with no such restriction. This test calls
/// digest_step_primitive directly to verify that digest_sub_step hashes
/// the sub-step's id and delegates to digest_step_primitive for the
/// primitive — even when that primitive is ForEach.
///
/// The ForEach hits the `other` arm in digest_step_primitive (hashing only
/// the canonical name "for_each"). This verifies digest_sub_step is not
/// accidentally Together-specific.
#[test]
fn test_digest_step_primitive_with_for_each_sub_step_produces_deterministic_digest() {
    use crate::mod_compile_lowering::digest_step_primitive;
    use crate::{StepAst, StepPrimitive, TogetherBranch};
    use vb_core::WorkflowDigest;

    // Construct a Together with a branch containing a ForEach sub-step.
    // The compile-time restriction (emit_single_body_set) doesn't apply
    // to the pure hashing function — we test the hashing path directly.
    let foreach_sub_step = StepAst {
        id: "inner-foreach".into(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "item".into(),
            input: "0".into(),
            at_once: Some(1),
            body: vec![],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let together = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label: "loop-branch".into(),
            steps: vec![foreach_sub_step],
        }],
    };

    let mut hasher1 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher1, &together).expect("test: branch count within u16");
    let digest1 = WorkflowDigest::from_bytes(hasher1.finalize().into());

    let is_all_zero = digest1.as_bytes().iter().all(|&b| b == 0);
    assert!(!is_all_zero, "foreach-sub digest must not be all zeros");

    // Deterministic
    let mut hasher2 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher2, &together).expect("test: branch count within u16");
    let digest2 = WorkflowDigest::from_bytes(hasher2.finalize().into());
    assert_eq!(digest1, digest2, "foreach-sub digest must be deterministic");

    // Changing the ForEach sub-step's ID must produce a different digest
    let foreach_sub_diff_id = StepAst {
        id: "different-foreach-id".into(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "item".into(),
            input: "0".into(),
            at_once: Some(1),
            body: vec![],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let together_diff_id = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label: "loop-branch".into(),
            steps: vec![foreach_sub_diff_id],
        }],
    };
    let mut hasher3 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher3, &together_diff_id).expect("test: branch count within u16");
    let digest3 = WorkflowDigest::from_bytes(hasher3.finalize().into());
    assert_ne!(
        digest1, digest3,
        "different foreach sub-step ID must produce different digest"
    );

    // ForEach vs Set as sub-step primitive must produce different digests
    let set_sub_step = StepAst {
        id: "inner-foreach".into(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".into(),
            value: "1".into(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let together_set = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label: "loop-branch".into(),
            steps: vec![set_sub_step],
        }],
    };
    let mut hasher4 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher4, &together_set).expect("test: branch count within u16");
    let digest4 = WorkflowDigest::from_bytes(hasher4.finalize().into());
    assert_ne!(
        digest1, digest4,
        "ForEach vs Set sub-step must produce different digests (different primitive hash)"
    );
}

// =========================================================================
// GAP-9: digest_step_primitive other-arm coverage
// =========================================================================

/// Unit test: digest_step_primitive's `other` arm handles all non-Together,
/// non-Set, non-Finish primitives without panic and produces deterministic,
/// non-zero digests.
///
/// Each variant's canonical name is hashed. The `other` arm is tested
/// exhaustively by enumerating all 9 non-special-cased variants.
#[test]
fn test_digest_step_primitive_other_arm_produces_deterministic_digest() {
    use crate::mod_compile_lowering::digest_step_primitive;
    use crate::{ChooseBranch, ScalarValue, StepPrimitive};
    use vb_core::WorkflowDigest;

    let other_variants: Vec<(&str, StepPrimitive)> = vec![
        (
            "do",
            StepPrimitive::Do {
                action: "act".into(),
                input: "in".into(),
            },
        ),
        (
            "choose",
            StepPrimitive::Choose {
                branches: vec![ChooseBranch {
                    when: "true".into(),
                    steps: vec![],
                }],
                otherwise: None.into(),
            },
        ),
        (
            "for_each",
            StepPrimitive::ForEach {
                variable: "item".into(),
                input: "items".into(),
                at_once: None,
                body: vec![],
            },
        ),
        (
            "collect",
            StepPrimitive::Collect {
                variable: "p".into(),
                source: "src".into(),
                pages: None,
                items: None,
                body: vec![],
            },
        ),
        (
            "aggregate",
            StepPrimitive::Aggregate {
                variable: "acc".into(),
                input: "list".into(),
                initial: "0".into(),
                body: vec![],
            },
        ),
        (
            "repeat",
            StepPrimitive::Repeat {
                max_attempts: 3,
                body: vec![],
            },
        ),
        (
            "wait",
            StepPrimitive::Wait {
                event: None,
                timeout: None,
            },
        ),
        (
            "ask",
            StepPrimitive::Ask {
                prompt: "?".into(),
                timeout: None,
            },
        ),
        (
            "save",
            StepPrimitive::Save {
                value: ScalarValue::String("val".into()),
            },
        ),
    ];

    for (name, primitive) in &other_variants {
        let mut hasher = blake3::Hasher::new();
        digest_step_primitive(&mut hasher, primitive).expect("test: branch count within u16");
        let digest = WorkflowDigest::from_bytes(hasher.finalize().into());

        let is_all_zero = digest.as_bytes().iter().all(|&b| b == 0);
        assert!(
            !is_all_zero,
            "digest_step_primitive other arm for '{name}' must produce non-zero digest"
        );

        let mut hasher2 = blake3::Hasher::new();
        digest_step_primitive(&mut hasher2, primitive).expect("test: branch count within u16");
        let digest2 = WorkflowDigest::from_bytes(hasher2.finalize().into());
        assert_eq!(
            digest, digest2,
            "digest_step_primitive other arm for '{name}' must be deterministic"
        );
    }

    // Also verify: different primitives produce different digests (since
    // each hashes a different canonical name).
    let mut digests_seen: Vec<WorkflowDigest> = Vec::new();
    for (_name, primitive) in &other_variants {
        let mut hasher = blake3::Hasher::new();
        digest_step_primitive(&mut hasher, primitive).expect("test: branch count within u16");
        let digest = WorkflowDigest::from_bytes(hasher.finalize().into());
        // All distinct canonical names should yield distinct digests
        for seen in &digests_seen {
            assert_ne!(
                digest, *seen,
                "different other-arm primitives must produce different digests"
            );
        }
        digests_seen.push(digest);
    }
}

// =========================================================================
// GAP-12: Direct digest_sub_step determinism (via digest_step_primitive)
// =========================================================================

/// Unit test: digest_sub_step produces deterministic, non-zero digests
/// and is sensitive to StepAst.id changes.
///
/// Since digest_sub_step is a private function, we test it through
/// digest_step_primitive on a Together that contains sub-steps. The
/// Together arm calls digest_sub_step for each branch step.
///
/// This test also verifies the digest is NOT all-zeros (a vacuous
/// pass indicator) and that changing only the sub-step's ID changes
/// the digest (proving digest_sub_step hashes the ID field).
#[test]
fn test_digest_sub_step_produces_deterministic_nonzero_digest() {
    use crate::mod_compile_lowering::digest_step_primitive;
    use crate::{StepAst, StepPrimitive, TogetherBranch};
    use vb_core::WorkflowDigest;

    // Base sub-step — will be cloned to avoid move-after-move issues
    let base_sub_step = StepAst {
        id: "inner-set".into(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".into(),
            value: "42".into(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let together = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label: "b1".into(),
            steps: vec![base_sub_step.clone()],
        }],
    };

    let mut hasher1 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher1, &together).expect("test: branch count within u16");
    let digest1 = WorkflowDigest::from_bytes(hasher1.finalize().into());

    let is_all_zero = digest1.as_bytes().iter().all(|&b| b == 0);
    assert!(
        !is_all_zero,
        "digest including sub-step must not be all zeros"
    );

    // Deterministic
    let mut hasher2 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher2, &together).expect("test: branch count within u16");
    let digest2 = WorkflowDigest::from_bytes(hasher2.finalize().into());
    assert_eq!(
        digest1, digest2,
        "digest including sub-step must be deterministic"
    );

    // Sensitivity to sub-step ID — construct from base with changed id
    let sub_step_diff_id = StepAst {
        id: "different-id".into(),
        ..base_sub_step.clone()
    };
    let together_diff = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label: "b1".into(),
            steps: vec![sub_step_diff_id],
        }],
    };
    let mut hasher3 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher3, &together_diff).expect("test: branch count within u16");
    let digest3 = WorkflowDigest::from_bytes(hasher3.finalize().into());
    assert_ne!(
        digest1, digest3,
        "different sub-step ID must produce different digest (digest_sub_step hashes id)"
    );

    // Sensitivity to sub-step primitive contents
    let sub_step_diff_val = StepAst {
        primitive: StepPrimitive::Set {
            output: "x".into(),
            value: "99".into(),
        },
        ..base_sub_step.clone()
    };
    let together_diff_val = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label: "b1".into(),
            steps: vec![sub_step_diff_val],
        }],
    };
    let mut hasher4 = blake3::Hasher::new();
    digest_step_primitive(&mut hasher4, &together_diff_val).expect("test: branch count within u16");
    let digest4 = WorkflowDigest::from_bytes(hasher4.finalize().into());
    assert_ne!(
        digest1, digest4,
        "different sub-step value must produce different digest (digest_sub_step hashes primitive)"
    );
}
