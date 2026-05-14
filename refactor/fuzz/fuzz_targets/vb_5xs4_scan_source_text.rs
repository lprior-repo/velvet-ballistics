#![no_main]

use libfuzzer_sys::fuzz_target;
use velvet_ballastics_workspace::quality::test_loop_inventory::{
    SourceText, TestFile, scan_test_file,
};

fuzz_target!(|data: &[u8]| {
    // Given hostile bytes from the public source-text boundary.
    let text = match String::from_utf8(data.to_vec()) {
        Ok(value) => SourceText::Text(value),
        Err(_) => SourceText::InvalidUtf8 { byte_offset: 3 },
    };

    // When scanning through the contracted public API.
    let _result = scan_test_file(TestFile::new("tests/fuzz_candidate.rs"), text);
});
