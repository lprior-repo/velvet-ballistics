use super::*;
use std::collections::HashSet;

#[test]
fn temp_keyspace_cleanup() {
    let temp = match TempKeyspace::open() {
        Ok(v) => v,
        Err(e) => panic!("TempKeyspace::open() should succeed, got Err({e:?})"),
    };
    let path = temp.path().to_path_buf();
    drop(temp);
    assert!(!path.exists());
}

#[test]
fn temp_keyspace_uniqueness() {
    let mut paths = HashSet::new();
    for _ in 0..100 {
        let temp = match TempKeyspace::open() {
            Ok(v) => v,
            Err(e) => panic!("TempKeyspace::open() should succeed, got Err({e:?})"),
        };
        let path = temp.path().to_path_buf();
        assert!(paths.insert(path), "temp keyspaces must have unique paths");
    }
}

#[test]
fn temp_keyspace_concurrent_uniqueness() {
    use std::thread;

    let handles: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                let mut paths = HashSet::new();
                for _ in 0..10 {
                    let temp = match TempKeyspace::open() {
                        Ok(v) => v,
                        Err(e) => {
                            panic!("TempKeyspace::open() should succeed, got Err({e:?})")
                        }
                    };
                    let path = temp.path().to_path_buf();
                    assert!(paths.insert(path), "temp keyspaces must have unique paths");
                }
                paths
            })
        })
        .collect();

    let mut all_paths = HashSet::new();
    for h in handles {
        let paths = match h.join() {
            Ok(v) => v,
            Err(_) => panic!("worker thread should not panic"),
        };
        for p in paths {
            assert!(
                all_paths.insert(p),
                "concurrent temp keyspaces must have unique paths"
            );
        }
    }
}
