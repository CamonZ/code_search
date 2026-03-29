//! Deduplication utilities for reducing code duplication across commands.
//!
//! This module provides reusable patterns for deduplicating collections:
//! - HashSet retain pattern (deduplicate_retain) - for in-place deduplication after sorting
//! - Combined sort and deduplicate operation (sort_and_deduplicate)

use std::collections::HashSet;
use std::hash::Hash;

/// Strategy A: HashSet retain pattern - deduplicate in-place
///
/// Use this when you have a collection that's already been sorted, and you want to remove
/// duplicate entries while preserving the sort order.
///
/// # Arguments
/// * `items` - Mutable vector of items to deduplicate
/// * `key_fn` - Function that extracts the deduplication key from each item
///
/// # Example
/// ```ignore
/// let mut calls = vec![...];
/// calls.sort_by_key(|c| c.line);
/// deduplicate_retain(&mut calls, |c| {
///     (c.callee.module.clone(), c.callee.name.clone(), c.callee.arity)
/// });
/// ```
pub fn deduplicate_retain<T, F, K>(items: &mut Vec<T>, key_fn: F)
where
    F: Fn(&T) -> K,
    K: Eq + Hash,
{
    let mut seen: HashSet<K> = HashSet::new();
    items.retain(|item| seen.insert(key_fn(item)));
}

/// Combined sort and deduplicate operation.
///
/// Sorts a collection using a comparator, then deduplicates using a different key.
/// Preserves the first occurrence of each duplicate.
///
/// Use this when you need to:
/// 1. Sort items by one criteria (e.g., line number)
/// 2. Remove duplicates based on different criteria (e.g., callee name)
///
/// # Arguments
/// * `items` - Mutable vector of items to sort and deduplicate
/// * `sort_cmp` - Comparator function that returns the ordering between two items
/// * `dedup_key_fn` - Function that extracts the deduplication key
///
/// # Example
/// ```ignore
/// let mut calls = vec![...];
/// sort_and_deduplicate(
///     &mut calls,
///     |a, b| a.line.cmp(&b.line),  // Sort by line number - no allocation
///     |c| (c.callee.module.clone(), c.callee.name.clone(), c.callee.arity)  // Dedup by callee
/// );
/// ```
pub fn sort_and_deduplicate<T, SC, DK, D>(
    items: &mut Vec<T>,
    sort_cmp: SC,
    dedup_key: DK,
)
where
    SC: FnMut(&T, &T) -> std::cmp::Ordering,
    DK: Fn(&T) -> D,
    D: Eq + Hash,
{
    items.sort_by(sort_cmp);
    deduplicate_retain(items, dedup_key);
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // deduplicate_retain tests
    // =========================================================================

    #[test]
    fn deduplicate_retain_removes_duplicates_preserving_order() {
        let mut items = vec![1, 2, 3, 2, 1, 4, 3, 5];
        deduplicate_retain(&mut items, |x| *x);
        assert_eq!(items, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn deduplicate_retain_keeps_first_occurrence() {
        // Pairs of (key, value) - dedup by key, verify first value survives
        let mut items = vec![(1, "first"), (2, "a"), (1, "second"), (2, "b")];
        deduplicate_retain(&mut items, |x| x.0);
        assert_eq!(items, vec![(1, "first"), (2, "a")]);
    }

    #[test]
    fn deduplicate_retain_all_duplicates() {
        let mut items = vec![7, 7, 7, 7];
        deduplicate_retain(&mut items, |x| *x);
        assert_eq!(items, vec![7]);
    }

    #[test]
    fn deduplicate_retain_no_duplicates() {
        let mut items = vec![1, 2, 3, 4];
        deduplicate_retain(&mut items, |x| *x);
        assert_eq!(items, vec![1, 2, 3, 4]);
    }

    #[test]
    fn deduplicate_retain_empty_input() {
        let mut items: Vec<i32> = vec![];
        deduplicate_retain(&mut items, |x| *x);
        assert!(items.is_empty());
    }

    #[test]
    fn deduplicate_retain_single_element() {
        let mut items = vec![42];
        deduplicate_retain(&mut items, |x| *x);
        assert_eq!(items, vec![42]);
    }

    #[test]
    fn deduplicate_retain_with_custom_key() {
        // Dedup by string length, keeping first occurrence of each length
        let mut items = vec!["hi", "hey", "go", "bye", "ok", "hello"];
        deduplicate_retain(&mut items, |s| s.len());
        assert_eq!(items, vec!["hi", "hey", "hello"]);
    }

    // =========================================================================
    // sort_and_deduplicate tests
    // =========================================================================

    #[test]
    fn sort_and_deduplicate_sorts_and_removes_duplicates() {
        let mut items = vec![3, 1, 2, 3, 1, 4];
        sort_and_deduplicate(&mut items, |a, b| a.cmp(b), |x| *x);
        assert_eq!(items, vec![1, 2, 3, 4]);
    }

    #[test]
    fn sort_and_deduplicate_verifies_ordering() {
        let mut items = vec![5, 3, 1, 4, 2];
        sort_and_deduplicate(&mut items, |a, b| a.cmp(b), |x| *x);
        assert_eq!(items, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn sort_and_deduplicate_verifies_dedup_after_sort() {
        // Sort descending, dedup by value
        let mut items = vec![1, 3, 2, 3, 1, 2];
        sort_and_deduplicate(&mut items, |a, b| b.cmp(a), |x| *x);
        assert_eq!(items, vec![3, 2, 1]);
    }

    #[test]
    fn sort_and_deduplicate_with_different_sort_and_dedup_keys() {
        // Tuples: sort by second field, dedup by first field
        let mut items = vec![
            ("a", 3),
            ("b", 1),
            ("a", 2),
            ("c", 4),
            ("b", 5),
        ];
        sort_and_deduplicate(
            &mut items,
            |a, b| a.1.cmp(&b.1),
            |x| x.0,
        );
        // After sort by .1: ("b",1), ("a",2), ("a",3), ("c",4), ("b",5)
        // After dedup by .0: ("b",1), ("a",2), ("c",4)
        assert_eq!(items, vec![("b", 1), ("a", 2), ("c", 4)]);
    }

    #[test]
    fn sort_and_deduplicate_empty_input() {
        let mut items: Vec<i32> = vec![];
        sort_and_deduplicate(&mut items, |a, b| a.cmp(b), |x| *x);
        assert!(items.is_empty());
    }

    #[test]
    fn sort_and_deduplicate_single_element() {
        let mut items = vec![42];
        sort_and_deduplicate(&mut items, |a, b| a.cmp(b), |x| *x);
        assert_eq!(items, vec![42]);
    }

    #[test]
    fn sort_and_deduplicate_all_duplicates() {
        let mut items = vec![5, 5, 5, 5];
        sort_and_deduplicate(&mut items, |a, b| a.cmp(b), |x| *x);
        assert_eq!(items, vec![5]);
    }

    #[test]
    fn sort_and_deduplicate_no_duplicates() {
        let mut items = vec![3, 1, 2];
        sort_and_deduplicate(&mut items, |a, b| a.cmp(b), |x| *x);
        assert_eq!(items, vec![1, 2, 3]);
    }
}
