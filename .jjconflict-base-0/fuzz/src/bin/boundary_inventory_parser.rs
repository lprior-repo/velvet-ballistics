#![no_main]

use libfuzzer_sys::fuzz_target;
use velvet_ballastics_workspace::boundary_inventory::parse_inventory;

fuzz_target!(|data: &[u8]| {
    let _result = parse_inventory(data);
});
