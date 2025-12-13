//! Output formatting tests for duplicates command.

#[cfg(test)]
mod tests {
    use super::super::execute::{DuplicatesResult, DuplicateGroup, DuplicateFunctionEntry};
    use crate::output::{OutputFormat, Outputable};

    #[test]
    fn test_to_table_empty() {
        let result = DuplicatesResult {
            total_groups: 0,
            total_duplicates: 0,
            groups: vec![],
        };

        let output = result.to_table();
        assert!(output.contains("Duplicate Functions"));
        assert!(output.contains("No duplicate functions found"));
    }

    #[test]
    fn test_to_table_single_group() {
        let result = DuplicatesResult {
            total_groups: 1,
            total_duplicates: 2,
            groups: vec![DuplicateGroup {
                hash: "abc123def456".to_string(),
                functions: vec![
                    DuplicateFunctionEntry {
                        module: "MyApp.User".to_string(),
                        name: "validate".to_string(),
                        arity: 1,
                        line: 10,
                        file: "lib/my_app/user.ex".to_string(),
                    },
                    DuplicateFunctionEntry {
                        module: "MyApp.Post".to_string(),
                        name: "validate".to_string(),
                        arity: 1,
                        line: 15,
                        file: "lib/my_app/post.ex".to_string(),
                    },
                ],
            }],
        };

        let output = result.to_table();
        assert!(output.contains("Duplicate Functions"));
        assert!(output.contains("Found 1 group(s)"));
        assert!(output.contains("MyApp.User.validate/1"));
        assert!(output.contains("MyApp.Post.validate/1"));
        assert!(output.contains("lib/my_app/user.ex"));
        assert!(output.contains("lib/my_app/post.ex"));
    }

    #[test]
    fn test_to_table_multiple_groups() {
        let result = DuplicatesResult {
            total_groups: 2,
            total_duplicates: 5,
            groups: vec![
                DuplicateGroup {
                    hash: "hash_a".to_string(),
                    functions: vec![
                        DuplicateFunctionEntry {
                            module: "A".to_string(),
                            name: "f1".to_string(),
                            arity: 1,
                            line: 10,
                            file: "a.ex".to_string(),
                        },
                        DuplicateFunctionEntry {
                            module: "B".to_string(),
                            name: "f1".to_string(),
                            arity: 1,
                            line: 20,
                            file: "b.ex".to_string(),
                        },
                    ],
                },
                DuplicateGroup {
                    hash: "hash_b".to_string(),
                    functions: vec![
                        DuplicateFunctionEntry {
                            module: "C".to_string(),
                            name: "f2".to_string(),
                            arity: 2,
                            line: 30,
                            file: "c.ex".to_string(),
                        },
                        DuplicateFunctionEntry {
                            module: "D".to_string(),
                            name: "f2".to_string(),
                            arity: 2,
                            line: 40,
                            file: "d.ex".to_string(),
                        },
                        DuplicateFunctionEntry {
                            module: "E".to_string(),
                            name: "f2".to_string(),
                            arity: 2,
                            line: 50,
                            file: "e.ex".to_string(),
                        },
                    ],
                },
            ],
        };

        let output = result.to_table();
        assert!(output.contains("Found 2 group(s)"));
        assert!(output.contains("5 function(s)"));
        assert!(output.contains("Group 1"));
        assert!(output.contains("Group 2"));
        assert!(output.contains("A.f1/1"));
        assert!(output.contains("C.f2/2"));
    }

    #[test]
    fn test_hash_truncation() {
        let long_hash = "abcdefghijklmnopqrstuvwxyz1234567890";
        let result = DuplicatesResult {
            total_groups: 1,
            total_duplicates: 2,
            groups: vec![DuplicateGroup {
                hash: long_hash.to_string(),
                functions: vec![
                    DuplicateFunctionEntry {
                        module: "A".to_string(),
                        name: "f".to_string(),
                        arity: 0,
                        line: 1,
                        file: "a.ex".to_string(),
                    },
                    DuplicateFunctionEntry {
                        module: "B".to_string(),
                        name: "f".to_string(),
                        arity: 0,
                        line: 2,
                        file: "b.ex".to_string(),
                    },
                ],
            }],
        };

        let output = result.to_table();
        // Hash should be truncated with "..."
        assert!(output.contains("..."));
    }

    #[test]
    fn test_format_json() {
        let result = DuplicatesResult {
            total_groups: 1,
            total_duplicates: 2,
            groups: vec![DuplicateGroup {
                hash: "abc".to_string(),
                functions: vec![
                    DuplicateFunctionEntry {
                        module: "M".to_string(),
                        name: "f".to_string(),
                        arity: 1,
                        line: 10,
                        file: "m.ex".to_string(),
                    },
                ],
            }],
        };

        let output = result.format(OutputFormat::Json);
        assert!(output.contains("total_groups"));
        assert!(output.contains("total_duplicates"));
        assert!(output.contains("groups"));
        assert!(output.contains("\"hash\""));
        assert!(output.contains("\"functions\""));
    }

    #[test]
    fn test_format_toon() {
        let result = DuplicatesResult {
            total_groups: 1,
            total_duplicates: 2,
            groups: vec![DuplicateGroup {
                hash: "abc".to_string(),
                functions: vec![
                    DuplicateFunctionEntry {
                        module: "M".to_string(),
                        name: "f".to_string(),
                        arity: 1,
                        line: 10,
                        file: "m.ex".to_string(),
                    },
                ],
            }],
        };

        let output = result.format(OutputFormat::Toon);
        // Toon format should contain key parts
        assert!(output.contains("total_groups"));
        assert!(output.contains("1")); // count value
    }

    #[test]
    fn test_format_table() {
        let result = DuplicatesResult {
            total_groups: 1,
            total_duplicates: 2,
            groups: vec![DuplicateGroup {
                hash: "abc".to_string(),
                functions: vec![
                    DuplicateFunctionEntry {
                        module: "M".to_string(),
                        name: "f".to_string(),
                        arity: 1,
                        line: 10,
                        file: "m.ex".to_string(),
                    },
                ],
            }],
        };

        let output = result.format(OutputFormat::Table);
        assert!(output.contains("Duplicate Functions"));
        assert!(output.contains("M.f/1"));
    }
}
