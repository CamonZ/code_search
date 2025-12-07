//! Output formatting for search command results.

use crate::output::Outputable;
use super::execute::SearchResult;

impl Outputable for SearchResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Search: {} ({})", self.pattern, self.kind));
        lines.push(String::new());

        if !self.modules.is_empty() {
            lines.push(format!("Modules ({}):", self.modules.len()));
            for m in &self.modules {
                lines.push(format!("  [{}] {}", m.project, m.name));
            }
        }

        if !self.functions.is_empty() {
            lines.push(format!("Functions ({}):", self.functions.len()));
            for f in &self.functions {
                let sig = if f.return_type.is_empty() {
                    format!("{}.{}/{}", f.module, f.name, f.arity)
                } else {
                    format!("{}.{}/{} -> {}", f.module, f.name, f.arity, f.return_type)
                };
                lines.push(format!("  [{}] {}", f.project, sig));
            }
        }

        if self.modules.is_empty() && self.functions.is_empty() {
            lines.push("No results found.".to_string());
        }

        lines.join("\n")
    }

    fn to_terse(&self) -> String {
        let count = self.modules.len() + self.functions.len();
        format!("kind={} pattern={} count={}", self.kind, self.pattern, count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::{ModuleResult, FunctionResult};
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    const EMPTY_TABLE_OUTPUT: &str = "\
Search: test (modules)

No results found.";

    const MODULES_TABLE_OUTPUT: &str = "\
Search: MyApp (modules)

Modules (2):
  [default] MyApp.Accounts
  [default] MyApp.Users";

    const FUNCTIONS_TABLE_OUTPUT: &str = "\
Search: get_ (functions)

Functions (1):
  [default] MyApp.Accounts.get_user/1 -> User.t()";

    const EMPTY_TERSE_OUTPUT: &str = "kind=modules pattern=test count=0";
    const MODULES_TERSE_OUTPUT: &str = "kind=modules pattern=MyApp count=2";
    const FUNCTIONS_TERSE_OUTPUT: &str = "kind=functions pattern=get_ count=1";

    #[fixture]
    fn empty_result() -> SearchResult {
        SearchResult {
            pattern: "test".to_string(),
            kind: "modules".to_string(),
            modules: vec![],
            functions: vec![],
        }
    }

    #[fixture]
    fn modules_result() -> SearchResult {
        SearchResult {
            pattern: "MyApp".to_string(),
            kind: "modules".to_string(),
            modules: vec![
                ModuleResult {
                    project: "default".to_string(),
                    name: "MyApp.Accounts".to_string(),
                    source: "unknown".to_string(),
                },
                ModuleResult {
                    project: "default".to_string(),
                    name: "MyApp.Users".to_string(),
                    source: "unknown".to_string(),
                },
            ],
            functions: vec![],
        }
    }

    #[fixture]
    fn functions_result() -> SearchResult {
        SearchResult {
            pattern: "get_".to_string(),
            kind: "functions".to_string(),
            modules: vec![],
            functions: vec![FunctionResult {
                project: "default".to_string(),
                module: "MyApp.Accounts".to_string(),
                name: "get_user".to_string(),
                arity: 1,
                return_type: "User.t()".to_string(),
            }],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: SearchResult) {
        assert_eq!(empty_result.to_table(), EMPTY_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_modules(modules_result: SearchResult) {
        assert_eq!(modules_result.to_table(), MODULES_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_functions(functions_result: SearchResult) {
        assert_eq!(functions_result.to_table(), FUNCTIONS_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_terse_empty(empty_result: SearchResult) {
        assert_eq!(empty_result.to_terse(), EMPTY_TERSE_OUTPUT);
    }

    #[rstest]
    fn test_to_terse_modules(modules_result: SearchResult) {
        assert_eq!(modules_result.to_terse(), MODULES_TERSE_OUTPUT);
    }

    #[rstest]
    fn test_to_terse_functions(functions_result: SearchResult) {
        assert_eq!(functions_result.to_terse(), FUNCTIONS_TERSE_OUTPUT);
    }

    #[rstest]
    fn test_format_json(modules_result: SearchResult) {
        let output = modules_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["pattern"], "MyApp");
        assert_eq!(parsed["kind"], "modules");
        assert_eq!(parsed["modules"].as_array().unwrap().len(), 2);
    }

    #[rstest]
    fn test_format_toon(modules_result: SearchResult) {
        let output = modules_result.format(OutputFormat::Toon);
        assert!(output.contains("pattern: MyApp"));
        assert!(output.contains("kind: modules"));
    }
}
