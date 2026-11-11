// Exhaustive bounded lengths; each element remains independently symbolic.
fn bounded_symbolic_items<T: kani::Arbitrary>(count: u8) -> Box<[T]> {
    match count {
        0 => Box::new([]),
        1 => symbolic_items_1(),
        2 => symbolic_items_2(),
        3 => symbolic_items_3(),
        4 => symbolic_items_4(),
        5 => symbolic_items_5(),
        6 => symbolic_items_6(),
        7 => symbolic_items_7(),
        8 => symbolic_items_8(),
        9 => symbolic_items_9(),
        10 => symbolic_items_10(),
        11 => symbolic_items_11(),
        12 => symbolic_items_12(),
        13 => symbolic_items_13(),
        14 => symbolic_items_14(),
        15 => symbolic_items_15(),
        16 => symbolic_items_16(),
        _ => {
            kani::assert(false, "symbolic collection length exceeds declared bound");
            std::process::abort()
        }
    }
}

fn symbolic_items_1<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([kani::any()])
}

fn symbolic_items_2<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([kani::any(), kani::any()])
}

fn symbolic_items_3<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([kani::any(), kani::any(), kani::any()])
}

fn symbolic_items_4<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([kani::any(), kani::any(), kani::any(), kani::any()])
}

fn symbolic_items_5<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_6<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_7<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_8<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_9<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_10<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_11<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_12<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_13<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_14<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_15<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

fn symbolic_items_16<T: kani::Arbitrary>() -> Box<[T]> {
    Box::new([
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    ])
}

#[kani::proof]
#[kani::unwind(5)]
fn kani_symbolic_collection_full_lengths() {
    let count: u8 = kani::any();
    kani::assume(count <= 16);
    let values: Box<[u16]> = bounded_symbolic_items(count);
    kani::assert(
        values.len() == usize::from(count),
        "every declared length is retained",
    );
    cover_declared_lengths(count);
    cover_collection_endpoints(&values);
    std::mem::forget(values);
}

fn cover_collection_endpoints(values: &[u16]) {
    kani::cover(values.first() == Some(&0), "first zero");
    kani::cover(values.first() == Some(&u16::MAX), "first max");
    kani::cover(values.last() == Some(&0), "last zero");
    kani::cover(values.last() == Some(&u16::MAX), "last max");
}

fn cover_declared_lengths(count: u8) {
    kani::cover(count == 0, "length 0");
    kani::cover(count == 1, "length 1");
    kani::cover(count == 2, "length 2");
    kani::cover(count == 3, "length 3");
    kani::cover(count == 4, "length 4");
    kani::cover(count == 5, "length 5");
    kani::cover(count == 6, "length 6");
    kani::cover(count == 7, "length 7");
    kani::cover(count == 8, "length 8");
    kani::cover(count == 9, "length 9");
    kani::cover(count == 10, "length 10");
    kani::cover(count == 11, "length 11");
    kani::cover(count == 12, "length 12");
    kani::cover(count == 13, "length 13");
    kani::cover(count == 14, "length 14");
    kani::cover(count == 15, "length 15");
    kani::cover(count == 16, "length 16");
}
