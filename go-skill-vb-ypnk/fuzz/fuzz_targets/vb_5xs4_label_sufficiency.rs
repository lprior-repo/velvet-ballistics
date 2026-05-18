#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_boundary_inventory::quality::test_loop_inventory::{
    CaseLabel, LabelEvidence, LabelingPolicy, Location, LoopPattern, LoopPatternKind,
    classify_loop_pattern,
};

fuzz_target!(|label: String| {
    // Given arbitrary assertion label text projected into public label evidence.
    let pattern = LoopPattern::new(
        "tests/fuzz_label.rs",
        Location::new(7, 5),
        LoopPatternKind::TableLoop,
        1,
        LabelEvidence::CaseOnly {
            case: CaseLabel::new(&label),
        },
    );

    // When classifying through the contracted public API.
    let _result = classify_loop_pattern(pattern, LabelingPolicy::RequireBehaviorAndCaseIdentity);
});
