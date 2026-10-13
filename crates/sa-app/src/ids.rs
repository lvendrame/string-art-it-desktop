//! Sequential, human-readable ids (`pin-42`). Port of web `idCounter.ts`.
//!
//! Counters are per-thread: all document mutation happens on the UI thread, and
//! per-thread state keeps parallel tests independent.

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static COUNTERS: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

pub fn next_id(prefix: &str) -> String {
    COUNTERS.with_borrow_mut(|c| {
        let n = c.entry(prefix.to_owned()).or_insert(0);
        *n += 1;
        format!("{prefix}-{n}")
    })
}

/// Bumps the counter for `id`'s prefix past its number so ids loaded from a file
/// are never re-minted.
pub fn seed_counter_from(id: &str) {
    let Some((prefix, num)) = id.rsplit_once('-') else {
        return;
    };
    if prefix.is_empty() || num.is_empty() || !num.bytes().all(|b| b.is_ascii_digit()) {
        return;
    }
    let Ok(num) = num.parse::<u64>() else { return };
    COUNTERS.with_borrow_mut(|c| {
        let n = c.entry(prefix.to_owned()).or_insert(0);
        if num > *n {
            *n = num;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_and_seeded() {
        let a = next_id("idtest");
        let b = next_id("idtest");
        assert_ne!(a, b);
        seed_counter_from("idtest-500");
        assert_eq!(next_id("idtest"), "idtest-501");
        seed_counter_from("idtest-3"); // lower: ignored
        assert_eq!(next_id("idtest"), "idtest-502");
        seed_counter_from("pin~mirror-x"); // non-numeric: ignored
        seed_counter_from("pinlayer-dup-7");
        assert_eq!(next_id("pinlayer-dup"), "pinlayer-dup-8");
    }
}
