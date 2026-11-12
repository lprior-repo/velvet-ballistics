//! Fuzz target for YAML compiler with strict profile validation.
//!
//! Command: cargo fuzz run vb_f04l_yaml_compiler_compile -- -max_len=65536 -runs=100000

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_compile::{CompileError, YamlCompiler};

fuzz_target!(|data: &[u8]| {
    let result = YamlCompiler::default().compile(data);
    if let Err(errors) = result {
        for error in errors.iter() {
            match error {
                CompileError::CanonicalYaml { .. }
                | CompileError::EmptySteps
                | CompileError::UnsupportedTopLevelDeclaration { .. }
                | CompileError::UnsupportedTopLevelResult
                | CompileError::UnsupportedStepControlField { .. }
                | CompileError::DuplicateStepId { .. }
                | CompileError::DuplicateOutputName { .. }
                | CompileError::UnknownOutputName { .. }
                | CompileError::StepFieldShape { .. }
                | CompileError::StepIndexOutOfRange { .. }
                | CompileError::SlotIndexOutOfRange { .. }
                | CompileError::PrimitiveLoweringLimitExceeded { .. }
                | CompileError::Workflow(_)
                | CompileError::UnsupportedStepPrimitive { .. } => {}
                _ => {}
            }
        }
    }
});
