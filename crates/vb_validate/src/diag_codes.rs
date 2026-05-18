#![forbid(unsafe_code)]
//! Stable diagnostic error codes (Section 16 of the master contract).

#![allow(unreachable_pub)]
/// Schema validation errors: E01xx.
pub const CODE_DUPLICATE_KEY: u16 = 0x0101;
pub const CODE_FORBIDDEN_YAML_FEATURE: u16 = 0x0102;
pub const CODE_UNKNOWN_TOP_LEVEL_FIELD: u16 = 0x0103;
pub const CODE_UNKNOWN_STEP_FIELD: u16 = 0x0104;
pub const CODE_MISSING_REQUIRED_FIELD: u16 = 0x0105;
pub const CODE_INVALID_VERSION: u16 = 0x0106;
pub const CODE_INVALID_ID: u16 = 0x0107;
pub const CODE_RESERVED_ID: u16 = 0x0108;
pub const CODE_DUPLICATE_ID: u16 = 0x0109;
pub const CODE_MULTIPLE_STEP_PRIMITIVES: u16 = 0x010A;
pub const CODE_MISSING_STEP_PRIMITIVE: u16 = 0x010B;

/// Reference validation errors: E02xx.
pub const CODE_UNKNOWN_REFERENCE: u16 = 0x0201;
pub const CODE_FUTURE_REFERENCE: u16 = 0x0202;
pub const CODE_SECRET_NOT_DECLARED: u16 = 0x0203;
pub const CODE_DIRECT_RUNTIME_REFERENCE: u16 = 0x0204;

/// Control-flow errors: E03xx.
pub const CODE_INVALID_THEN_TARGET: u16 = 0x0301;
pub const CODE_CONTROL_FLOW_CYCLE: u16 = 0x0302;
pub const CODE_UNREACHABLE_STEP: u16 = 0x0303;
pub const CODE_INVALID_CHOOSE: u16 = 0x0304;
pub const CODE_INVALID_FOR_EACH: u16 = 0x0305;
pub const CODE_INVALID_TOGETHER: u16 = 0x0306;
pub const CODE_INVALID_COLLECT: u16 = 0x0307;
pub const CODE_INVALID_REDUCE: u16 = 0x0308;
pub const CODE_INVALID_REPEAT: u16 = 0x0309;

/// Type/taint/resource errors: E04xx.
pub const CODE_INVALID_WAIT: u16 = 0x0401;
pub const CODE_INVALID_ASK: u16 = 0x0402;
pub const CODE_INVALID_FINISH: u16 = 0x0403;
pub const CODE_INVALID_RETRY: u16 = 0x0404;
pub const CODE_INVALID_ON_ERROR: u16 = 0x0405;
pub const CODE_SECRET_RESULT_LEAK: u16 = 0x0406;
pub const CODE_TYPE_MISMATCH: u16 = 0x0407;
pub const CODE_PAYLOAD_TOO_LARGE: u16 = 0x0408;
pub const CODE_LIMIT_REQUIRED: u16 = 0x0409;
pub const CODE_LIMIT_EXCEEDED: u16 = 0x040A;
pub const CODE_UNSUPPORTED_TRIGGER: u16 = 0x040B;
pub const CODE_HTTP_TRIGGER_OUT_OF_CORE: u16 = 0x040C;

/// Gate verifier errors: E05xx.
pub const CODE_EXPRESSION_STACK_EXCEEDED: u16 = 0x0501;
pub const CODE_EXPRESSION_STACK_MISMATCH: u16 = 0x0502;
pub const CODE_ACCESSOR_SLOT_OUT_OF_RANGE: u16 = 0x0503;
pub const CODE_ACCESSOR_PATH_INVALID: u16 = 0x0504;
pub const CODE_SLOT_REFERENCE_OUT_OF_RANGE: u16 = 0x0505;
pub const CODE_LOOP_BODY_STEP_OUT_OF_RANGE: u16 = 0x0506;
pub const CODE_SLOT_DEPENDENCY_CYCLE: u16 = 0x0507;
pub const CODE_NODE_KIND_CONSTRAINT_VIOLATION: u16 = 0x0508;
pub const CODE_ACTION_CONTRACT_MISSING: u16 = 0x0509;
pub const CODE_ACTION_CONTRACT_ORPHAN: u16 = 0x050A;
pub const CODE_SLOT_TYPE_INCONSISTENCY: u16 = 0x050B;
pub const CODE_NON_DETERMINISTIC_PATH: u16 = 0x050C;
pub const CODE_CAPABILITY_NAME_EMPTY: u16 = 0x050D;
pub const CODE_CAPABILITY_NAME_TOO_LONG: u16 = 0x050E;
pub const CODE_CAPABILITY_NAME_INVALID: u16 = 0x050F;
pub const CODE_CAPABILITY_ACTION_MISMATCH: u16 = 0x0510;
pub const CODE_CAPABILITY_DUPLICATE: u16 = 0x0511;
pub const CODE_ACCESSOR_PATH_TOO_DEEP: u16 = 0x0512;
pub const CODE_ACCESSOR_SYMBOL_OUT_OF_BOUNDS: u16 = 0x0513;

