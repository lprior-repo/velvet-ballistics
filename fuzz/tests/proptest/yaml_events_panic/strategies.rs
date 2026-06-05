//! Strategy generators for YAML event proptest.
#![forbid(unsafe_code)]
use proptest::prelude::*;
use vb_core::diagnostic::HasSymbolicCode;

// ---------------------------------------------------------------------------
// Strategy: generate arbitrary UTF-8 strings of bounded length
// ---------------------------------------------------------------------------

/// UTF-8 string strategy bounded to 1 KiB for tractable test runs.
fn utf8_string_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::char::any(), 0..1024)
        .prop_map(|chars| chars.into_iter().collect::<String>())
}

/// UTF-8 string strategy for small inputs (≤ 256 bytes).
fn _small_utf8_string_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::char::any(), 0..256)
        .prop_map(|chars| chars.into_iter().collect::<String>())
}

// ---------------------------------------------------------------------------
// Strategy: generate valid YAML-like strings for non-empty event testing
// ---------------------------------------------------------------------------

/// Generates structurally valid YAML strings (mappings, sequences, scalars)
/// that should produce at least one event when parsed.
fn valid_yaml_strategy() -> impl Strategy<Value = String> {
    // Build strategies without cloning regex generators.
    let key_strat = proptest::string::string_regex(r"[a-zA-Z0-9_ ]{1,20}").unwrap();
    let val_strat = proptest::string::string_regex(r"[a-zA-Z0-9_ ]{1,20}").unwrap();

    let scalar_strat = proptest::string::string_regex(r"[a-zA-Z0-9_]{1,20}").unwrap();

    // Key-value pair strategy.
    let kv = key_strat.prop_flat_map(move |k| {
        let v = proptest::string::string_regex(r"[a-zA-Z0-9_ ]{1,20}").unwrap();
        v.prop_map(move |v| format!("{k}: {v}"))
    });

    // Sequence item strategy.
    let seq_item = scalar_strat.prop_map(|s| format!("- {s}"));

    // Nested mapping strategy.
    let nested = val_strat.prop_flat_map(move |parent| {
        let inner_kv = proptest::string::string_regex(r"[a-zA-Z0-9_ ]{1,10}").unwrap();
        let inner_v = proptest::string::string_regex(r"[a-zA-Z0-9_ ]{1,10}").unwrap();
        (inner_kv, inner_v, Just(parent)).prop_map(|(k, v, p)| format!("{p}:\n  {k}: {v}"))
    });

    prop_oneof![
        kv.boxed(),
        proptest::collection::vec(seq_item, 1..5)
            .prop_map(|items| items.join("\n"))
            .boxed(),
        nested.boxed(),
    ]
}

// ---------------------------------------------------------------------------
// Strategy: generate arbitrary YamlError variant values
// ---------------------------------------------------------------------------

