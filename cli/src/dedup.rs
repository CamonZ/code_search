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
