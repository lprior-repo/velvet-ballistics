use super::gate_08_inputs::{Gate8Inputs, one_accessor};
use vb_core::ids::{SlotIdx, SymbolId};
use vb_core::workflow::{AccessorProgram, PathSegment};

struct MixedData {
    slots: u16,
    symbols: u32,
    roots: [SlotIdx; 3],
    fields: [SymbolId; 2],
    indices: [u32; 6],
}

fn mixed_data() -> MixedData {
    let data = MixedData {
        slots: kani::any(),
        symbols: kani::any(),
        roots: kani::any(),
        fields: kani::any(),
        indices: kani::any(),
    };
    kani::assume(data.slots >= 3 && data.symbols >= 2);
    kani::assume(data.roots.iter().all(|r| r.get() < data.slots));
    kani::assume(data.fields.iter().all(|s| s.get() < data.symbols));
    kani::assume(data.indices.iter().all(|i| *i != u32::MAX));
    data
}

fn program(root: SlotIdx, path: Box<[PathSegment]>) -> AccessorProgram {
    AccessorProgram { root, path }
}

fn mixed_front(data: &MixedData) -> [AccessorProgram; 3] {
    let [root, _, _] = data.roots;
    let [field, _] = data.fields;
    let [index, _, _, _, _, _] = data.indices;
    [
        program(root, Box::new([])),
        program(root, Box::new([PathSegment::Field(field)])),
        program(root, Box::new([PathSegment::Index(index)])),
    ]
}

fn mixed_back(data: &MixedData) -> [AccessorProgram; 3] {
    let [root0, root1, root2] = data.roots;
    let [field0, field1] = data.fields;
    let [index0, index1, index2, index3, index4, index100] = data.indices;
    let chain = Box::new([
        PathSegment::Field(field0),
        PathSegment::Index(index0),
        PathSegment::Field(field1),
    ]);
    let depth_five = [index0, index1, index2, index3, index4]
        .into_iter()
        .map(PathSegment::Index)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    [
        program(root0, chain),
        program(root1, Box::new([PathSegment::Index(index100)])),
        program(root2, depth_five),
    ]
}

pub(super) fn mixed_inputs() -> Gate8Inputs {
    let data = mixed_data();
    Gate8Inputs {
        accessors: mixed_front(&data)
            .into_iter()
            .chain(mixed_back(&data))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        slot_count: data.slots,
        symbols_count: data.symbols,
    }
}

pub(super) fn expression_inputs() -> Gate8Inputs {
    let slots: u16 = kani::any();
    let symbols: u32 = kani::any();
    let root: SlotIdx = kani::any();
    let field: SymbolId = kani::any();
    kani::assume(slots >= 2 && symbols >= 2);
    kani::assume(root.get() < slots && field.get() < symbols);
    Gate8Inputs {
        accessors: Box::new([
            program(root, Box::new([PathSegment::Field(field)])),
            program(root, Box::new([PathSegment::Index(0)])),
        ]),
        slot_count: slots,
        symbols_count: symbols,
    }
}

pub(super) fn depth_inputs(depth: usize) -> Gate8Inputs {
    let slots: u16 = kani::any();
    let root: SlotIdx = kani::any();
    let index: u32 = kani::any();
    kani::assume(slots > 0 && root.get() < slots && index != u32::MAX);
    one_accessor(
        program(
            root,
            vec![PathSegment::Index(index); depth].into_boxed_slice(),
        ),
        slots,
        kani::any(),
    )
}
