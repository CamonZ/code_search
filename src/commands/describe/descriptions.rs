//! Centralized descriptions for all available commands.

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandCategory {
    Query,
    Analysis,
    Search,
    Type,
    Module,
    Other,
}

impl std::fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query => write!(f, "Query Commands"),
            Self::Analysis => write!(f, "Analysis Commands"),
            Self::Search => write!(f, "Search Commands"),
            Self::Type => write!(f, "Type Search Commands"),
            Self::Module => write!(f, "Module Commands"),
            Self::Other => write!(f, "Other Commands"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub description: String,
    pub command: String,
}

impl Example {
    pub fn new(description: &str, command: &str) -> Self {
        Self {
            description: description.to_string(),
            command: command.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDescription {
    pub name: String,
    pub brief: String,
    pub category: CommandCategory,
    pub description: String,
    pub usage: String,
    pub examples: Vec<Example>,
    pub related: Vec<String>,
}

impl CommandDescription {
    pub fn new(
        name: &str,
        brief: &str,
        category: CommandCategory,
        description: &str,
        usage: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            brief: brief.to_string(),
            category,
            description: description.to_string(),
            usage: usage.to_string(),
            examples: Vec::new(),
            related: Vec::new(),
        }
    }

    pub fn with_examples(mut self, examples: Vec<Example>) -> Self {
        self.examples = examples;
        self
    }

    pub fn with_related(mut self, related: Vec<&str>) -> Self {
        self.related = related.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// Get all available command descriptions
pub fn all_descriptions() -> Vec<CommandDescription> {
    vec![
        // Query Commands
        CommandDescription::new(
            "calls-to",
            "Find callers of a given function",
            CommandCategory::Query,
            "Finds all functions that call a specific function. Use this to answer: 'Who calls this function?'",
            "code_search calls-to -m <MODULE> -f <FUNCTION> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all callers of MyApp.Repo.get/2", "code_search calls-to -m MyApp.Repo -f get -a 2"),
            Example::new("Find callers of any function named 'validate'", "code_search calls-to -m \"MyApp\\..*\" -f validate -r"),
        ])
        .with_related(vec!["calls-from", "trace", "path"]),

        CommandDescription::new(
            "calls-from",
            "Find what a function calls",
            CommandCategory::Query,
            "Finds all functions that are called by a specific function. Use this to answer: 'What does this function call?'",
            "code_search calls-from -m <MODULE> -f <FUNCTION> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all functions called by MyApp.Repo.get/2", "code_search calls-from -m MyApp.Repo -f get -a 2"),
            Example::new("Find what a module calls", "code_search calls-from -m MyApp.Accounts"),
        ])
        .with_related(vec!["calls-to", "trace", "path"]),

        CommandDescription::new(
            "trace",
            "Forward call trace from a function",
            CommandCategory::Query,
            "Traces call chains forward from a starting function. Shows the full path of calls that can be reached from a given function.",
            "code_search trace -m <MODULE> -f <FUNCTION> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Trace all calls from a function", "code_search trace -m MyApp.API -f create_user"),
            Example::new("Limit trace depth to 3 levels", "code_search trace -m MyApp.API -f create_user -d 3"),
        ])
        .with_related(vec!["calls-from", "reverse-trace", "path"]),

        CommandDescription::new(
            "reverse-trace",
            "Backward call trace to a function",
            CommandCategory::Query,
            "Traces call chains backward to a target function. Shows all code paths that can lead to a given function.",
            "code_search reverse-trace -m <MODULE> -f <FUNCTION> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all paths leading to a function", "code_search reverse-trace -m MyApp.API -f validate_token"),
            Example::new("Limit trace depth to 2 levels", "code_search reverse-trace -m MyApp.API -f validate_token -d 2"),
        ])
        .with_related(vec!["calls-to", "trace", "path"]),

        CommandDescription::new(
            "path",
            "Find a call path between two functions",
            CommandCategory::Query,
            "Finds one or more call paths connecting two functions. Useful for understanding how code flows from a source to a target.",
            "code_search path -m <MODULE> -f <FUNCTION> --target-module <MODULE> --target-function <FUNCTION>",
        )
        .with_examples(vec![
            Example::new("Find call path between two functions", "code_search path -m MyApp.API -f create_user --target-module MyApp.DB -f insert"),
        ])
        .with_related(vec!["trace", "reverse-trace", "calls-from"]),

        // Analysis Commands
        CommandDescription::new(
            "hotspots",
            "Find high-connectivity functions and modules",
            CommandCategory::Analysis,
            "Identifies functions and modules with the most incoming or outgoing calls. These are high-impact areas that affect many other parts of the codebase.",
            "code_search hotspots [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all hotspots", "code_search hotspots"),
            Example::new("Find top 20 hotspots", "code_search hotspots --limit 20"),
        ])
        .with_related(vec!["god-modules", "boundaries", "complexity"]),

        CommandDescription::new(
            "unused",
            "Find functions that are never called",
            CommandCategory::Analysis,
            "Identifies functions that have no incoming calls. These may be dead code, internal helpers, or entry points.",
            "code_search unused [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all unused functions", "code_search unused"),
            Example::new("Find unused functions in a module", "code_search unused -m MyApp.Utils"),
        ])
        .with_related(vec!["hotspots", "duplicates", "large-functions"]),

        CommandDescription::new(
            "god-modules",
            "Find god modules - modules with high function count and connectivity",
            CommandCategory::Analysis,
            "Identifies modules that are overly large or have too much responsibility. These are candidates for refactoring.",
            "code_search god-modules [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all god modules", "code_search god-modules"),
            Example::new("Show top 10 god modules", "code_search god-modules --limit 10"),
        ])
        .with_related(vec!["hotspots", "boundaries", "complexity"]),

        CommandDescription::new(
            "boundaries",
            "Find boundary modules with high fan-in but low fan-out",
            CommandCategory::Analysis,
            "Identifies modules that many others depend on but have few dependencies. These are key integration points.",
            "code_search boundaries [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all boundary modules", "code_search boundaries"),
        ])
        .with_related(vec!["god-modules", "hotspots", "depends-on"]),

        CommandDescription::new(
            "duplicates",
            "Find functions with identical or near-identical implementations",
            CommandCategory::Analysis,
            "Identifies duplicate code that could be consolidated into reusable functions.",
            "code_search duplicates [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all duplicate functions", "code_search duplicates"),
            Example::new("Find duplicates in a module", "code_search duplicates -m MyApp.Utils"),
        ])
        .with_related(vec!["duplicate-hotspots", "unused", "large-functions"]),

        CommandDescription::new(
            "duplicate-hotspots",
            "Find modules with the most duplicated functions",
            CommandCategory::Analysis,
            "Identifies modules that have high levels of code duplication, indicating refactoring opportunities.",
            "code_search duplicate-hotspots [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find modules with most duplicates", "code_search duplicate-hotspots"),
        ])
        .with_related(vec!["duplicates", "hotspots", "large-functions"]),

        CommandDescription::new(
            "complexity",
            "Display complexity metrics for functions",
            CommandCategory::Analysis,
            "Shows complexity metrics like number of clauses, arguments, and lines for each function.",
            "code_search complexity [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Show complexity for all functions", "code_search complexity"),
            Example::new("Show top 50 most complex functions", "code_search complexity --limit 50"),
        ])
        .with_related(vec!["large-functions", "many-clauses", "hotspots"]),

        CommandDescription::new(
            "large-functions",
            "Find large functions that may need refactoring",
            CommandCategory::Analysis,
            "Identifies functions that are large by line count, suggesting they may need to be broken down.",
            "code_search large-functions [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all large functions", "code_search large-functions"),
            Example::new("Show top 30 largest functions", "code_search large-functions --limit 30"),
        ])
        .with_related(vec!["complexity", "many-clauses", "hotspots"]),

        CommandDescription::new(
            "many-clauses",
            "Find functions with many pattern-matched heads",
            CommandCategory::Analysis,
            "Identifies functions that have many clauses/definitions, suggesting they may be doing too much.",
            "code_search many-clauses [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find functions with many clauses", "code_search many-clauses"),
        ])
        .with_related(vec!["complexity", "large-functions", "hotspots"]),

        CommandDescription::new(
            "cycles",
            "Detect circular dependencies between modules",
            CommandCategory::Analysis,
            "Finds circular dependencies in the module dependency graph, which indicate architectural issues.",
            "code_search cycles [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all circular dependencies", "code_search cycles"),
        ])
        .with_related(vec!["depends-on", "depended-by", "boundaries"]),

        // Search Commands
        CommandDescription::new(
            "search",
            "Search for modules or functions by name pattern",
            CommandCategory::Search,
            "Finds modules or functions matching a given pattern. Use this as a starting point for other analyses.",
            "code_search search -p <PATTERN> -k [modules|functions] [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find modules containing 'User'", "code_search search -p User"),
            Example::new("Find functions starting with 'get_'", "code_search search -p get_ -k functions"),
            Example::new("Use regex pattern", "code_search search -p '^MyApp\\.API' -r"),
        ])
        .with_related(vec!["location", "function", "browse-module"]),

        CommandDescription::new(
            "location",
            "Find where a function is defined",
            CommandCategory::Search,
            "Shows the file path and line numbers where a function is defined. Useful for quickly navigating to code.",
            "code_search location -m <MODULE> -f <FUNCTION> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find location of a function", "code_search location -m MyApp.Repo -f get"),
            Example::new("Find any function named 'validate'", "code_search location -f validate"),
        ])
        .with_related(vec!["search", "function", "browse-module"]),

        CommandDescription::new(
            "function",
            "Show function signature",
            CommandCategory::Search,
            "Displays the full signature of a function including arguments, return type, and metadata.",
            "code_search function -m <MODULE> -f <FUNCTION> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Show function signature", "code_search function -m MyApp.Repo -f get -a 2"),
        ])
        .with_related(vec!["search", "location", "accepts"]),

        CommandDescription::new(
            "browse-module",
            "Browse all definitions in a module or file",
            CommandCategory::Module,
            "Lists all functions, structs, and other definitions in a module. Great for exploring unfamiliar code.",
            "code_search browse-module -m <MODULE> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Browse a module", "code_search browse-module -m MyApp.Accounts"),
            Example::new("Browse with limit", "code_search browse-module -m MyApp.Accounts --limit 50"),
        ])
        .with_related(vec!["search", "location", "struct-modules"]),

        // Type Search Commands
        CommandDescription::new(
            "accepts",
            "Find functions accepting a specific type pattern",
            CommandCategory::Type,
            "Finds all functions that have a parameter matching a type pattern. Useful for finding consumers of a type.",
            "code_search accepts -p <PATTERN> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find functions accepting a type", "code_search accepts -p User"),
            Example::new("Use regex for type pattern", "code_search accepts -p 'Result\\(.*\\)' -r"),
        ])
        .with_related(vec!["returns", "struct-usage", "function"]),

        CommandDescription::new(
            "returns",
            "Find functions returning a specific type pattern",
            CommandCategory::Type,
            "Finds all functions that return a type matching a pattern. Useful for finding providers of a type.",
            "code_search returns -p <PATTERN> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find functions returning a type", "code_search returns -p ':ok"),
            Example::new("Use regex for type pattern", "code_search returns -p 'Ok\\(.*\\)' -r"),
        ])
        .with_related(vec!["accepts", "struct-usage", "function"]),

        CommandDescription::new(
            "struct-usage",
            "Find functions that work with a given struct type",
            CommandCategory::Type,
            "Finds all functions that accept or return a specific struct type. Shows all code that interacts with a type.",
            "code_search struct-usage -p <PATTERN> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find all functions using a struct", "code_search struct-usage -p User"),
        ])
        .with_related(vec!["accepts", "returns", "struct-modules"]),

        CommandDescription::new(
            "struct-modules",
            "Show which modules work with a given struct type",
            CommandCategory::Type,
            "Lists all modules that have functions accepting or returning a specific struct. Shows module responsibilities.",
            "code_search struct-modules -p <PATTERN> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find modules working with a struct", "code_search struct-modules -p User"),
        ])
        .with_related(vec!["struct-usage", "browse-module", "hotspots"]),

        // Module Commands
        CommandDescription::new(
            "depends-on",
            "Show what modules a given module depends on",
            CommandCategory::Module,
            "Lists all modules that a given module calls or depends on. Shows outgoing module dependencies.",
            "code_search depends-on -m <MODULE> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find module dependencies", "code_search depends-on -m MyApp.API"),
        ])
        .with_related(vec!["depended-by", "cycles", "boundaries"]),

        CommandDescription::new(
            "depended-by",
            "Show what modules depend on a given module",
            CommandCategory::Module,
            "Lists all modules that call or depend on a given module. Shows incoming module dependencies.",
            "code_search depended-by -m <MODULE> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Find modules that depend on this one", "code_search depended-by -m MyApp.Repo"),
        ])
        .with_related(vec!["depends-on", "cycles", "boundaries"]),

        CommandDescription::new(
            "clusters",
            "Analyze module connectivity using namespace-based clustering",
            CommandCategory::Module,
            "Groups modules into clusters based on their namespace structure and interdependencies.\n\n\
             Output columns:\n\
             - Internal: calls between modules within the same namespace\n\
             - Out: calls from this namespace to other namespaces\n\
             - In: calls from other namespaces into this one\n\
             - Cohesion: internal / (internal + out + in) — higher = more self-contained\n\
             - Instab: out / (in + out) — 0 = stable (depended upon), 1 = unstable (depends on others)",
            "code_search clusters [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Show module clusters", "code_search clusters"),
        ])
        .with_related(vec!["god-modules", "boundaries", "depends-on"]),

        // Other Commands
        CommandDescription::new(
            "setup",
            "Create database schema without importing data",
            CommandCategory::Other,
            "Initializes a new database with the required schema for storing call graph data.",
            "code_search setup [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Create database schema", "code_search setup --db ./my_project.db"),
            Example::new("Force recreate", "code_search setup --db ./my_project.db --force"),
        ])
        .with_related(vec!["import"]),

        CommandDescription::new(
            "import",
            "Import a call graph JSON file into the database",
            CommandCategory::Other,
            "Loads call graph data from a JSON file into the database. Must run setup first.",
            "code_search import --file <FILE> [OPTIONS]",
        )
        .with_examples(vec![
            Example::new("Import call graph data", "code_search import --file call_graph.json"),
        ])
        .with_related(vec!["setup"]),
    ]
}

/// Get a single command description by name
pub fn get_description(name: &str) -> Option<CommandDescription> {
    all_descriptions().into_iter().find(|d| d.name == name)
}

/// Get all descriptions grouped by category
pub fn descriptions_by_category() -> std::collections::BTreeMap<CommandCategory, Vec<(String, String)>> {
    let mut map = std::collections::BTreeMap::new();

    for desc in all_descriptions() {
        map.entry(desc.category)
            .or_insert_with(Vec::new)
            .push((desc.name.clone(), desc.brief.clone()));
    }

    map
}
