#![forbid(unsafe_code)]

//! Proptests for collect lowering: vb-8mdp.7
//!
//! Behaviors covered:
//!   B-013: Collect emits exactly 4 nodes with consecutive IDs
//!   (CollectStart, SetConst, CollectPage, CollectFinish).
//!   B-014: CollectStart limit/page_size match input; source slot preserved.
//!   B-015: Step offset property (body=id+1, page=id+2, done=id+3).
//!   B-016: Budget defaults (limit/page_size default to 1 when None).
//!   B-017: Max valid start ID = u16::MAX - 3.

use proptest::prelude::*;
use vb_compile::{SlotCompiler, compile_workflow, lower_collect};
use vb_core::{
    CompiledNodeKind,
    ids::{SlotIdx, StepIdx},
};

// =========================================================================
// Helper: access a node at a StepIdx, returning Option
// =========================================================================

/// Convenience: fetch node i from the compiled workflow.
fn node_at(wf: &vb_core::CompiledWorkflow, step: u16) -> Option<&vb_core::CompiledNode> {
    wf.node(StepIdx::new(step))
}

// =========================================================================
// YAML generators
// =========================================================================

/// Build a complete YAML string for a workflow whose sole substantive step
/// is a `collect` primitive with the given parameters, followed by a
/// `finish` step.
fn collect_yaml(
    source_slot: &str,
    pages: Option<u32>,
    items: Option<u32>,
    set_value: &str,
) -> String {
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: collect-test\nwhen:\n  manual: {}\nsteps:\n",
    );
    // Collect step
    yaml.push_str("  - id: collect_pages\n    collect:\n");
    yaml.push_str("      variable: page\n");
    yaml.push_str(&format!("      source: \"{source_slot}\"\n"));
    if let Some(p) = pages {
        yaml.push_str(&format!("      pages: {p}\n"));
    }
    if let Some(i) = items {
        yaml.push_str(&format!("      items: {i}\n"));
    }
    yaml.push_str("      steps:\n");
    yaml.push_str("        - id: remember_page\n");
    yaml.push_str("          set:\n");
    yaml.push_str("            output: page_seen\n");
    yaml.push_str(&format!("            value: \"{set_value}\"\n"));
    // Finish step
    yaml.push_str("  - id: done\n    finish:\n      result: 0\n");
    yaml
}

/// Build a YAML string where the collect step is NOT the first step, so
/// its StepIdx is offset from 0.
fn collect_yaml_with_preamble(source_slot: &str, pages: Option<u32>, items: Option<u32>) -> String {
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: collect-test\nwhen:\n  manual: {}\nsteps:\n",
    );
    // Preamble: two set steps that consume IDs 0 and 1
    yaml.push_str("  - id: setup_a\n    set:\n      output: a\n      value: \"10\"\n");
    yaml.push_str("  - id: setup_b\n    set:\n      output: b\n      value: \"20\"\n");
    // Collect step (starts at ID 2)
    yaml.push_str("  - id: collect_pages\n    collect:\n");
    yaml.push_str("      variable: page\n");
    yaml.push_str(&format!("      source: \"{}\"\n", source_slot));
    if let Some(p) = pages {
        yaml.push_str(&format!("      pages: {}\n", p));
    }
    if let Some(i) = items {
        yaml.push_str(&format!("      items: {}\n", i));
    }
    yaml.push_str("      steps:\n");
    yaml.push_str("        - id: remember_page\n");
    yaml.push_str("          set:\n");
    yaml.push_str("            output: page_seen\n");
    yaml.push_str("            value: \"42\"\n");
    // Finish step
    yaml.push_str("  - id: done\n    finish:\n      result: 0\n");
    yaml
}

/// YAML that does NOT provide explicit pages or items (tests B-016 defaults).
fn collect_yaml_no_budget() -> String {
    collect_yaml("0", None, None, "7")
}

// =========================================================================
// Strategies
// =========================================================================

/// Arbitrary collect parameters for proptesting.
#[derive(Debug, Clone)]
struct CollectParams {
    source: String,
    pages: Option<u32>,
    items: Option<u32>,
    set_value: String,
}

fn collect_params_strategy() -> impl Strategy<Value = CollectParams> {
    (
        "[0-9]+",
        any::<Option<u32>>(),
        any::<Option<u32>>(),
        "[0-9]+",
    )
        .prop_map(|(source, pages, items, set_value)| CollectParams {
            source,
            pages,
            items,
            set_value,
        })
}

/// Source slot as a digit string (small values for safety).
fn source_digit_str() -> impl Strategy<Value = String> {
    "[0-9]".prop_map(|s: String| s)
}

