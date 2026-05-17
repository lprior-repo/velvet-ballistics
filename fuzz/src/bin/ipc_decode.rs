//! Fuzz target: ipc_decode.

#[cfg(feature = "fuzz")]
fn main() {
    // Hand off to the libfuzzer entry point directly
    // The ipc_decode module is compiled into the fuzz binary via the lib.rs module include
    extern "C" {
        fn LLVMFuzzerTestOneInputIpcDecodeHeader(data: *const u8, len: usize) -> i32;
        fn LLVMFuzzerTestOneInputIpcDecodeFrame(data: *const u8, len: usize) -> i32;
        fn LLVMFuzzerTestOneInputIpcDecodeEdgeCases(data: *const u8, len: usize) -> i32;
    }

    let mut input = Vec::new();
    if std::io::Read::read_to_end(&mut std::io::stdin(), &mut input).is_ok() {
        if !input.is_empty() {
            unsafe {
                LLVMFuzzerTestOneInputIpcDecodeHeader(input.as_ptr(), input.len());
                LLVMFuzzerTestOneInputIpcDecodeFrame(input.as_ptr(), input.len());
                LLVMFuzzerTestOneInputIpcDecodeEdgeCases(input.as_ptr(), input.len());
            }
        }
    }
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
