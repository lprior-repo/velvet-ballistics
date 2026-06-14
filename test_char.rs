#[cfg(kani)]
mod tests {
    #[kani::proof]
    #[kani::unwind(10)]
    fn check_char() {
        let len: usize = kani::any();
        kani::assume(len <= 5);
        let mut s = String::new();
        for _ in 0..len {
            s.push(kani::any::<char>());
        }
    }
}