// =========================================================================
// Proptest: B-013 — Exactly 4 collect nodes with consecutive IDs
// =========================================================================

proptest! {
    /// B-013: For a valid collect YAML, the compiled workflow contains
    /// exactly 4 collect-related nodes (CollectStart, SetConst body,
    /// CollectPage, CollectFinish) with consecutive IDs relative to the
    /// collect step's own StepIdx.
    #[test]
    fn collect_four_nodes_consecutive_ids(
        params in collect_params_strategy(),
    ) {
        let yaml = collect_yaml(&params.source, params.pages, params.items, &params.set_value);
        let result = compile_workflow(yaml.as_bytes());

        // Source parsing can fail; skip invalid source strings
        if result.is_ok() {
            let wf = result.unwrap();
            // Expect at least 5 nodes (4 collect + 1 finish)
            prop_assert!(wf.node_count() >= 5,
                "workflow with collect should have at least 5 nodes, got {}",
                wf.node_count());

            // Node 0 must be CollectStart
            let start = node_at(&wf, 0).expect("node 0 must exist");
            prop_assert_eq!(start.id.get(), 0,
                "CollectStart must have ID 0");
            prop_assert!(
                matches!(&start.kind, CompiledNodeKind::CollectStart { .. }),
                "node[0] must be CollectStart, got {:?}", start.kind
            );

            // Node 1 must be SetConst (body step)
            let set_node = node_at(&wf, 1).expect("node 1 must exist");
            prop_assert_eq!(set_node.id.get(), 1,
                "body node must have ID 1 (id+1)");
            prop_assert!(
                matches!(&set_node.kind, CompiledNodeKind::SetConst { .. }),
                "node[1] must be SetConst, got {:?}", set_node.kind
            );

            // Node 2 must be CollectPage
            let page = node_at(&wf, 2).expect("node 2 must exist");
            prop_assert_eq!(page.id.get(), 2,
                "CollectPage must have ID 2 (id+2)");
            prop_assert!(
                matches!(&page.kind, CompiledNodeKind::CollectPage { .. }),
                "node[2] must be CollectPage, got {:?}", page.kind
            );

            // Node 3 must be CollectFinish
            let finish_node = node_at(&wf, 3).expect("node 3 must exist");
            prop_assert_eq!(finish_node.id.get(), 3,
                "CollectFinish must have ID 3 (id+3)");
            prop_assert!(
                matches!(&finish_node.kind, CompiledNodeKind::CollectFinish { .. }),
                "node[3] must be CollectFinish, got {:?}", finish_node.kind
            );
        }
    }

    /// B-013: When collect is not the first step, the 4 collect nodes
    /// still have consecutive IDs offset from the collect step's own
    /// StepIdx.
    #[test]
    fn collect_four_consecutive_ids_after_preamble(
        source in source_digit_str(),
        pages in any::<Option<u32>>(),
        items in any::<Option<u32>>(),
    ) {
        let yaml = collect_yaml_with_preamble(&source, pages, items);
        let result = compile_workflow(yaml.as_bytes());

        if result.is_ok() {
            let wf = result.unwrap();
            // Preamble takes IDs 0,1. Collect starts at ID 2.
            let base: u16 = 2;

            prop_assert!(wf.node_count() >= 6,
                "workflow with preamble+collect should have at least 6 nodes, got {}",
                wf.node_count());

            // CollectStart at base
            let start = node_at(&wf, base).expect("CollectStart must exist");
            prop_assert_eq!(start.id.get(), base);
            prop_assert!(
                matches!(&start.kind, CompiledNodeKind::CollectStart { .. }),
                "node at base must be CollectStart"
            );

            // SetConst at base+1
            let body_node = node_at(&wf, base + 1).expect("body SetConst must exist");
            prop_assert_eq!(body_node.id.get(), base + 1);
            prop_assert!(
                matches!(&body_node.kind, CompiledNodeKind::SetConst { .. }),
                "node at base+1 must be SetConst"
            );

            // CollectPage at base+2
            let page = node_at(&wf, base + 2).expect("CollectPage must exist");
            prop_assert_eq!(page.id.get(), base + 2);
            prop_assert!(
                matches!(&page.kind, CompiledNodeKind::CollectPage { .. }),
                "node at base+2 must be CollectPage"
            );

            // CollectFinish at base+3
            let finish_node = node_at(&wf, base + 3).expect("CollectFinish must exist");
            prop_assert_eq!(finish_node.id.get(), base + 3);
            prop_assert!(
                matches!(&finish_node.kind, CompiledNodeKind::CollectFinish { .. }),
                "node at base+3 must be CollectFinish"
            );
        }
    }
}

