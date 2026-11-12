use crate::ValidationResult;
use vb_core::ids::SlotIdx;
use vb_core::workflow::{AccessorProgram, PathSegment};

pub(super) struct SelectedAccessors(std::array::IntoIter<AccessorProgram, 3>);

impl std::ops::Deref for SelectedAccessors {
    type Target = [AccessorProgram];
    fn deref(&self) -> &[AccessorProgram] {
        self.0.as_slice()
    }
}

pub(super) struct Gate8Inputs<A = Box<[AccessorProgram]>> {
    pub(super) accessors: A,
    pub(super) slot_count: u16,
    pub(super) symbols_count: u32,
}

impl<A: std::ops::Deref<Target = [AccessorProgram]>> Gate8Inputs<A> {
    pub(super) fn validate(&self) -> ValidationResult<()> {
        crate::gate_08_accessor::validate_accessor_programs(
            self.accessors.deref(),
            self.slot_count,
            self.symbols_count,
        )
    }
}

pub(super) fn arbitrary_inputs() -> Gate8Inputs<SelectedAccessors> {
    let count: u8 = kani::any();
    kani::assume(count <= 3);
    let all: [AccessorProgram; 3] = kani::any();
    let mut iter = all.into_iter();
    let drop_count = 3u8.saturating_sub(count);
    iter.by_ref().take(usize::from(drop_count)).for_each(drop);
    Gate8Inputs {
        accessors: SelectedAccessors(iter),
        slot_count: kani::any(),
        symbols_count: kani::any(),
    }
}

pub(super) fn valid_inputs() -> Gate8Inputs<SelectedAccessors> {
    let input = arbitrary_inputs();
    kani::assume(input.slot_count > 0 && input.symbols_count > 0);
    kani::assume(
        input
            .accessors
            .iter()
            .all(|a| a.root.get() < input.slot_count),
    );
    kani::assume(input.accessors.iter().all(|a| {
        a.path.iter().all(|s| match s {
            PathSegment::Field(symbol) => symbol.get() < input.symbols_count,
            PathSegment::Index(index) => *index != u32::MAX,
            _ => false,
        })
    }));
    input
}

pub(super) fn one_accessor(
    accessor: AccessorProgram,
    slot_count: u16,
    symbols_count: u32,
) -> Gate8Inputs {
    Gate8Inputs {
        accessors: Box::new([accessor]),
        slot_count,
        symbols_count,
    }
}

pub(super) fn root_oob_inputs() -> Gate8Inputs {
    let accessor: AccessorProgram = kani::any();
    let slot_count: u16 = kani::any();
    kani::assume(accessor.root.get() >= slot_count);
    one_accessor(accessor, slot_count, kani::any())
}

pub(super) fn symbol_oob_inputs() -> Gate8Inputs {
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    let root: SlotIdx = kani::any();
    let symbol: vb_core::ids::SymbolId = kani::any();
    let index: u32 = kani::any();
    kani::assume(slot_count > 0 && root.get() < slot_count);
    kani::assume(symbol.get() >= symbols_count && index != u32::MAX);
    let accessor = AccessorProgram {
        root,
        path: Box::new([PathSegment::Index(index), PathSegment::Field(symbol)]),
    };
    one_accessor(accessor, slot_count, symbols_count)
}

pub(super) fn sentinel_inputs() -> Gate8Inputs {
    let accessor: AccessorProgram = kani::any();
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    kani::assume(slot_count > 0 && accessor.root.get() < slot_count);
    kani::assume(accessor.path.iter().all(|s| match s {
        PathSegment::Field(symbol) => symbol.get() < symbols_count,
        PathSegment::Index(_) => true,
        _ => false,
    }));
    kani::assume(
        accessor
            .path
            .iter()
            .any(|s| matches!(s, PathSegment::Index(u32::MAX))),
    );
    one_accessor(accessor, slot_count, symbols_count)
}
