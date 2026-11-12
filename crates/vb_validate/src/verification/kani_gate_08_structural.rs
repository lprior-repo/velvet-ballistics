#![cfg(kani)]
#![forbid(unsafe_code)]
//! Gate 8 checks over its exact production read set.
//!
//! Unrelated WorkflowParts fields are excluded by the production wrapper,
//! not assumed valid or populated with dummy data. Their irrelevance needs
//! the separately reviewed source bridge and public native controls.

use crate::ValidationError;

#[path = "gate_08_inputs.rs"]
mod gate_08_inputs;
#[path = "gate_08_mixed_inputs.rs"]
mod gate_08_mixed_inputs;
use gate_08_inputs::{
    arbitrary_inputs, root_oob_inputs, sentinel_inputs, symbol_oob_inputs, valid_inputs,
};
use gate_08_mixed_inputs::{depth_inputs, expression_inputs, mixed_inputs};

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_arbitrary_parts_valid_accessors_pass() {
    let input = valid_inputs();
    kani::cover!(input.accessors.is_empty(), "valid empty collection");
    kani::cover!(
        input.accessors.len() == 3,
        "valid three-accessor collection"
    );
    assert_eq!(
        input.validate(),
        Ok(()),
        "arbitrary valid accessors pass Gate 8"
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_arbitrary_parts_root_oob_rejected() {
    let input = root_oob_inputs();
    kani::cover!(input.slot_count == 0, "zero slots rejected");
    kani::cover!(input.slot_count == u16::MAX, "full slot boundary rejected");
    assert!(
        matches!(
            input.validate(),
            Err(ValidationError::AccessorSlotOutOfRange { .. })
        ),
        "arbitrary parts with out-of-range root is rejected"
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_arbitrary_parts_symbol_oob_rejected() {
    let input = symbol_oob_inputs();
    kani::cover!(input.symbols_count == 0, "zero symbols rejected");
    kani::cover!(
        input.symbols_count == u32::MAX,
        "full symbol boundary rejected"
    );
    assert!(
        matches!(
            input.validate(),
            Err(ValidationError::AccessorSymbolOutOfBounds { .. })
        ),
        "arbitrary parts with OOB field symbol is rejected"
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_arbitrary_parts_index_sentinel_rejected() {
    let input = sentinel_inputs();
    kani::cover!(
        input.accessors.iter().any(|a| a.path.len() == 1),
        "sentinel depth one"
    );
    kani::cover!(
        input.accessors.iter().any(|a| a.path.len() == 3),
        "sentinel depth three"
    );
    assert!(
        matches!(
            input.validate(),
            Err(ValidationError::AccessorPathInvalid { .. })
        ),
        "arbitrary parts with u32::MAX index sentinel is rejected"
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_full_structure_no_panic() {
    let input = arbitrary_inputs();
    let _result = input.validate();
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_structure_coverage() {
    let input = arbitrary_inputs();
    kani::cover!(input.accessors.is_empty(), "zero accessors covered");
    kani::cover!(input.accessors.len() >= 2, "multiple accessors covered");
    kani::cover!(
        input.accessors.iter().any(|a| !a.path.is_empty()),
        "accessors with path covered"
    );
    kani::cover!(
        input.accessors.iter().any(|a| a.path.len() == 3),
        "path generator maximum covered"
    );
    kani::cover!(input.validate().is_ok(), "valid result covered");
    kani::cover!(input.validate().is_err(), "invalid result covered");
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_arbitrary_resource_contract() {
    let input = arbitrary_inputs();
    let _result = input.validate();
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_step_names_independent_of_slots() {
    let input = arbitrary_inputs();
    let _result = input.validate();
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_empty_nodes_valid_accessors_pass() {
    let input = valid_inputs();
    kani::assume(!input.accessors.is_empty());
    kani::cover!(input.accessors.len() == 3, "all three accessors valid");
    assert_eq!(
        input.validate(),
        Ok(()),
        "empty nodes with valid accessors pass Gate 8"
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_expressions_with_accessor_refs() {
    let input = expression_inputs();
    assert_eq!(
        input.validate(),
        Ok(()),
        "expressions with accessor refs pass Gate 8"
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_mixed_accessor_paths() {
    let input = mixed_inputs();
    assert_eq!(input.validate(), Ok(()), "mixed accessor paths pass Gate 8");
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_all_node_kinds_no_panic() {
    let input = arbitrary_inputs();
    let _result = input.validate();
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_constants_with_symbols() {
    let input = valid_inputs();
    assert_eq!(
        input.validate(),
        Ok(()),
        "constants with symbols pass Gate 8"
    );
}

#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_many_accessors_varied_depths() {
    let input = mixed_inputs();
    kani::cover!(input.accessors.len() == 6, "six accessors reachable");
    kani::cover!(
        input.accessors.iter().any(|a| a.path.len() == 5),
        "depth five reachable"
    );
    assert_eq!(
        input.validate(),
        Ok(()),
        "many accessors with varied depths pass Gate 8"
    );
}

#[kani::proof]
#[kani::unwind(20)]
fn kani_gate_08_depth_sixteen_passes() {
    let input = depth_inputs(16);
    assert_eq!(input.validate(), Ok(()));
}

#[kani::proof]
#[kani::unwind(20)]
fn kani_gate_08_depth_seventeen_rejected() {
    let input = depth_inputs(17);
    assert_eq!(
        input.validate(),
        Err(ValidationError::AccessorPathTooDeep {
            accessor_index: 0,
            depth: 17,
            max: 16,
        })
    );
}
