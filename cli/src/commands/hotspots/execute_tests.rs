//! Execute tests for hotspots command.

#[cfg(test)]
mod tests {
    use super::super::HotspotsCmd;
    use crate::commands::CommonArgs;
    use crate::commands::Execute;
    use crate::commands::CommandRunner;
    use crate::output::OutputFormat;
    use db::queries::hotspots::HotspotKind;
    use rstest::{fixture, rstest};

    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,
    }

    // =========================================================================
    // The call_graph fixture contains 15 functions across 5 modules:
    //   Controller (index, show, create)
    //   Accounts (get_user/1, get_user/2, list_users, validate_email)
    //   Service (process, fetch, do_fetch)
    //   Repo (get, all, insert)
    //   Notifier (notify, send_email)
    //
    // Call edges (11 total):
    //   Controller.index -> Accounts.list_users
    //   Controller.show  -> Accounts.get_user/1
    //   Controller.create -> Service.process
    //   Accounts.get_user/1 -> Repo.get
    //   Accounts.get_user/2 -> Repo.get
    //   Accounts.list_users -> Repo.all
    //   Service.process -> Service.fetch
    //   Service.process -> Notifier.notify
    //   Service.fetch -> Service.do_fetch
    //   Service.do_fetch -> Repo.get
    //   Notifier.notify -> Notifier.send_email
    //
    // Derived hotspot values:
    //   Repo.get:         in=3, out=0  (highest incoming)
    //   Service.process:  in=1, out=2  (highest outgoing)
    //   Both above:       total=3      (tied highest total)
    //   Functions with 0 incoming and 0 outgoing: validate_email, insert
    // =========================================================================

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    #[rstest]
    fn test_hotspots_incoming(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind, "incoming");
        assert_eq!(result.entries.len(), 15);
        assert_eq!(result.total_items, 15);

        // First entry should be Repo.get with highest incoming count
        assert_eq!(result.entries[0].module, "MyApp.Repo");
        assert_eq!(result.entries[0].function, "get");
        assert_eq!(result.entries[0].incoming, 3);
        assert_eq!(result.entries[0].outgoing, 0);
        assert_eq!(result.entries[0].total, 3);

        // Functions with zero connections should be last
        let last = result.entries.last().unwrap();
        assert_eq!(last.incoming, 0);
    }

    #[rstest]
    fn test_hotspots_outgoing(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Outgoing,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind, "outgoing");
        assert_eq!(result.entries.len(), 15);
        assert_eq!(result.total_items, 15);

        // First entry should be Service.process with highest outgoing count
        assert_eq!(result.entries[0].module, "MyApp.Service");
        assert_eq!(result.entries[0].function, "process");
        assert_eq!(result.entries[0].outgoing, 2);

        // Last entries should have 0 outgoing
        let last = result.entries.last().unwrap();
        assert_eq!(last.outgoing, 0);
    }

    #[rstest]
    fn test_hotspots_total(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Total,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind, "total");
        assert_eq!(result.entries.len(), 15);
        assert_eq!(result.total_items, 15);

        // Top entry should have total=3 (either Repo.get or Service.process)
        assert_eq!(result.entries[0].total, 3);

        // Verify sorted descending by total
        for window in result.entries.windows(2) {
            assert!(
                window[0].total >= window[1].total,
                "Entries should be sorted by total descending: {} >= {}",
                window[0].total,
                window[1].total,
            );
        }

        // Last entries should have total=0
        let last = result.entries.last().unwrap();
        assert_eq!(last.total, 0);
    }

    #[rstest]
    fn test_hotspots_ratio(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Ratio,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.kind, "ratio");
        assert_eq!(result.entries.len(), 15);
        assert_eq!(result.total_items, 15);

        // Top entries should have ratio 9999.0 (incoming > 0, outgoing == 0)
        assert_eq!(result.entries[0].ratio, 9999.0);

        // Verify sorted descending by ratio
        for window in result.entries.windows(2) {
            assert!(
                window[0].ratio >= window[1].ratio,
                "Entries should be sorted by ratio descending: {} >= {}",
                window[0].ratio,
                window[1].ratio,
            );
        }
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    #[rstest]
    fn test_hotspots_with_module_filter(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: Some("MyApp.Accounts".to_string()),
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 20,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        // MyApp.Accounts has 4 functions: get_user/1, get_user/2, list_users, validate_email
        assert_eq!(result.entries.len(), 4);
        assert_eq!(result.total_items, 4);

        // All entries must be from MyApp.Accounts (exact match, not contains)
        for entry in &result.entries {
            assert_eq!(entry.module, "MyApp.Accounts");
        }

        // Verify specific functions are present
        let function_names: Vec<&str> = result.entries.iter().map(|e| e.function.as_str()).collect();
        assert!(function_names.contains(&"get_user"), "Should contain get_user");
        assert!(function_names.contains(&"list_users"), "Should contain list_users");
        assert!(
            function_names.contains(&"validate_email"),
            "Should contain validate_email"
        );
    }

    #[rstest]
    fn test_hotspots_with_limit(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 2,
            },
        };
        let result = cmd.execute(&*populated_db).expect("Execute should succeed");

        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.total_items, 2);
    }

    #[rstest]
    fn test_hotspots_exclude_generated(populated_db: Box<dyn db::backend::Database>) {
        // First, get results WITHOUT excluding generated
        let cmd_all = HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result_all = cmd_all
            .execute(&*populated_db)
            .expect("Execute should succeed");

        // Then, get results WITH excluding generated
        let cmd_filtered = HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: true,
            common: CommonArgs {
                regex: false,
                limit: 100,
            },
        };
        let result_filtered = cmd_filtered
            .execute(&*populated_db)
            .expect("Execute should succeed");

        assert_eq!(result_filtered.kind, "incoming");

        // The call_graph fixture has no generated functions (generated_by is null for all),
        // so exclude_generated should return the same count since there's nothing to filter.
        // This validates the flag is accepted and the query runs successfully.
        assert_eq!(result_filtered.entries.len(), result_all.entries.len());
    }

    // =========================================================================
    // run() integration test (CommandRunner trait)
    // =========================================================================

    #[rstest]
    fn test_run_produces_formatted_output(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 3,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Table)
            .expect("run() should succeed");

        // Verify the table header
        assert!(
            output.contains("Hotspots (incoming)"),
            "Output should contain header"
        );

        // Verify the summary line reflects the actual count
        assert!(
            output.contains("Found 3 functions:"),
            "Output should show count"
        );

        // Verify the top hotspot (Repo.get) appears in output
        assert!(
            output.contains("MyApp.Repo.get"),
            "Output should contain top hotspot Repo.get"
        );

        // Verify column data is present
        assert!(output.contains("in"), "Output should contain 'in' column");
        assert!(output.contains("out"), "Output should contain 'out' column");
        assert!(
            output.contains("total"),
            "Output should contain 'total' column"
        );
        assert!(
            output.contains("ratio"),
            "Output should contain 'ratio' column"
        );
    }

    #[rstest]
    fn test_run_json_format(populated_db: Box<dyn db::backend::Database>) {
        let cmd = HotspotsCmd {
            module: None,
            kind: HotspotKind::Incoming,
            exclude_generated: false,
            common: CommonArgs {
                regex: false,
                limit: 3,
            },
        };
        let output = cmd
            .run(&*populated_db, OutputFormat::Json)
            .expect("run() should succeed");

        let parsed: serde_json::Value =
            serde_json::from_str(&output).expect("Output should be valid JSON");

        assert_eq!(parsed["kind"], "incoming");
        assert_eq!(parsed["total_items"], 3);
        assert_eq!(
            parsed["entries"].as_array().unwrap().len(),
            3,
            "JSON should contain 3 entries"
        );

        // Verify first entry is Repo.get
        assert_eq!(parsed["entries"][0]["module"], "MyApp.Repo");
        assert_eq!(parsed["entries"][0]["function"], "get");
        assert_eq!(parsed["entries"][0]["incoming"], 3);
    }
}
