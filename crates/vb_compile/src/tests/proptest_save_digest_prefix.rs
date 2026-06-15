//! Proptest coverage for vb-pkif2: Save digest prefix invariant.
//!
//! After the fix, `digest_step_primitive(StepPrimitive::Save { .. })` must
//! start with `b"set"`, making it digest-identical with the Set arm
//! (master §25: Save and Set must be digest-equivalent).

use proptest::prelude::*;
use vb_yaml::ast::ScalarValue;

fn save_digest_bytes() -> impl Strategy<Value = Vec<u8>> {
    any::<String>().prop_map(|s| s.into_bytes())
}

proptest! {
    #[test]
    fn save_digest_prefix_is_set(data in save_digest_bytes()) {
        let save = vb_yaml::ast::StepPrimitive::Save { value: ScalarValue::String(String::from_utf8_lossy(&data).to_string()) };
        let digest = crate::mod_compile_lowering::digest_step_primitive(&save);
        let prefix = "set".as_bytes();
        assert!(
            digest.starts_with(prefix),
            "Save digest must start with b\"set\" (prefix: {:?}, got: {:?})",
            prefix,
            digest
        );
    }

    #[test]
    fn set_digest_prefix_is_set() {
        let set = vb_yaml::ast::StepPrimitive::Set {
            output: "x".to_string(),
            value: ScalarValue::String("any".to_string()),
        };
        let digest = crate::mod_compile_lowering::digest_step_primitive(&set);
        let prefix = "set".as_bytes();
        assert!(
            digest.starts_with(prefix),
            "Set digest must start with b\"set\" (got: {:?})",
            digest
        );
    }

    #[test]
    fn save_and_set_digest_start_with_same_prefix() {
        let save = vb_yaml::ast::StepPrimitive::Save {
            value: ScalarValue::String("test_value".to_string()),
        };
        let set = vb_yaml::ast::StepPrimitive::Set {
            output: "y".to_string(),
            value: ScalarValue::String("test_value".to_string()),
        };
        let save_digest = crate::mod_compile_lowering::digest_step_primitive(&save);
        let set_digest = crate::mod_compile_lowering::digest_step_primitive(&set);
        let prefix_len = 3; // "set" tag bytes
        assert!(
            save_digest.starts_with(&set_digest[..prefix_len.min(set_digest.len())]),
            "Save digest must start with the same {} bytes as Set digest (save: {:?}, set: {:?})",
            prefix_len,
            &save_digest[..prefix_len.min(save_digest.len())],
            &set_digest[..prefix_len.min(set_digest.len())]
        );
    }

    #[test]
    fn save_digest_does_not_start_with_save_bytes() {
        let save = vb_yaml::ast::StepPrimitive::Save {
            value: ScalarValue::String("anything".to_string()),
        };
        let digest = crate::mod_compile_lowering::digest_step_primitive(&save);
        let old_prefix = "save".as_bytes();
        assert!(
            !digest.starts_with(old_prefix),
            "Save digest must NOT start with b\"save\" after fix (got: {:?})",
            digest
        );
    }
}