/// Contracts-as-data validation errors: E06xx.
pub const CODE_MISSING_SCHEMA_VERSION: u16 = 0x0601;
pub const CODE_INVALID_KIND: u16 = 0x0602;
pub const CODE_VERSION_MONOTONICITY_BREACH: u16 = 0x0603;
pub const CODE_CUE_VET_FAILED: u16 = 0x0604;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn all_codes() -> Vec<(&'static str, u16)> {
        vec![
            ("CODE_DUPLICATE_KEY", CODE_DUPLICATE_KEY),
            ("CODE_FORBIDDEN_YAML_FEATURE", CODE_FORBIDDEN_YAML_FEATURE),
            ("CODE_UNKNOWN_TOP_LEVEL_FIELD", CODE_UNKNOWN_TOP_LEVEL_FIELD),
            ("CODE_UNKNOWN_STEP_FIELD", CODE_UNKNOWN_STEP_FIELD),
            ("CODE_MISSING_REQUIRED_FIELD", CODE_MISSING_REQUIRED_FIELD),
            ("CODE_INVALID_VERSION", CODE_INVALID_VERSION),
            ("CODE_INVALID_ID", CODE_INVALID_ID),
            ("CODE_RESERVED_ID", CODE_RESERVED_ID),
            ("CODE_DUPLICATE_ID", CODE_DUPLICATE_ID),
            (
                "CODE_MULTIPLE_STEP_PRIMITIVES",
                CODE_MULTIPLE_STEP_PRIMITIVES,
            ),
            ("CODE_MISSING_STEP_PRIMITIVE", CODE_MISSING_STEP_PRIMITIVE),
            ("CODE_UNKNOWN_REFERENCE", CODE_UNKNOWN_REFERENCE),
            ("CODE_FUTURE_REFERENCE", CODE_FUTURE_REFERENCE),
            ("CODE_SECRET_NOT_DECLARED", CODE_SECRET_NOT_DECLARED),
            (
                "CODE_DIRECT_RUNTIME_REFERENCE",
                CODE_DIRECT_RUNTIME_REFERENCE,
            ),
            ("CODE_INVALID_THEN_TARGET", CODE_INVALID_THEN_TARGET),
            ("CODE_CONTROL_FLOW_CYCLE", CODE_CONTROL_FLOW_CYCLE),
            ("CODE_UNREACHABLE_STEP", CODE_UNREACHABLE_STEP),
            ("CODE_INVALID_CHOOSE", CODE_INVALID_CHOOSE),
            ("CODE_INVALID_FOR_EACH", CODE_INVALID_FOR_EACH),
            ("CODE_INVALID_TOGETHER", CODE_INVALID_TOGETHER),
            ("CODE_INVALID_COLLECT", CODE_INVALID_COLLECT),
            ("CODE_INVALID_REDUCE", CODE_INVALID_REDUCE),
            ("CODE_INVALID_REPEAT", CODE_INVALID_REPEAT),
            ("CODE_INVALID_WAIT", CODE_INVALID_WAIT),
            ("CODE_INVALID_ASK", CODE_INVALID_ASK),
            ("CODE_INVALID_FINISH", CODE_INVALID_FINISH),
            ("CODE_INVALID_RETRY", CODE_INVALID_RETRY),
            ("CODE_INVALID_ON_ERROR", CODE_INVALID_ON_ERROR),
            ("CODE_SECRET_RESULT_LEAK", CODE_SECRET_RESULT_LEAK),
            ("CODE_TYPE_MISMATCH", CODE_TYPE_MISMATCH),
            ("CODE_PAYLOAD_TOO_LARGE", CODE_PAYLOAD_TOO_LARGE),
            ("CODE_LIMIT_REQUIRED", CODE_LIMIT_REQUIRED),
            ("CODE_LIMIT_EXCEEDED", CODE_LIMIT_EXCEEDED),
            ("CODE_UNSUPPORTED_TRIGGER", CODE_UNSUPPORTED_TRIGGER),
            (
                "CODE_HTTP_TRIGGER_OUT_OF_CORE",
                CODE_HTTP_TRIGGER_OUT_OF_CORE,
            ),
            (
                "CODE_EXPRESSION_STACK_EXCEEDED",
                CODE_EXPRESSION_STACK_EXCEEDED,
            ),
            (
                "CODE_EXPRESSION_STACK_MISMATCH",
                CODE_EXPRESSION_STACK_MISMATCH,
            ),
            (
                "CODE_ACCESSOR_SLOT_OUT_OF_RANGE",
                CODE_ACCESSOR_SLOT_OUT_OF_RANGE,
            ),
            ("CODE_ACCESSOR_PATH_INVALID", CODE_ACCESSOR_PATH_INVALID),
            (
                "CODE_SLOT_REFERENCE_OUT_OF_RANGE",
                CODE_SLOT_REFERENCE_OUT_OF_RANGE,
            ),
            (
                "CODE_LOOP_BODY_STEP_OUT_OF_RANGE",
                CODE_LOOP_BODY_STEP_OUT_OF_RANGE,
            ),
            ("CODE_SLOT_DEPENDENCY_CYCLE", CODE_SLOT_DEPENDENCY_CYCLE),
            (
                "CODE_NODE_KIND_CONSTRAINT_VIOLATION",
                CODE_NODE_KIND_CONSTRAINT_VIOLATION,
            ),
            ("CODE_ACTION_CONTRACT_MISSING", CODE_ACTION_CONTRACT_MISSING),
            ("CODE_ACTION_CONTRACT_ORPHAN", CODE_ACTION_CONTRACT_ORPHAN),
            ("CODE_SLOT_TYPE_INCONSISTENCY", CODE_SLOT_TYPE_INCONSISTENCY),
            ("CODE_NON_DETERMINISTIC_PATH", CODE_NON_DETERMINISTIC_PATH),
            ("CODE_CAPABILITY_NAME_EMPTY", CODE_CAPABILITY_NAME_EMPTY),
            (
                "CODE_CAPABILITY_NAME_TOO_LONG",
                CODE_CAPABILITY_NAME_TOO_LONG,
            ),
            ("CODE_CAPABILITY_NAME_INVALID", CODE_CAPABILITY_NAME_INVALID),
            (
                "CODE_CAPABILITY_ACTION_MISMATCH",
                CODE_CAPABILITY_ACTION_MISMATCH,
            ),
            ("CODE_CAPABILITY_DUPLICATE", CODE_CAPABILITY_DUPLICATE),
            ("CODE_ACCESSOR_PATH_TOO_DEEP", CODE_ACCESSOR_PATH_TOO_DEEP),
            (
                "CODE_ACCESSOR_SYMBOL_OUT_OF_BOUNDS",
                CODE_ACCESSOR_SYMBOL_OUT_OF_BOUNDS,
            ),
            ("CODE_MISSING_SCHEMA_VERSION", CODE_MISSING_SCHEMA_VERSION),
            ("CODE_INVALID_KIND", CODE_INVALID_KIND),
            (
                "CODE_VERSION_MONOTONICITY_BREACH",
                CODE_VERSION_MONOTONICITY_BREACH,
            ),
            ("CODE_CUE_VET_FAILED", CODE_CUE_VET_FAILED),
        ]
    }

    #[test]
    fn all_codes_are_non_zero() {
        for (name, code) in all_codes() {
            assert_ne!(code, 0, "{name} should not be zero");
        }
    }

    #[test]
    fn all_codes_are_unique() {
        let codes = all_codes();
        let mut seen = BTreeSet::new();
        for (name, code) in codes {
            assert!(seen.insert(code), "duplicate code {code:#06x} for {name}");
        }
    }

    #[test]
    fn schema_codes_are_in_e01xx_range() {
        let schema_codes = [
            CODE_DUPLICATE_KEY,
            CODE_FORBIDDEN_YAML_FEATURE,
            CODE_UNKNOWN_TOP_LEVEL_FIELD,
            CODE_UNKNOWN_STEP_FIELD,
            CODE_MISSING_REQUIRED_FIELD,
            CODE_INVALID_VERSION,
            CODE_INVALID_ID,
            CODE_RESERVED_ID,
            CODE_DUPLICATE_ID,
            CODE_MULTIPLE_STEP_PRIMITIVES,
            CODE_MISSING_STEP_PRIMITIVE,
        ];
        for code in schema_codes {
            let high = (code >> 8) & 0xFF;
            assert_eq!(
                high, 0x01,
                "schema code {code:#06x} should be in E01xx range"
            );
        }
    }

    #[test]
    fn reference_codes_are_in_e02xx_range() {
        let ref_codes = [
            CODE_UNKNOWN_REFERENCE,
            CODE_FUTURE_REFERENCE,
            CODE_SECRET_NOT_DECLARED,
            CODE_DIRECT_RUNTIME_REFERENCE,
        ];
        for code in ref_codes {
            let high = (code >> 8) & 0xFF;
            assert_eq!(
                high, 0x02,
                "reference code {code:#06x} should be in E02xx range"
            );
        }
    }

    #[test]
    fn control_flow_codes_are_in_e03xx_range() {
        let cf_codes = [
            CODE_INVALID_THEN_TARGET,
            CODE_CONTROL_FLOW_CYCLE,
            CODE_UNREACHABLE_STEP,
            CODE_INVALID_CHOOSE,
            CODE_INVALID_FOR_EACH,
            CODE_INVALID_TOGETHER,
            CODE_INVALID_COLLECT,
            CODE_INVALID_REDUCE,
            CODE_INVALID_REPEAT,
        ];
        for code in cf_codes {
            let high = (code >> 8) & 0xFF;
            assert_eq!(
                high, 0x03,
                "control-flow code {code:#06x} should be in E03xx range"
            );
        }
    }

    #[test]
    fn type_taint_codes_are_in_e04xx_range() {
        let tt_codes = [
            CODE_INVALID_WAIT,
            CODE_INVALID_ASK,
            CODE_INVALID_FINISH,
            CODE_INVALID_RETRY,
            CODE_INVALID_ON_ERROR,
            CODE_SECRET_RESULT_LEAK,
            CODE_TYPE_MISMATCH,
            CODE_PAYLOAD_TOO_LARGE,
            CODE_LIMIT_REQUIRED,
            CODE_LIMIT_EXCEEDED,
            CODE_UNSUPPORTED_TRIGGER,
            CODE_HTTP_TRIGGER_OUT_OF_CORE,
        ];
        for code in tt_codes {
            let high = (code >> 8) & 0xFF;
            assert_eq!(
                high, 0x04,
                "type/taint code {code:#06x} should be in E04xx range"
            );
        }
    }

    #[test]
    fn gate_codes_are_in_e05xx_range() {
        let gate_codes = [
            CODE_EXPRESSION_STACK_EXCEEDED,
            CODE_EXPRESSION_STACK_MISMATCH,
            CODE_ACCESSOR_SLOT_OUT_OF_RANGE,
            CODE_ACCESSOR_PATH_INVALID,
            CODE_SLOT_REFERENCE_OUT_OF_RANGE,
            CODE_LOOP_BODY_STEP_OUT_OF_RANGE,
            CODE_SLOT_DEPENDENCY_CYCLE,
            CODE_NODE_KIND_CONSTRAINT_VIOLATION,
            CODE_ACTION_CONTRACT_MISSING,
            CODE_ACTION_CONTRACT_ORPHAN,
            CODE_SLOT_TYPE_INCONSISTENCY,
            CODE_NON_DETERMINISTIC_PATH,
            CODE_CAPABILITY_NAME_EMPTY,
            CODE_CAPABILITY_NAME_TOO_LONG,
            CODE_CAPABILITY_NAME_INVALID,
            CODE_CAPABILITY_ACTION_MISMATCH,
            CODE_CAPABILITY_DUPLICATE,
            CODE_ACCESSOR_PATH_TOO_DEEP,
            CODE_ACCESSOR_SYMBOL_OUT_OF_BOUNDS,
        ];
        for code in gate_codes {
            let high = (code >> 8) & 0xFF;
            assert_eq!(high, 0x05, "gate code {code:#06x} should be in E05xx range");
        }
    }

    #[test]
    fn contracts_codes_are_in_e06xx_range() {
        let contracts_codes = [
            CODE_MISSING_SCHEMA_VERSION,
            CODE_INVALID_KIND,
            CODE_VERSION_MONOTONICITY_BREACH,
            CODE_CUE_VET_FAILED,
        ];
        for code in contracts_codes {
            let high = (code >> 8) & 0xFF;
            assert_eq!(
                high, 0x06,
                "contracts code {code:#06x} should be in E06xx range"
            );
        }
    }

    #[test]
    fn code_count_matches_total() {
        let codes = all_codes();
        assert_eq!(codes.len(), 59, "expected exactly 59 diagnostic codes");
    }
}
