//! Core types for representing function calls.

use serde::Serialize;

/// A function reference with optional definition location and type information.
/// Queries populate only the fields they need - optional fields are skipped during serialization.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct FunctionRef {
    pub module: String,
    pub name: String,
    pub arity: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
}

impl FunctionRef {
    /// Create a minimal function reference (module, name, arity only).
    pub fn new(module: impl Into<String>, name: impl Into<String>, arity: i64) -> Self {
        Self {
            module: module.into(),
            name: name.into(),
            arity,
            kind: None,
            file: None,
            start_line: None,
            end_line: None,
            args: None,
            return_type: None,
        }
    }

    /// Create a function reference with full definition info.
    pub fn with_definition(
        module: impl Into<String>,
        name: impl Into<String>,
        arity: i64,
        kind: impl Into<String>,
        file: impl Into<String>,
        start_line: i64,
        end_line: i64,
    ) -> Self {
        Self {
            module: module.into(),
            name: name.into(),
            arity,
            kind: Some(kind.into()),
            file: Some(file.into()),
            start_line: Some(start_line),
            end_line: Some(end_line),
            args: None,
            return_type: None,
        }
    }

    /// Create a function reference with type information.
    pub fn with_types(
        module: impl Into<String>,
        name: impl Into<String>,
        arity: i64,
        kind: impl Into<String>,
        file: impl Into<String>,
        start_line: i64,
        end_line: i64,
        args: impl Into<String>,
        return_type: impl Into<String>,
    ) -> Self {
        Self {
            module: module.into(),
            name: name.into(),
            arity,
            kind: Some(kind.into()),
            file: Some(file.into()),
            start_line: Some(start_line),
            end_line: Some(end_line),
            args: Some(args.into()),
            return_type: Some(return_type.into()),
        }
    }

    /// Format as "name/arity" or "Module.name/arity" if module differs from context.
    pub fn format_name(&self, context_module: Option<&str>) -> String {
        if context_module == Some(self.module.as_str()) {
            format!("{}/{}", self.name, self.arity)
        } else {
            format!("{}.{}/{}", self.module, self.name, self.arity)
        }
    }

    /// Format location as "L42:50" or "file.ex:L42:50".
    /// Returns None if no location info available.
    pub fn format_location(&self, context_file: Option<&str>) -> Option<String> {
        let (start, end) = match (self.start_line, self.end_line) {
            (Some(s), Some(e)) => (s, e),
            _ => return None,
        };

        let file = self.file.as_deref()?;
        let filename = file.rsplit('/').next().unwrap_or(file);
        let context_filename = context_file
            .map(|f| f.rsplit('/').next().unwrap_or(f));

        if context_filename == Some(filename) {
            Some(format!("L{}:{}", start, end))
        } else {
            Some(format!("{}:L{}:{}", filename, start, end))
        }
    }

    /// Format kind as "[def]" or empty string if no kind.
    pub fn format_kind(&self) -> String {
        self.kind
            .as_ref()
            .filter(|k| !k.is_empty())
            .map(|k| format!(" [{}]", k))
            .unwrap_or_default()
    }
}

/// A directed call relationship.
#[derive(Debug, Clone, Serialize)]
pub struct Call {
    pub caller: FunctionRef,
    pub callee: FunctionRef,
    pub line: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
}

impl Call {
    /// Check if this is a struct construction call (e.g., %MyStruct{}).
    pub fn is_struct_call(&self) -> bool {
        self.callee.name == "%"
    }

    /// Format as outgoing call: "→ @ L37 name/arity [kind] (location)"
    pub fn format_outgoing(&self, context_module: &str, context_file: &str) -> String {
        let name = self.callee.format_name(Some(context_module));
        let kind = self.callee.format_kind();
        let location = self
            .callee
            .format_location(Some(context_file))
            .map(|loc| format!(" ({})", loc))
            .unwrap_or_default();

        format!("→ @ L{} {}{}{}", self.line, name, kind, location)
    }

    /// Format as incoming call: "← @ L37 name/arity [kind] (location)"
    pub fn format_incoming(&self, context_module: &str, context_file: &str) -> String {
        let name = self.caller.format_name(Some(context_module));
        let kind = self.caller.format_kind();
        let location = self
            .caller
            .format_location(Some(context_file))
            .map(|loc| format!(" ({})", loc))
            .unwrap_or_default();

        format!("← @ L{} {}{}{}", self.line, name, kind, location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_ref_format_name_same_module() {
        let func = FunctionRef::new("MyModule", "my_func", 2);
        assert_eq!(func.format_name(Some("MyModule")), "my_func/2");
    }

    #[test]
    fn test_function_ref_format_name_different_module() {
        let func = FunctionRef::new("OtherModule", "other_func", 1);
        assert_eq!(
            func.format_name(Some("MyModule")),
            "OtherModule.other_func/1"
        );
    }

    #[test]
    fn test_function_ref_format_location_same_file() {
        let func = FunctionRef::with_definition(
            "MyModule",
            "my_func",
            2,
            "def",
            "/path/to/my_module.ex",
            10,
            20,
        );
        assert_eq!(
            func.format_location(Some("/other/path/my_module.ex")),
            Some("L10:20".to_string())
        );
    }

    #[test]
    fn test_function_ref_format_location_different_file() {
        let func = FunctionRef::with_definition(
            "MyModule",
            "my_func",
            2,
            "def",
            "/path/to/my_module.ex",
            10,
            20,
        );
        assert_eq!(
            func.format_location(Some("/path/to/other.ex")),
            Some("my_module.ex:L10:20".to_string())
        );
    }

    #[test]
    fn test_call_format_outgoing() {
        let call = Call {
            caller: FunctionRef::with_definition(
                "MyModule",
                "caller_func",
                1,
                "def",
                "/path/to/my_module.ex",
                10,
                30,
            ),
            callee: FunctionRef::with_definition(
                "MyModule",
                "callee_func",
                2,
                "defp",
                "/path/to/my_module.ex",
                40,
                50,
            ),
            line: 25,
            call_type: None,
            depth: None,
        };

        assert_eq!(
            call.format_outgoing("MyModule", "/path/to/my_module.ex"),
            "→ @ L25 callee_func/2 [defp] (L40:50)"
        );
    }

    #[test]
    fn test_call_format_outgoing_different_module() {
        let call = Call {
            caller: FunctionRef::new("MyModule", "caller_func", 1),
            callee: FunctionRef::with_definition(
                "OtherModule",
                "other_func",
                0,
                "def",
                "/path/to/other.ex",
                5,
                15,
            ),
            line: 12,
            call_type: None,
            depth: None,
        };

        assert_eq!(
            call.format_outgoing("MyModule", "/path/to/my_module.ex"),
            "→ @ L12 OtherModule.other_func/0 [def] (other.ex:L5:15)"
        );
    }

    #[test]
    fn test_is_struct_call() {
        let struct_call = Call {
            caller: FunctionRef::new("MyModule", "func", 1),
            callee: FunctionRef::new("MyStruct", "%", 2),
            line: 10,
            call_type: None,
            depth: None,
        };
        assert!(struct_call.is_struct_call());

        let normal_call = Call {
            caller: FunctionRef::new("MyModule", "func", 1),
            callee: FunctionRef::new("OtherModule", "other", 0),
            line: 10,
            call_type: None,
            depth: None,
        };
        assert!(!normal_call.is_struct_call());
    }
}
