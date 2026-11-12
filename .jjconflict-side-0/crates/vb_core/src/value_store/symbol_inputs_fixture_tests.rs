use super::*;

pub(super) fn symbols(texts: &[&str]) -> Vec<Box<str>> {
    texts.iter().map(|text| Box::<str>::from(*text)).collect()
}

pub(super) fn checkpoint(texts: &[&str]) -> ValueStoreCheckpointV1 {
    ValueStoreCheckpointV1 {
        symbols: symbols(texts),
        lists: vec![],
        list_taints: vec![],
        objects: vec![],
        blobs: vec![],
        max_arena_entries: 0,
    }
}

pub(super) fn symbol_roots(ids: &[u32]) -> Box<[SlotValue]> {
    ids.iter()
        .map(|id| SlotValue::Symbol(SymbolId::new(*id)))
        .collect()
}

fn input_fields() -> Box<[ObjectField]> {
    Box::from([
        ObjectField::with_taint(
            SymbolId::new(1),
            SlotValue::Symbol(SymbolId::new(0)),
            Taint::Secret,
        ),
        ObjectField::with_taint(SymbolId::new(1), SlotValue::I64(9), Taint::Clean),
        ObjectField::with_taint(
            SymbolId::new(0),
            SlotValue::Symbol(SymbolId::new(3)),
            Taint::DerivedFromSecret,
        ),
    ])
}

fn expected_fields() -> Box<[ObjectField]> {
    Box::from([
        ObjectField::with_taint(
            SymbolId::new(0),
            SlotValue::Symbol(SymbolId::new(2)),
            Taint::Secret,
        ),
        ObjectField::with_taint(SymbolId::new(0), SlotValue::I64(9), Taint::Clean),
        ObjectField::with_taint(
            SymbolId::new(2),
            SlotValue::Symbol(SymbolId::new(3)),
            Taint::DerivedFromSecret,
        ),
    ])
}

pub(super) fn golden_input() -> ValueStoreCheckpointV1 {
    let mut input = checkpoint(&["a", "k", "a", "z"]);
    input.lists = vec![Box::from([
        SlotValue::Symbol(SymbolId::new(1)),
        SlotValue::Symbol(SymbolId::new(2)),
        SlotValue::Object(ObjectId::new(0)),
        SlotValue::Blob(BlobId::new(0)),
    ])];
    input.list_taints = vec![Box::from([
        Taint::Clean,
        Taint::Secret,
        Taint::DerivedFromSecret,
        Taint::Clean,
    ])];
    input.objects = vec![input_fields()];
    input.blobs = vec![bytes::Bytes::from_static(b"blob")];
    input.max_arena_entries = 7;
    input
}

pub(super) fn golden_expected() -> ValueStoreCheckpointV1 {
    let mut expected = checkpoint(&["k", "p", "a", "z"]);
    expected.lists = vec![Box::from([
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::Symbol(SymbolId::new(2)),
        SlotValue::Object(ObjectId::new(0)),
        SlotValue::Blob(BlobId::new(0)),
    ])];
    expected.list_taints = vec![Box::from([
        Taint::Clean,
        Taint::Secret,
        Taint::DerivedFromSecret,
        Taint::Clean,
    ])];
    expected.objects = vec![expected_fields()];
    expected.blobs = vec![bytes::Bytes::from_static(b"blob")];
    expected.max_arena_entries = 7;
    expected
}

pub(super) fn golden_roots() -> Box<[SlotValue]> {
    Box::from([
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::Symbol(SymbolId::new(1)),
        SlotValue::Symbol(SymbolId::new(2)),
        SlotValue::Symbol(SymbolId::new(3)),
        SlotValue::List(ListId::new(0)),
        SlotValue::Object(ObjectId::new(0)),
        SlotValue::Blob(BlobId::new(0)),
    ])
}

pub(super) fn golden_expected_roots() -> Box<[SlotValue]> {
    Box::from([
        SlotValue::Symbol(SymbolId::new(2)),
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::Symbol(SymbolId::new(2)),
        SlotValue::Symbol(SymbolId::new(3)),
        SlotValue::List(ListId::new(0)),
        SlotValue::Object(ObjectId::new(0)),
        SlotValue::Blob(BlobId::new(0)),
    ])
}

pub(super) fn state_input() -> ValueStoreCheckpointV1 {
    let mut input = checkpoint(&["a", "b"]);
    input.lists = vec![symbol_roots(&[1])];
    input.list_taints = vec![Box::from([Taint::Secret])];
    input.objects = vec![Box::from([ObjectField::with_taint(
        SymbolId::new(0),
        SlotValue::I64(42),
        Taint::DerivedFromSecret,
    )])];
    input
}

pub(super) fn state_expected() -> ValueStoreCheckpointV1 {
    let mut expected = checkpoint(&["compiler", "a", "b"]);
    expected.lists = vec![symbol_roots(&[2])];
    expected.list_taints = vec![Box::from([Taint::Secret])];
    expected.objects = vec![Box::from([ObjectField::with_taint(
        SymbolId::new(1),
        SlotValue::I64(42),
        Taint::DerivedFromSecret,
    )])];
    expected
}

pub(super) fn invalid_taints() -> ValueStoreCheckpointV1 {
    let mut input = checkpoint(&["a"]);
    input.lists = vec![Box::from([SlotValue::I64(1)])];
    input.list_taints = vec![Box::from([])];
    input
}
