#![no_main]

use libfuzzer_sys::fuzz_target;
use velvet_ballastics_workspace::quality::test_loop_inventory::{
    SourceText, TestFile, scan_test_file,
};

fuzz_target!(|source: String| {
    // Given Rust-like macro/generated source text.
    let text = SourceText::Text(source);

    // When scanning source mappings through the public API.
    let _result = scan_test_file(TestFile::new("tests/untraceable_generated_loop.rs"), text);
});