// =========================================================================
// Proptest: B-014 & B-015 — CollectStart fields match input + offsets
// =========================================================================

proptest! {
    /// B-014+B-015: CollectStart.limit and CollectStart.page_size match
    /// the input `pages` and `items` (or default to 1). Source slot is
    /// preserved through CollectPage.collector_slot and
    /// CollectFinish.collector_slot. Offsets: body=id+1, page=id+2, done=id+3.
    #[test]
    fn collect_start_fields_match_input_and_offsets_correct(
        source_digit in source_digit_str(),
        pages in any::<Option<u32>>(),
        items in any::<Option<u32>>(),
    ) {
        let yaml = collect_yaml(&source_digit, pages, items, "42");
        let result = compile_workflow(yaml.as_bytes());

        if result.is_ok() {
            let wf = result.unwrap();

            let start = node_at(&wf, 0).expect("node 0 must exist");
            match &start.kind {
                CompiledNodeKind::CollectStart {
                    source,
                    limit,
                    page_size,
                    body,
                    done,
                } => {
                    // Source slot parsed from the digit string
                    let expected_slot: u16 = source_digit.parse().unwrap_or(0);
                    prop_assert_eq!(source.get(), expected_slot,
                        "source slot must match parsed digit");
                    let expected_limit = pages.unwrap_or(1);
                    let expected_page_size = items.unwrap_or(1);
                    prop_assert_eq!(*limit, expected_limit,
                        "limit must match pages.unwrap_or(1)");
                    prop_assert_eq!(*page_size, expected_page_size,
                        "page_size must match items.unwrap_or(1)");

                    // B-015: body=id+1, done=id+3
                    prop_assert_eq!(body.get(), 1,
                        "CollectStart.body must be id+1");
                    prop_assert_eq!(done.get(), 3,
                        "CollectStart.done must be id+3");
                }
                other => prop_assert!(false,
                    "expected CollectStart, got {:?}", other),
            }

            // CollectPage has collector_slot == source, body+done offsets
            let page_node = node_at(&wf, 2).expect("CollectPage must exist");
            match &page_node.kind {
                CompiledNodeKind::CollectPage {
                    collector_slot,
                    body,
                    done,
                } => {
                    let expected_slot: u16 = source_digit.parse().unwrap_or(0);
                    prop_assert_eq!(collector_slot.get(), expected_slot,
                        "CollectPage.collector_slot must match source");
                    prop_assert_eq!(body.get(), 1,
                        "CollectPage.body = id+1");
                    prop_assert_eq!(done.get(), 3,
                        "CollectPage.done = id+3");
                }
                other => prop_assert!(false,
                    "expected CollectPage, got {:?}", other),
            }

            // CollectFinish has collector_slot == source
            let finish_node = node_at(&wf, 3).expect("CollectFinish must exist");
            match &finish_node.kind {
                CompiledNodeKind::CollectFinish { collector_slot } => {
                    let expected_slot: u16 = source_digit.parse().unwrap_or(0);
                    prop_assert_eq!(collector_slot.get(), expected_slot,
                        "CollectFinish.collector_slot must match source");
                }
                other => prop_assert!(false,
                    "expected CollectFinish, got {:?}", other),
            }
        }
    }
}

// =========================================================================
// Deterministic tests: B-016 — Budget defaults
// =========================================================================

#[test]
fn collect_defaults_limit_and_page_size_to_one() {
    let yaml = collect_yaml_no_budget();
    let result = compile_workflow(yaml.as_bytes());

    assert!(result.is_ok(), "collect without budget should compile");
    let wf = result.unwrap();

    let start = node_at(&wf, 0).expect("CollectStart must exist");
    match &start.kind {
        CompiledNodeKind::CollectStart {
            limit, page_size, ..
        } => {
            assert_eq!(*limit, 1, "default limit (pages) should be 1 when omitted");
            assert_eq!(
                *page_size, 1,
                "default page_size (items) should be 1 when omitted"
            );
        }
        other => assert!(false, "expected CollectStart, got {other:?}"),
    }
}

// =========================================================================
// Deterministic tests: B-017 — Max valid start ID
// =========================================================================

#[test]
fn step_idx_checked_add_three_at_boundary() {
    // u16::MAX - 3 = 65532: should succeed
    let valid = StepIdx::new(65532);
    let result = valid.checked_add(3);
    assert!(
        result.is_some(),
        "id = 65532 (u16::MAX - 3) should allow +3 offset"
    );
    if let Some(stepped) = result {
        assert_eq!(stepped.get(), 65535, "65532 + 3 = 65535 (u16::MAX)");
    }

    // u16::MAX - 2 = 65533: should fail
    let overflow = StepIdx::new(65533);
    assert!(
        overflow.checked_add(3).is_none(),
        "id = 65533 (u16::MAX - 2) must return None for +3 offset"
    );
}

