//! Output formatting tests for duplicates command.

#[cfg(test)]
mod tests {
    use super::super::execute::{
        DuplicateFunctionEntry, DuplicateGroup, DuplicateSummary, DuplicatesByModuleResult,
        DuplicatesOutput, DuplicatesResult, ModuleDuplicates,
    };
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

    // =========================================================================
    // By-module output tests
    // =========================================================================

    #[test]
    fn test_by_module_to_table_empty() {
        let result = DuplicatesByModuleResult {
            total_modules: 0,
            total_duplicates: 0,
            modules: vec![],
        };

        let output = result.to_table();
        assert!(output.contains("Modules with Most Duplicates"));
        assert!(output.contains("No duplicate functions found"));
    }

    #[test]
    fn test_by_module_to_table_single() {
        let result = DuplicatesByModuleResult {
            total_modules: 1,
            total_duplicates: 3,
            modules: vec![ModuleDuplicates {
                name: "MyApp.Utils".to_string(),
                duplicate_count: 3,
                top_duplicates: vec![
                    DuplicateSummary {
                        name: "validate".to_string(),
                        arity: 1,
                        copy_count: 2,
                    },
                    DuplicateSummary {
                        name: "format".to_string(),
                        arity: 2,
                        copy_count: 1,
                    },
                ],
            }],
        };

        let output = result.to_table();
        assert!(output.contains("Modules with Most Duplicates"));
        assert!(output.contains("Found 3 duplicated function(s) across 1 module(s)"));
        assert!(output.contains("MyApp.Utils (3 duplicates)"));
        assert!(output.contains("validate/1 (2 copies)"));
        assert!(output.contains("format/2 (1 copies)"));
    }

    #[test]
    fn test_by_module_to_table_multiple() {
        let result = DuplicatesByModuleResult {
            total_modules: 2,
            total_duplicates: 5,
            modules: vec![
                ModuleDuplicates {
                    name: "MyApp.Users".to_string(),
                    duplicate_count: 3,
                    top_duplicates: vec![DuplicateSummary {
                        name: "validate".to_string(),
                        arity: 1,
                        copy_count: 3,
                    }],
                },
                ModuleDuplicates {
                    name: "MyApp.Posts".to_string(),
                    duplicate_count: 2,
                    top_duplicates: vec![DuplicateSummary {
                        name: "format".to_string(),
                        arity: 0,
                        copy_count: 2,
                    }],
                },
            ],
        };

        let output = result.to_table();
        assert!(output.contains("Found 5 duplicated function(s) across 2 module(s)"));
        assert!(output.contains("MyApp.Users (3 duplicates)"));
        assert!(output.contains("MyApp.Posts (2 duplicates)"));
    }

    #[test]
    fn test_by_module_format_json() {
        let result = DuplicatesByModuleResult {
            total_modules: 1,
            total_duplicates: 2,
            modules: vec![ModuleDuplicates {
                name: "MyApp".to_string(),
                duplicate_count: 2,
                top_duplicates: vec![DuplicateSummary {
                    name: "f".to_string(),
                    arity: 1,
                    copy_count: 2,
                }],
            }],
        };

        let output = result.format(OutputFormat::Json);
        assert!(output.contains("\"total_modules\""));
        assert!(output.contains("\"total_duplicates\""));
        assert!(output.contains("\"modules\""));
        assert!(output.contains("\"name\""));
        assert!(output.contains("\"duplicate_count\""));
        assert!(output.contains("\"top_duplicates\""));
        assert!(output.contains("\"copy_count\""));
    }

    #[test]
    fn test_by_module_format_toon() {
        let result = DuplicatesByModuleResult {
            total_modules: 1,
            total_duplicates: 2,
            modules: vec![ModuleDuplicates {
                name: "MyApp".to_string(),
                duplicate_count: 2,
                top_duplicates: vec![DuplicateSummary {
                    name: "f".to_string(),
                    arity: 1,
                    copy_count: 2,
                }],
            }],
        };

        let output = result.format(OutputFormat::Toon);
        assert!(output.contains("total_modules"));
        assert!(output.contains("total_duplicates"));
    }

    // =========================================================================
    // DuplicatesOutput enum tests
    // =========================================================================

    #[test]
    fn test_output_enum_detailed_empty() {
        let result = DuplicatesOutput::Detailed(DuplicatesResult {
            total_groups: 0,
            total_duplicates: 0,
            groups: vec![],
        });

        let output = result.to_table();
        assert!(output.contains("Duplicate Functions"));
        assert!(output.contains("No duplicate functions found"));
    }

    #[test]
    fn test_output_enum_by_module_empty() {
        let result = DuplicatesOutput::ByModule(DuplicatesByModuleResult {
            total_modules: 0,
            total_duplicates: 0,
            modules: vec![],
        });

        let output = result.to_table();
        assert!(output.contains("Modules with Most Duplicates"));
        assert!(output.contains("No duplicate functions found"));
    }

