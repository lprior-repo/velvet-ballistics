use super::encode_yaml;
use vb_core::workflow::{WorkflowIrV2, WorkflowParts};

use super::versioned_test_support as support;
use support::TestResult;

#[test]
fn versioned_yaml_preserves_legacy_shape() -> TestResult {
    let workflow = support::legacy()?;
    let yaml = encode_yaml(&workflow)?;
    assert_eq!(yaml, serde_saphyr::to_string(&workflow.to_parts())?);
    assert_eq!(
        serde_saphyr::from_str::<WorkflowParts>(&yaml)?,
        workflow.to_parts()
    );
    Ok(())
}

#[test]
fn versioned_yaml_empty_binding_is_unwrapped_v2() -> TestResult {
    let workflow = support::bound(&[])?;
    let decoded: WorkflowIrV2 = serde_saphyr::from_str(&encode_yaml(&workflow)?)?;
    assert_eq!(decoded.wire_version, 2);
    assert_eq!(decoded.parts, workflow.to_parts());
    assert_eq!(decoded.symbol_payloads, support::symbols(&[]));
    Ok(())
}

#[test]
fn versioned_yaml_retains_exact_ordered_payloads() -> TestResult {
    let workflow = support::bound(&["key", "é", " "])?;
    let decoded: WorkflowIrV2 = serde_saphyr::from_str(&encode_yaml(&workflow)?)?;
    assert_eq!(decoded.wire_version, 2);
    assert_eq!(decoded.parts, workflow.to_parts());
    assert_eq!(
        decoded.symbol_payloads,
        support::symbols(&["key", "é", " "])
    );
    Ok(())
}