#[test]
fn step_idx_max_is_u16_max_minus_three_for_offset_three() {
    // Explicitly prove that the maximum safe start ID for a +3 offset is
    // exactly u16::MAX - 3.
    for offset in 0u16..=3u16 {
        let max_id = u16::MAX.saturating_sub(offset);
        let step = StepIdx::new(max_id);
        let result = step.checked_add(offset);
        assert!(
            result.is_some(),
            "id={max_id} should allow +{offset} offset"
        );
        if let Some(s) = result {
            assert_eq!(s.get(), max_id.saturating_add(offset));
        }
    }
    // One beyond: fails
    let over = StepIdx::new(u16::MAX.wrapping_sub(2)); // 65533
    assert!(
        over.checked_add(3).is_none(),
        "id=65533 must fail for +3 offset"
    );
}

/// B-017: With a moderate preamble (100 steps), verify the collect step
/// still computes correct body/page/done offsets. The type-level boundary
/// at u16::MAX - 3 is tested by `step_idx_checked_add_three_at_boundary`
/// and `step_idx_max_is_u16_max_minus_three_for_offset_three`.
#[test]
fn collect_with_moderate_preamble_has_correct_offsets() {
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: collect-mod\nwhen:\n  manual: {}\nsteps:\n",
    );
    // 100 preamble set steps consume IDs 0..99
    for i in 0u16..100u16 {
        yaml.push_str(&format!(
            "  - id: pre_{i}\n    set:\n      output: v_{i}\n      value: \"1\"\n"
        ));
    }
    // Collect step at ID 100
    yaml.push_str("  - id: collect_pages\n    collect:\n");
    yaml.push_str("      variable: page\n");
    yaml.push_str("      source: \"0\"\n");
    yaml.push_str("      steps:\n");
    yaml.push_str("        - id: remember_page\n");
    yaml.push_str("          set:\n");
    yaml.push_str("            output: page_seen\n");
    yaml.push_str("            value: \"7\"\n");
    // Finish step at ID 104
    yaml.push_str("  - id: done\n    finish:\n      result: 0\n");

    let result = compile_workflow(yaml.as_bytes());
    assert!(
        result.is_ok(),
        "collect after 100-step preamble should compile: {:?}",
        result.err()
    );
    let wf = result.unwrap();

    // CollectStart at ID 100
    let start = node_at(&wf, 100).expect("CollectStart must exist at id 100");
    assert!(matches!(&start.kind, CompiledNodeKind::CollectStart { .. }));
    assert_eq!(start.id.get(), 100);

    // SetConst at ID 101
    let set_node = node_at(&wf, 101).expect("SetConst must exist at id 101");
    assert!(matches!(&set_node.kind, CompiledNodeKind::SetConst { .. }));

    // CollectPage at ID 102
    let page = node_at(&wf, 102).expect("CollectPage must exist at id 102");
    assert!(matches!(&page.kind, CompiledNodeKind::CollectPage { .. }));

    // CollectFinish at ID 103
    let finish_node = node_at(&wf, 103).expect("CollectFinish must exist at id 103");
    assert!(matches!(
        &finish_node.kind,
        CompiledNodeKind::CollectFinish { .. }
    ));

    match &start.kind {
        CompiledNodeKind::CollectStart { body, done, .. } => {
            assert_eq!(body.get(), 101, "body = id+1 = 101");
            assert_eq!(done.get(), 103, "done = id+3 = 103");
        }
        _ => assert!(false, "expected CollectStart"),
    }
}

// =========================================================================
// Deterministic tests for lower_collect (public API)
// =========================================================================

#[test]
fn lower_collect_emits_three_nodes() {
    let mut builder = SlotCompiler::new();
    let id = StepIdx::new(10);
    let source = SlotIdx::new(7);
    let body = StepIdx::new(11);
    let done = StepIdx::new(12);

    let result = lower_collect(id, source, 5, 10, body, done, &mut builder);
    assert!(result.is_ok(), "lower_collect should succeed");
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 3, "lower_collect emits exactly 3 nodes");
}

