#![no_main]

use libfuzzer_sys::fuzz_target;
use velvet_ballastics_workspace::quality::test_loop_inventory::{
    MutationEvidence, ValidatedInventory, render_inventory_report,
};

fuzz_target!(|claim: Option<String>| {
    // Given arbitrary report mutation-claim input with absent mutation evidence.
    let inventory = ValidatedInventory::with_findings(vec![], MutationEvidence::NotProvided, claim);

    // When rendering through the public API.
    if let Ok(validated) = inventory {
        let _result = render_inventory_report(validated);
    }
});
