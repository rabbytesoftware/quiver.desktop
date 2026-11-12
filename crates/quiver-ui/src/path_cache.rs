//! A cache of tessellated paths. GPUI re-runs every canvas paint closure on every frame, and building a
//! `Path` (parsing the SVG data, then lyon tessellation) is far more expensive than painting one: a screen with
//! a few dozen icons spent its frame budget there. A `Path` holds window coordinates, so the key carries the
//! origin it was built at: a static icon hits the cache every frame, a scrolling one rebuilds as it did before.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use gpui::{Path, Pixels};

/// Entries kept before the cache is dropped and refilled (a scroll walks the origin through many values).
const CAPACITY: usize = 4096;

type Paths = Rc<Vec<Path<Pixels>>>;

thread_local! {
    static CACHE: RefCell<HashMap<u64, Paths>> = RefCell::new(HashMap::new());
}

fn hash_of(key: &impl Hash) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// The paths stored under `key`, built with `build` on a miss. A failed build is not stored.
pub(crate) fn cached<E>(
    key: impl Hash,
    build: impl FnOnce() -> Result<Vec<Path<Pixels>>, E>,
) -> Result<Paths, E> {
    let key = hash_of(&key);
    if let Some(hit) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return Ok(hit);
    }
    let built = Rc::new(build()?);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= CAPACITY {
            c.clear();
        }
        c.insert(key, built.clone());
    });
    Ok(built)
}
