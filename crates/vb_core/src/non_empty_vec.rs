#![forbid(unsafe_code)]

//! Non-empty vector guaranteed to contain at least one element.
//!
//! Used for error accumulation where at least one error must exist.

use core::fmt;

/// A non-empty vector guaranteed to contain at least one element.
///
/// # Invariants
/// - `head` is always a valid `T`
/// - `len() >= 1`
/// - `is_empty()` always returns `false`
/// - `first()` never panics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyVec<T> {
    head: T,
    tail: Vec<T>,
}

impl<T> NonEmptyVec<T> {
    /// Creates a non-empty vector from a single element.
    #[must_use]
    pub fn new(head: T) -> Self {
        Self {
            head,
            tail: Vec::new(),
        }
    }

    /// Creates a non-empty vector with a head element and a tail vec.
    #[must_use]
    pub fn with_tail(head: T, tail: Vec<T>) -> Self {
        Self { head, tail }
    }

    /// Creates a non-empty vector from a `Vec`. Returns `None` if the vec is empty.
    #[must_use]
    pub fn from_vec(mut vec: Vec<T>) -> Option<Self> {
        if vec.is_empty() {
            return None;
        }
        // Take the first element as head; the rest become the tail.
        // Order is preserved: head was vec[0], tail is vec[1..].
        let head = vec.remove(0);
        Some(Self { head, tail: vec })
    }

    /// Returns a reference to the first element.
    #[must_use]
    pub fn first(&self) -> &T {
        &self.head
    }

    /// Returns a reference to the last element.
    #[must_use]
    pub fn last(&self) -> &T {
        self.tail.last().unwrap_or(&self.head)
    }

    /// Returns the number of elements (always >= 1).
    #[must_use]
    pub fn len(&self) -> usize {
        1_usize.saturating_add(self.tail.len())
    }

    /// Always returns `false`. A `NonEmptyVec` is never empty by construction.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Appends an element to the end.
    pub fn push(&mut self, value: T) {
        self.tail.push(value);
    }

    /// Extends the collection from an iterator.
    pub fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
        self.tail.extend(iter);
    }
}

impl<T> NonEmptyVec<T>
where
    T: Clone,
{
    /// Consumes the non-empty vec and returns a plain `Vec<T>`.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        let mut vec = Vec::with_capacity(self.len());
        vec.push(self.head);
        vec.extend(self.tail);
        vec
    }
}

impl<T> From<NonEmptyVec<T>> for Vec<T>
where
    T: Clone,
{
    fn from(nev: NonEmptyVec<T>) -> Self {
        nev.into_vec()
    }
}

impl<T: fmt::Display> fmt::Display for NonEmptyVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.head)?;
        for item in &self.tail {
            write!(f, ", {item}")?;
        }
        Ok(())
    }
}

/// Consuming iterator over `NonEmptyVec`.
pub struct IntoIter<T> {
    head: Option<T>,
    tail: std::vec::IntoIter<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.head.take().or_else(|| self.tail.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let head_count = if self.head.is_some() { 1 } else { 0 };
        let (lo, hi) = self.tail.size_hint();
        (
            lo.saturating_add(head_count),
            hi.and_then(|h| h.checked_add(head_count)),
        )
    }
}

impl<T> IntoIterator for NonEmptyVec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            head: Some(self.head),
            tail: self.tail.into_iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_len_one() {
        let nev = NonEmptyVec::new(42);
        assert_eq!(nev.len(), 1);
        assert!(!nev.is_empty());
    }

    #[test]
    fn new_first_returns_head() {
        let nev = NonEmptyVec::new("hello");
        assert_eq!(nev.first(), &"hello");
        assert_eq!(nev.last(), &"hello");
    }

    #[test]
    fn with_tail_correct_len() {
        let nev = NonEmptyVec::with_tail(1, vec![2, 3, 4]);
        assert_eq!(nev.len(), 4);
        assert!(!nev.is_empty());
        assert_eq!(nev.first(), &1);
        assert_eq!(nev.last(), &4);
    }

    #[test]
    fn from_vec_returns_none_for_empty() {
        let empty: Vec<i32> = Vec::new();
        assert!(NonEmptyVec::from_vec(empty).is_none());
    }

    #[test]
    fn from_vec_returns_some_for_non_empty() {
        let nev = NonEmptyVec::from_vec(vec![10, 20, 30]);
        assert!(nev.is_some());
        let nev = nev.expect("should be some");
        assert_eq!(nev.first(), &10);
        assert_eq!(nev.len(), 3);
        assert_eq!(nev.last(), &30);
    }

    #[test]
    fn push_increases_len() {
        let mut nev = NonEmptyVec::new(1);
        assert_eq!(nev.len(), 1);
        nev.push(2);
        assert_eq!(nev.len(), 2);
        assert_eq!(nev.last(), &2);
    }

    #[test]
    fn into_vec_round_trip() {
        let original = vec![1, 2, 3, 4, 5];
        let nev = NonEmptyVec::from_vec(original.clone()).expect("should be some");
        let round_tripped: Vec<i32> = nev.into_vec();
        assert_eq!(round_tripped, original);
    }

    #[test]
    fn into_iter_exhaustive() {
        let nev = NonEmptyVec::with_tail(10, vec![20, 30]);
        let collected: Vec<i32> = nev.into_iter().collect();
        assert_eq!(collected, vec![10, 20, 30]);
    }

    #[test]
    fn from_trait_works() {
        let nev = NonEmptyVec::with_tail(7, vec![8, 9]);
        let v: Vec<i32> = nev.into();
        assert_eq!(v, vec![7, 8, 9]);
    }

    #[test]
    fn extend_appends_all_elements_preserving_order() {
        // B43: extend appends all elements preserving order
        let mut nev = NonEmptyVec::new(1);
        nev.extend(vec![2, 3, 4]);
        assert_eq!(nev.len(), 4);
        assert_eq!(nev.first(), &1);
        assert_eq!(nev.last(), &4);
        let collected: Vec<i32> = nev.into_vec();
        assert_eq!(collected, vec![1, 2, 3, 4]);
    }

    #[test]
    fn display_renders_comma_separated_elements() {
        // B47: Display renders elements comma-separated
        let nev = NonEmptyVec::with_tail(1, vec![2, 3]);
        let rendered = format!("{nev}");
        // Format is "head, tail_item1, tail_item2, ..."
        assert!(rendered.starts_with('1'));
        assert!(rendered.contains("2"));
        assert!(rendered.contains("3"));
        assert!(rendered.contains(','));
    }

    #[test]
    fn into_vec_on_single_element_does_not_double_allocate_head() {
        // B48: into_vec on single-element returns exactly one element
        let nev = NonEmptyVec::new(99_u32);
        let v: Vec<u32> = nev.into_vec();
        assert_eq!(v.len(), 1);
        assert_eq!(v, vec![99_u32]);
    }

    #[test]
    fn with_tail_empty_tail_preserves_head() {
        // B38: with_tail with empty tail preserves head as only element
        let nev = NonEmptyVec::with_tail("only", vec![]);
        assert_eq!(nev.len(), 1);
        assert_eq!(nev.first(), &"only");
        assert_eq!(nev.last(), &"only");
    }

    #[test]
    fn into_vec_large_round_trip_preserves_all() {
        // Large vec round-trip (regression for PO-K02 timeout gap)
        let original: Vec<i32> = (0..10_000).collect();
        let nev = NonEmptyVec::from_vec(original.clone()).expect("non-empty");
        let recovered: Vec<i32> = nev.into_vec();
        assert_eq!(recovered, original);
    }
}
