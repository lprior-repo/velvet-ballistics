#![forbid(unsafe_code)]

#[derive(Debug, Clone)]
pub(super) struct KaniReplayMap<K, V, const N: usize> {
    entries: [Option<(K, V)>; N],
}

impl<K: Copy + Eq, V: Copy, const N: usize> KaniReplayMap<K, V, N> {
    pub(super) fn new() -> Self {
        Self { entries: [None; N] }
    }

    pub(super) fn get(&self, key: &K) -> Option<&V> {
        for entry in &self.entries {
            if let Some((candidate, value)) = entry.as_ref()
                && candidate == key
            {
                return Some(value);
            }
        }
        None
    }

    pub(super) fn insert(&mut self, key: K, value: V) -> Option<V> {
        for entry in &mut self.entries {
            if let Some((candidate, stored)) = entry.as_mut()
                && *candidate == key
            {
                let previous = *stored;
                *stored = value;
                return Some(previous);
            }
        }
        for entry in &mut self.entries {
            if entry.is_none() {
                *entry = Some((key, value));
                return None;
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub(super) struct KaniReplaySet<K, const N: usize> {
    entries: [Option<K>; N],
}

impl<K: Copy + Eq, const N: usize> KaniReplaySet<K, N> {
    pub(super) fn new() -> Self {
        Self { entries: [None; N] }
    }

    pub(super) fn contains(&self, key: &K) -> bool {
        for entry in &self.entries {
            if let Some(candidate) = entry.as_ref()
                && candidate == key
            {
                return true;
            }
        }
        false
    }

    pub(super) fn insert(&mut self, key: K) -> bool {
        if self.contains(&key) {
            return false;
        }
        for entry in &mut self.entries {
            if entry.is_none() {
                *entry = Some(key);
                return true;
            }
        }
        false
    }
}
