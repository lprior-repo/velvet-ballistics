//! Proptest coverage for vb-pkif2: Save canonical name aliasing invariant.
//!
//! After the fix, `canonical_primitive_name(StepPrimitive::Save { .. })` must
//! return `"set"`, making it digest-identical with the Set arm (master §25).

use proptest::prelude::*;
use vb_yaml::ast::{ScalarValue, StepPrimitive};

proptest! {
    #[test]
    fn save_canonical_name_is_set(_v in prop_oneof![
        any::<String>().prop_map(|s| ScalarValue::String(s)),
        any::<i64>().prop_map(ScalarValue::Integer),
    ]) {
        // After fix: Save returns "set", not "save"
        let save = StepPrimitive::Save {
            value: _v.clone(),
        };
        let result = crate::mod_compile_lowering::canonical_primitive_name(&save);
        assert_eq!(result, "set", "Save canonical name must be \"set\" for aliasing invariant");
    }

    #[test]
    fn set_canonical_name_is_set() {
        let set = StepPrimitive::Set {
            output: "x".to_string(),
            value: vb_yaml::ast::ScalarValue::String("1".to_string()),
        };
        let result = crate::mod_compile_lowering::canonical_primitive_name(&set);
        assert_eq!(result, "set", "Set canonical name must be \"set\"");
    }

    #[test]
    fn save_and_set_have_same_canonical_name() {
        let save = StepPrimitive::Save {
            value: ScalarValue::String("hello".to_string()),
        };
        let set = StepPrimitive::Set {
            output: "x".to_string(),
            value: ScalarValue::String("hello".to_string()),
        };
        let save_name = crate::mod_compile_lowering::canonical_primitive_name(&save);
        let set_name = crate::mod_compile_lowering::canonical_primitive_name(&set);
        assert_eq!(save_name, set_name, "Save and Set must have identical canonical names");
    }
}
