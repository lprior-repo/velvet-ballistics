# Boundary map and source anchors

| Boundary | Source | Ownership / observation |
| --- | --- | --- |
| Author declarations | crates/vb_yaml/src/yaml_ast/parse_fields.rs; types/fields.rs | Parser retains InputField.key/value as AuthorValue |
| Schema meaning | crates/vb_compile/src/mod_compile_lowering/input_schema.rs | Pure compiler shape parser; no default resolver |
| Slot reservation/binding | crates/vb_compile/src/mod_compile_lowering/part_01.rs; finish_inputs.rs | CI part_01 owner; no edits authorized during analysis |
| Primitive consumers | part_02.rs; part_03.rs; part_04.rs; part_14.rs; part_05_utility.rs in same lowering dir | Numeric-only slot_from_text in non-Finish paths; SetConst-only Set |
| Numeric artifact | crates/vb_core/src/workflow/mod.rs | CompiledInputSlot has slot/kind; coordinated shared API/wire owner |
| CLI transport mapping | crates/velvet_ballistics/src/run_compiled.rs | Exact positional postcard decode; omissions accepted |
| Runtime seeding | crates/vb_runtime/src/shard/helpers.rs; lifecycle/chunk_001.rs | Seeds raw handles; fresh store allocated separately |
| Owned backing | crates/vb_core/src/value_store.rs; value.rs | Run-owned arenas; ConstValue excludes list/object/blob payloads |
| Atomic retained admission | Separate candidate9f0557 and vb-netau integration | Not present in this draft base; external integration dependency |

Pure core resolves validated numeric bindings and bounded owned values. YAML/postcard decoding, storage, process lifecycle and durable recovery are imperative boundaries. No runtime HTTP/JSON/YAML or source-name resolution is proposed.