#[test]
fn lower_collect_node_ids_are_id_body_done() {
    let mut builder = SlotCompiler::new();
    let id = StepIdx::new(10);
    let body = StepIdx::new(11);
    let done = StepIdx::new(12);

    let result = lower_collect(id, SlotIdx::new(0), 5, 10, body, done, &mut builder);
    let nodes = result.unwrap();

    assert_eq!(nodes[0].id.get(), 10, "CollectStart at id");
    assert_eq!(nodes[1].id.get(), 11, "CollectPage at body");
    assert_eq!(nodes[2].id.get(), 12, "CollectFinish at done");
}

#[test]
fn lower_collect_collectstart_has_correct_fields() {
    let mut builder = SlotCompiler::new();
    let id = StepIdx::new(10);
    let source = SlotIdx::new(7);
    let limit = 5u32;
    let page_size = 10u32;
    let body = StepIdx::new(11);
    let done = StepIdx::new(12);

    let result = lower_collect(id, source, limit, page_size, body, done, &mut builder);
    let nodes = result.unwrap();

    match &nodes[0].kind {
        CompiledNodeKind::CollectStart {
            source: s,
            limit: l,
            page_size: p,
            body: b,
            done: d,
        } => {
            assert_eq!(s.get(), 7, "source slot preserved");
            assert_eq!(*l, 5, "limit preserved");
            assert_eq!(*p, 10, "page_size preserved");
            assert_eq!(b.get(), 11, "body step preserved");
            assert_eq!(d.get(), 12, "done step preserved");
        }
        other => assert!(false, "expected CollectStart, got {other:?}"),
    }
}

#[test]
fn lower_collect_collectpage_has_correct_slot_reference() {
    let mut builder = SlotCompiler::new();
    let id = StepIdx::new(10);
    let source = SlotIdx::new(7);
    let body = StepIdx::new(11);
    let done = StepIdx::new(12);

    let result = lower_collect(id, source, 3, 4, body, done, &mut builder);
    let nodes = result.unwrap();

    match &nodes[1].kind {
        CompiledNodeKind::CollectPage {
            collector_slot,
            body: b,
            done: d,
        } => {
            assert_eq!(
                collector_slot.get(),
                7,
                "CollectPage.collector_slot matches source"
            );
            assert_eq!(b.get(), 11, "CollectPage.body preserved");
            assert_eq!(d.get(), 12, "CollectPage.done preserved");
        }
        other => assert!(false, "expected CollectPage, got {other:?}"),
    }
}

#[test]
fn lower_collect_collectfinish_has_correct_slot_reference() {
    let mut builder = SlotCompiler::new();
    let id = StepIdx::new(10);
    let source = SlotIdx::new(7);
    let body = StepIdx::new(11);
    let done = StepIdx::new(12);

    let result = lower_collect(id, source, 3, 4, body, done, &mut builder);
    let nodes = result.unwrap();

    match &nodes[2].kind {
        CompiledNodeKind::CollectFinish { collector_slot } => {
            assert_eq!(
                collector_slot.get(),
                7,
                "CollectFinish.collector_slot matches source"
            );
        }
        other => assert!(false, "expected CollectFinish, got {other:?}"),
    }
}

#[test]
fn lower_collect_accepts_zero_limit_and_page_size() {
    let mut builder = SlotCompiler::new();
    let result = lower_collect(
        StepIdx::new(10),
        SlotIdx::new(0),
        0,
        0,
        StepIdx::new(11),
        StepIdx::new(12),
        &mut builder,
    );
    assert!(
        result.is_ok(),
        "lower_collect should accept zero limit/page_size"
    );
}

#[test]
fn lower_collect_accepts_u32_max_values() {
    let mut builder = SlotCompiler::new();
    let result = lower_collect(
        StepIdx::new(10),
        SlotIdx::new(0),
        u32::MAX,
        u32::MAX,
        StepIdx::new(11),
        StepIdx::new(12),
        &mut builder,
    );
    assert!(
        result.is_ok(),
        "lower_collect should accept u32::MAX limit/page_size"
    );
}

#[test]
fn lower_collect_preserves_source_in_builder() {
    // Verify that record_slot is called on the source within lower_collect.
    let mut builder = SlotCompiler::new();
    let source = SlotIdx::new(42);
    let result = lower_collect(
        StepIdx::new(10),
        source,
        1,
        1,
        StepIdx::new(11),
        StepIdx::new(12),
        &mut builder,
    );
    assert!(result.is_ok());
    // The builder should have recorded the slot
    let count = builder.slot_count();
    assert!(count.is_ok(), "slot_count should be valid: {count:?}");
    // source slot 42 means slot_count should be at least 43
    if let Ok(c) = count {
        assert!(
            c >= 43,
            "slot_count should be at least source.get()+1=43, got {c}"
        );
    }
}
