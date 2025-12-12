//! Deduplication utilities for reducing code duplication across commands.
//!
//! This module provides reusable patterns for deduplicating collections using different strategies:
//! - Strategy A: HashSet retain pattern (deduplicate_retain) - for in-place deduplication after sorting
//! - Strategy B: HashSet prevention pattern (DeduplicationFilter) - for preventing duplicates during collection

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
    K: Eq + Hash + Clone,
{
    let mut seen: HashSet<K> = HashSet::new();
    items.retain(|item| seen.insert(key_fn(item)));
}

/// Strategy B: HashSet prevention pattern - check before adding
///
/// Use this when collecting items and you want to prevent duplicates from being added
/// in the first place, without needing to sort or post-process.
///
/// # Example
/// ```ignore
/// let mut filter = DeduplicationFilter::new();
/// for entry in entries {
///     if filter.should_process(entry_key) {
///         // Add entry to result
///     }
/// }
/// ```
#[derive(Debug)]
pub struct DeduplicationFilter<K: Eq + Hash> {
    processed: HashSet<K>,
}

impl<K: Eq + Hash> DeduplicationFilter<K> {
    /// Create a new empty deduplication filter
    pub fn new() -> Self {
        Self {
            processed: HashSet::new(),
        }
    }

    /// Check if a key should be processed (inserted into the set)
    ///
    /// Returns true if the key is new and was successfully inserted, false if it was already present.
    pub fn should_process(&mut self, key: K) -> bool {
        self.processed.insert(key)
    }
}

impl<K: Eq + Hash> Default for DeduplicationFilter<K> {
    fn default() -> Self {
        Self::new()
    }
}