    #[test]
    fn test_output_enum_detailed_with_data() {
        let result = DuplicatesOutput::Detailed(DuplicatesResult {
            total_groups: 1,
            total_duplicates: 2,
            groups: vec![DuplicateGroup {
                hash: "abc123".to_string(),
                functions: vec![
                    DuplicateFunctionEntry {
                        module: "MyApp.User".to_string(),
                        name: "validate".to_string(),
                        arity: 1,
                        line: 10,
                        file: "lib/user.ex".to_string(),
                    },
                    DuplicateFunctionEntry {
                        module: "MyApp.Post".to_string(),
                        name: "validate".to_string(),
                        arity: 1,
                        line: 20,
                        file: "lib/post.ex".to_string(),
                    },
                ],
            }],
        });

        // Table format
        let table = result.to_table();
        assert!(table.contains("Duplicate Functions"));
        assert!(table.contains("Found 1 group(s)"));
        assert!(table.contains("MyApp.User.validate/1"));
        assert!(table.contains("MyApp.Post.validate/1"));

        // JSON format
        let json = result.format(OutputFormat::Json);
        assert!(json.contains("\"total_groups\": 1"));
        assert!(json.contains("\"total_duplicates\": 2"));
        assert!(json.contains("\"hash\": \"abc123\""));
        assert!(json.contains("\"module\": \"MyApp.User\""));

        // Toon format
        let toon = result.format(OutputFormat::Toon);
        assert!(toon.contains("total_groups"));
        assert!(toon.contains("groups"));
    }

    #[test]
    fn test_output_enum_by_module_with_data() {
        let result = DuplicatesOutput::ByModule(DuplicatesByModuleResult {
            total_modules: 2,
            total_duplicates: 5,
            modules: vec![
                ModuleDuplicates {
                    name: "MyApp.Users".to_string(),
                    duplicate_count: 3,
                    top_duplicates: vec![DuplicateSummary {
                        name: "validate".to_string(),
                        arity: 1,
                        copy_count: 3,
                    }],
                },
                ModuleDuplicates {
                    name: "MyApp.Posts".to_string(),
                    duplicate_count: 2,
                    top_duplicates: vec![DuplicateSummary {
                        name: "format".to_string(),
                        arity: 0,
                        copy_count: 2,
                    }],
                },
            ],
        });

        // Table format
        let table = result.to_table();
        assert!(table.contains("Modules with Most Duplicates"));
        assert!(table.contains("Found 5 duplicated function(s) across 2 module(s)"));
        assert!(table.contains("MyApp.Users (3 duplicates)"));
        assert!(table.contains("MyApp.Posts (2 duplicates)"));

        // JSON format
        let json = result.format(OutputFormat::Json);
        assert!(json.contains("\"total_modules\": 2"));
        assert!(json.contains("\"total_duplicates\": 5"));
        assert!(json.contains("\"name\": \"MyApp.Users\""));
        assert!(json.contains("\"duplicate_count\": 3"));

        // Toon format
        let toon = result.format(OutputFormat::Toon);
        assert!(toon.contains("total_modules"));
        assert!(toon.contains("modules"));
    }

    #[test]
    fn test_output_enum_detailed_json_structure() {
        let result = DuplicatesOutput::Detailed(DuplicatesResult {
            total_groups: 1,
            total_duplicates: 2,
            groups: vec![DuplicateGroup {
                hash: "test_hash".to_string(),
                functions: vec![
                    DuplicateFunctionEntry {
                        module: "A".to_string(),
                        name: "func".to_string(),
                        arity: 0,
                        line: 1,
                        file: "a.ex".to_string(),
                    },
                ],
            }],
        });

        let json = result.format(OutputFormat::Json);
        // Verify JSON structure has expected fields
        assert!(json.contains("\"total_groups\""));
        assert!(json.contains("\"total_duplicates\""));
        assert!(json.contains("\"groups\""));
        assert!(json.contains("\"hash\""));
        assert!(json.contains("\"functions\""));
        assert!(json.contains("\"module\""));
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"arity\""));
        assert!(json.contains("\"line\""));
        assert!(json.contains("\"file\""));
    }

    #[test]
    fn test_output_enum_by_module_json_structure() {
        let result = DuplicatesOutput::ByModule(DuplicatesByModuleResult {
            total_modules: 1,
            total_duplicates: 2,
            modules: vec![ModuleDuplicates {
                name: "TestModule".to_string(),
                duplicate_count: 2,
                top_duplicates: vec![DuplicateSummary {
                    name: "func".to_string(),
                    arity: 1,
                    copy_count: 2,
                }],
            }],
        });

        let json = result.format(OutputFormat::Json);
        // Verify JSON structure has expected fields
        assert!(json.contains("\"total_modules\""));
        assert!(json.contains("\"total_duplicates\""));
        assert!(json.contains("\"modules\""));
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"duplicate_count\""));
        assert!(json.contains("\"top_duplicates\""));
        assert!(json.contains("\"arity\""));
        assert!(json.contains("\"copy_count\""));
    }
}
