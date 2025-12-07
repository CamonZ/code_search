//! Output formatting for location command results.

use crate::output::Outputable;
use super::execute::LocationResult;

impl Outputable for LocationResult {
    fn to_table(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Location: {}.{}", self.module_pattern, self.function_pattern));
        lines.push(String::new());

        if !self.locations.is_empty() {
            lines.push(format!("Found {} location(s):", self.locations.len()));
            for loc in &self.locations {
                let sig = format!("{}.{}/{}", loc.module, loc.name, loc.arity);
                lines.push(format!("  [{}] {} ({})", loc.project, sig, loc.kind));
                lines.push(format!("       {}", loc.format_location()));
            }
        } else {
            lines.push("No locations found.".to_string());
        }

        lines.join("\n")
    }

    fn to_terse(&self) -> String {
        if self.locations.is_empty() {
            String::new()
        } else {
            self.locations
                .iter()
                .map(|l| {
                    format!(
                        "{},{},{},{},{},{},{},{}",
                        l.project, l.file, l.start_line, l.end_line, l.module, l.kind, l.name, l.arity
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::execute::FunctionLocation;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    const EMPTY_TABLE_OUTPUT: &str = "\
Location: MyApp.foo

No locations found.";

    const SINGLE_TABLE_OUTPUT: &str = "\
Location: MyApp.Accounts.get_user

Found 1 location(s):
  [default] MyApp.Accounts.get_user/1 (def)
       lib/my_app/accounts.ex:10:15";

    const MULTIPLE_TABLE_OUTPUT: &str = "\
Location: MyApp.*.user

Found 2 location(s):
  [default] MyApp.Accounts.get_user/1 (def)
       lib/my_app/accounts.ex:10:15
  [default] MyApp.Users.create_user/1 (def)
       lib/my_app/users.ex:5:12";

    const EMPTY_TERSE_OUTPUT: &str = "";
    const SINGLE_TERSE_OUTPUT: &str = "default,lib/my_app/accounts.ex,10,15,MyApp.Accounts,def,get_user,1";
    const MULTIPLE_TERSE_OUTPUT: &str = "\
default,lib/my_app/accounts.ex,10,15,MyApp.Accounts,def,get_user,1
default,lib/my_app/users.ex,5,12,MyApp.Users,def,create_user,1";

    #[fixture]
    fn empty_result() -> LocationResult {
        LocationResult {
            module_pattern: "MyApp".to_string(),
            function_pattern: "foo".to_string(),
            locations: vec![],
        }
    }

    #[fixture]
    fn single_result() -> LocationResult {
        LocationResult {
            module_pattern: "MyApp.Accounts".to_string(),
            function_pattern: "get_user".to_string(),
            locations: vec![FunctionLocation {
                project: "default".to_string(),
                file: "lib/my_app/accounts.ex".to_string(),
                start_line: 10,
                end_line: 15,
                module: "MyApp.Accounts".to_string(),
                kind: "def".to_string(),
                name: "get_user".to_string(),
                arity: 1,
            }],
        }
    }

    #[fixture]
    fn multiple_result() -> LocationResult {
        LocationResult {
            module_pattern: "MyApp.*".to_string(),
            function_pattern: "user".to_string(),
            locations: vec![
                FunctionLocation {
                    project: "default".to_string(),
                    file: "lib/my_app/accounts.ex".to_string(),
                    start_line: 10,
                    end_line: 15,
                    module: "MyApp.Accounts".to_string(),
                    kind: "def".to_string(),
                    name: "get_user".to_string(),
                    arity: 1,
                },
                FunctionLocation {
                    project: "default".to_string(),
                    file: "lib/my_app/users.ex".to_string(),
                    start_line: 5,
                    end_line: 12,
                    module: "MyApp.Users".to_string(),
                    kind: "def".to_string(),
                    name: "create_user".to_string(),
                    arity: 1,
                },
            ],
        }
    }

    #[rstest]
    fn test_to_table_empty(empty_result: LocationResult) {
        assert_eq!(empty_result.to_table(), EMPTY_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_single(single_result: LocationResult) {
        assert_eq!(single_result.to_table(), SINGLE_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_table_multiple(multiple_result: LocationResult) {
        assert_eq!(multiple_result.to_table(), MULTIPLE_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_to_terse_empty(empty_result: LocationResult) {
        assert_eq!(empty_result.to_terse(), EMPTY_TERSE_OUTPUT);
    }

    #[rstest]
    fn test_to_terse_single(single_result: LocationResult) {
        assert_eq!(single_result.to_terse(), SINGLE_TERSE_OUTPUT);
    }

    #[rstest]
    fn test_to_terse_multiple(multiple_result: LocationResult) {
        assert_eq!(multiple_result.to_terse(), MULTIPLE_TERSE_OUTPUT);
    }

    #[rstest]
    fn test_format_json(single_result: LocationResult) {
        let output = single_result.format(OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(parsed["module_pattern"], "MyApp.Accounts");
        assert_eq!(parsed["function_pattern"], "get_user");
        assert_eq!(parsed["locations"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["locations"][0]["file"], "lib/my_app/accounts.ex");
    }

    #[rstest]
    fn test_format_toon(single_result: LocationResult) {
        let output = single_result.format(OutputFormat::Toon);
        assert!(output.contains("module_pattern: MyApp.Accounts"));
        assert!(output.contains("function_pattern: get_user"));
    }

    #[rstest]
    fn test_format_toon_location_fields(single_result: LocationResult) {
        let output = single_result.format(OutputFormat::Toon);
        // Toon format renders arrays with header showing field names and count
        assert!(output.contains("locations[1]{"));
        // Field names in header (alphabetically sorted by toon)
        assert!(output.contains("arity"));
        assert!(output.contains("end_line"));
        assert!(output.contains("file"));
        assert!(output.contains("kind"));
        assert!(output.contains("module"));
        assert!(output.contains("name"));
        assert!(output.contains("project"));
        assert!(output.contains("start_line"));
        // Values are rendered in a row: arity, end_line, file, kind, module, name, project, start_line
        assert!(output.contains("1,15,lib/my_app/accounts.ex,def,MyApp.Accounts,get_user,default,10"));
    }

    #[rstest]
    fn test_format_toon_empty(empty_result: LocationResult) {
        let output = empty_result.format(OutputFormat::Toon);
        assert!(output.contains("module_pattern: MyApp"));
        assert!(output.contains("function_pattern: foo"));
        // Empty array is rendered as locations[0]
        assert!(output.contains("locations[0]"));
    }
}
