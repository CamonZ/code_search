//! Parameter binding helpers for query building.
//!
//! Provides utilities for collecting query parameters in a type-safe way
//! before compiling them into backend-specific placeholder syntax.

use std::collections::BTreeMap;
use cozo::DataValue;
use crate::db::Params;

/// Helper for building query parameters.
///
/// Collects named parameters into a `BTreeMap` that can be used with
/// compiled queries. Parameters are stored as `DataValue` but can be
/// constructed from any type that converts to `DataValue`.
#[derive(Debug, Clone)]
pub struct ParamBuilder {
    params: BTreeMap<String, DataValue>,
}

impl ParamBuilder {
    /// Create a new, empty parameter builder.
    pub fn new() -> Self {
        Self {
            params: BTreeMap::new(),
        }
    }

    /// Add a parameter to the builder.
    ///
    /// # Arguments
    ///
    /// * `name` - The parameter name (e.g., "module", "limit")
    /// * `value` - The parameter value, converted to `DataValue`
    pub fn add<T: Into<DataValue>>(&mut self, name: impl Into<String>, value: T) {
        self.params.insert(name.into(), value.into());
    }

    /// Build the final `Params` map.
    pub fn build(self) -> Params {
        self.params
    }

    /// Get a reference to the underlying params map.
    pub fn params(&self) -> &Params {
        &self.params
    }

    /// Get the number of parameters collected so far.
    pub fn len(&self) -> usize {
        self.params.len()
    }

    /// Check if the builder is empty.
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }
}

impl Default for ParamBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_param_builder_new() {
        let builder = ParamBuilder::new();
        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_param_builder_add_string() {
        let mut builder = ParamBuilder::new();
        builder.add("name", "test_value");
        assert_eq!(builder.len(), 1);

        let params = builder.build();
        assert!(params.contains_key("name"));
    }

    #[test]
    fn test_param_builder_add_i64() {
        let mut builder = ParamBuilder::new();
        builder.add("limit", 10i64);
        assert_eq!(builder.len(), 1);

        let params = builder.build();
        assert!(params.contains_key("limit"));
    }

    #[test]
    fn test_param_builder_add_multiple() {
        let mut builder = ParamBuilder::new();
        builder.add("x", 1i64);
        builder.add("y", "test");
        builder.add("z", 3.14f64);

        assert_eq!(builder.len(), 3);
        let params = builder.build();
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_param_builder_default() {
        let builder = ParamBuilder::default();
        assert!(builder.is_empty());
    }

    #[test]
    fn test_param_builder_clone() {
        let mut builder = ParamBuilder::new();
        builder.add("key", 42i64);

        let cloned = builder.clone();
        assert_eq!(cloned.len(), 1);
    }

    #[test]
    fn test_param_builder_params_ref() {
        let mut builder = ParamBuilder::new();
        builder.add("test", "value");

        let params = builder.params();
        assert!(params.contains_key("test"));
    }

    #[test]
    fn test_param_builder_overwrite() {
        let mut builder = ParamBuilder::new();
        builder.add("key", 1i64);
        builder.add("key", 2i64);
        assert_eq!(builder.len(), 1);

        let params = builder.build();
        if let Some(DataValue::Num(cozo::Num::Int(v))) = params.get("key") {
            assert_eq!(*v, 2);
        } else {
            panic!("Expected i64 value");
        }
    }
}