/// Generates an arbitrary `vb_yaml::YamlError` variant with field data.
fn yaml_error_strategy() -> impl Strategy<Value = vb_yaml::YamlError> {
    let variant: BoxedStrategy<u8> = (0u8..21u8).boxed();

    variant.prop_flat_map(|v| match v {
        0 => Just(vb_yaml::YamlError::DuplicateKey {
            key: String::from("test_key").into_boxed_str(),
        })
        .boxed(),
        1 => (any::<String>())
            .prop_map(|s| {
                let leaked: &'static str = Box::leak(s.into_boxed_str());
                vb_yaml::YamlError::ForbiddenFeature { detail: leaked }
            })
            .boxed(),
        2 => Just(vb_yaml::YamlError::AnchorAliasMerge).boxed(),
        3 => (any::<String>())
            .prop_map(|s| vb_yaml::YamlError::CustomTag {
                tag: s.into_boxed_str(),
            })
            .boxed(),
        4 => Just(vb_yaml::YamlError::BinaryScalar).boxed(),
        5 => (0usize..100usize)
            .prop_map(|count| vb_yaml::YamlError::MultipleDocuments { count })
            .boxed(),
        6 => (any::<String>())
            .prop_map(|s| vb_yaml::YamlError::AmbiguousScalar {
                scalar: s.into_boxed_str(),
            })
            .boxed(),
        7 => ((0usize..10_000usize), (0usize..10_000usize))
            .prop_map(|(size, max)| vb_yaml::YamlError::SourceTooLarge { size, max })
            .boxed(),
        8 => ((0u16..255u16), (0u16..255u16))
            .prop_map(|(depth, max)| vb_yaml::YamlError::NestingTooDeep { depth, max })
            .boxed(),
        9 => ((0u32..10_000u32), (0u32..10_000u32))
            .prop_map(|(count, max)| vb_yaml::YamlError::NodeLimitExceeded { count, max })
            .boxed(),
        10 => ((0usize..10_000usize), (0usize..10_000usize))
            .prop_map(|(len, max)| vb_yaml::YamlError::ScalarTooLong { len, max })
            .boxed(),
        11 => ((0usize..10_000usize), (0usize..10_000usize))
            .prop_map(|(len, max)| vb_yaml::YamlError::SequenceTooLong { len, max })
            .boxed(),
        12 => ((0usize..10_000usize), (0usize..10_000usize))
            .prop_map(|(count, max)| vb_yaml::YamlError::MappingTooLarge { count, max })
            .boxed(),
        13 => (any::<String>())
            .prop_map(|s| vb_yaml::YamlError::UnknownField {
                field: s.into_boxed_str(),
            })
            .boxed(),
        14 => Just(vb_yaml::YamlError::EmptySource).boxed(),
        15 => {
            let field_strat = proptest::string::string_regex(r"[a-zA-Z_]{1,20}").unwrap();
            field_strat
                .prop_map(|field| {
                    let leaked: &'static str = Box::leak(field.into_boxed_str());
                    vb_yaml::YamlError::MissingField { field: leaked }
                })
                .boxed()
        }
        16 => {
            let f_strat = proptest::string::string_regex(r"[a-zA-Z_]{1,10}").unwrap();
            f_strat
                .prop_flat_map(move |field| {
                    let e2 = proptest::string::string_regex(r"[a-zA-Z_]{1,10}").unwrap();
                    e2.prop_map(move |expected| {
                        let f_leaked: &'static str = Box::leak(field.clone().into_boxed_str());
                        let e_leaked: &'static str = Box::leak(expected.into_boxed_str());
                        vb_yaml::YamlError::FieldShape {
                            field: f_leaked,
                            expected: e_leaked,
                        }
                    })
                })
                .boxed()
        }
        17 => ((0usize..100usize), any::<String>())
            .prop_map(|(line, reason)| vb_yaml::YamlError::ParseError {
                line,
                reason: reason.into_boxed_str(),
            })
            .boxed(),
        18 => {
            let feat_strat = proptest::string::string_regex(r"[a-zA-Z_]{1,20}").unwrap();
            feat_strat
                .prop_map(|feature| {
                    let leaked: &'static str = Box::leak(feature.into_boxed_str());
                    vb_yaml::YamlError::UnsupportedFeature { feature: leaked }
                })
                .boxed()
        }
        19 => {
            let trig_strat = proptest::string::string_regex(r"[a-zA-Z_]{1,20}").unwrap();
            trig_strat
                .prop_map(|trigger| {
                    let leaked: &'static str = Box::leak(trigger.into_boxed_str());
                    vb_yaml::YamlError::UnsupportedTrigger { trigger: leaked }
                })
                .boxed()
        }
        20 => {
            let p_strat = proptest::string::string_regex(r"[a-zA-Z_]{1,10}").unwrap();
            p_strat
                .prop_flat_map(move |primitive| {
                    let c2 = proptest::string::string_regex(r"[a-zA-Z_]{1,10}").unwrap();
                    c2.prop_map(move |canonical| {
                        let p_leaked: &'static str = Box::leak(primitive.clone().into_boxed_str());
                        let c_leaked: &'static str = Box::leak(canonical.into_boxed_str());
                        vb_yaml::YamlError::LegacyPrimitive {
                            primitive: p_leaked,
                            canonical: c_leaked,
                        }
                    })
                })
                .boxed()
        }
        _ => unreachable!(),
    })
}
