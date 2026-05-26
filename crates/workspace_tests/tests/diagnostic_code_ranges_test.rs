use core::str::FromStr;

use vb_core::{CoreError, DiagnosticCode, SlotIdx, StepIdx};
use vb_ipc::IpcError;
use vb_runtime::RuntimeError;
use vb_storage::JournalError;
use vb_validate::ValidationError;
use vb_core::span::Span;

#[test]
fn diagnostic_code_ranges_are_globally_partitioned_by_crate() {
    assert_eq!(
        vb_validate::diagnostic::error_code(&ValidationError::DuplicateKey { span: Span::ZERO }),
        DiagnosticCode::new(0x0101)
    );
    assert_eq!(
        vb_validate::diagnostic::error_code(&ValidationError::HttpTriggerOutOfCore { span: Span::ZERO }),
        DiagnosticCode::new(0x040C)
    );
    assert_eq!(
        CoreError::InvalidProgramCounter {
            step: StepIdx::ZERO
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x1001)
    );
    assert_eq!(
        CoreError::SlotOutOfBounds {
            slot: SlotIdx::ZERO
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x1011)
    );
    assert_eq!(
        CoreError::TogetherBranchLimitExceeded { max: 1 }.diagnostic_code(),
        DiagnosticCode::new(0x1405)
    );
    assert_eq!(
        RuntimeError::QueueFull.diagnostic_code(),
        DiagnosticCode::new(0x2001)
    );
    assert_eq!(
        RuntimeError::UnsupportedFullRecoveryHydration.diagnostic_code(),
        DiagnosticCode::new(0x200D)
    );
    assert_eq!(
        IpcError::Full.diagnostic_code(),
        DiagnosticCode::new(0x3001)
    );
    assert_eq!(
        IpcError::ResponseDecodeFailed.diagnostic_code(),
        DiagnosticCode::new(0x300E)
    );
    assert_eq!(
        JournalError::KeyCapacity.diagnostic_code(),
        DiagnosticCode::new(0x4003)
    );
    assert_eq!(
        JournalError::PostcardDecodeFailed.diagnostic_code(),
        DiagnosticCode::new(0x4015)
    );
}

#[test]
fn diagnostic_code_parser_accepts_each_global_partition() {
    assert_eq!(
        DiagnosticCode::from_str("E040C"),
        Ok(DiagnosticCode::new(0x040C))
    );
    assert_eq!(
        DiagnosticCode::from_str("E1405"),
        Ok(DiagnosticCode::new(0x1405))
    );
    assert_eq!(
        DiagnosticCode::from_str("E200D"),
        Ok(DiagnosticCode::new(0x200D))
    );
    assert_eq!(
        DiagnosticCode::from_str("E300E"),
        Ok(DiagnosticCode::new(0x300E))
    );
    assert_eq!(
        DiagnosticCode::from_str("E4015"),
        Ok(DiagnosticCode::new(0x4015))
    );
}
