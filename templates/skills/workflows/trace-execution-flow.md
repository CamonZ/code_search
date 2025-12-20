# Workflow: Trace Execution Flow

Understand how code executes by tracing call chains from entry points through the application.

## When to Use

- Debugging: "How does a request flow through the system?"
- Understanding: "What happens when this function is called?"
- Documentation: "What's the execution path for this feature?"

## Step-by-Step Process

### 1. Identify the Entry Point

Entry points are typically:
- Phoenix controller actions
- GenServer callbacks (`handle_call`, `handle_cast`, `handle_info`)
- Plug pipelines
- Mix tasks
- Public API functions

Find potential entry points:
```bash
# Find controller actions
code_search --format toon search Controller --kind modules

# Browse a controller to find actions
code_search --format toon browse-module MyApp.UserController

# Find GenServer callbacks
code_search --format toon location handle_call
```

### 2. Trace Forward from Entry Point

Use `trace` to follow the call chain:
```bash
# Trace 5 levels deep (default)
code_search --format toon trace MyApp.UserController create

# Trace deeper for complex flows
code_search --format toon trace MyApp.UserController create --depth 10
```

**Reading the output:**
- `depth: 1` = direct calls from the starting function
- `depth: 2` = calls from those callees
- Each entry shows: caller → callee with file:line

### 3. Explore Specific Branches

When you see an interesting callee, dive deeper:
```bash
# What does this specific function call?
code_search --format toon calls-from MyApp.Accounts create_user

# Continue tracing from there
code_search --format toon trace MyApp.Accounts create_user --depth 5
```

### 4. Find the Path Between Two Points

If you know the start and end:
```bash
code_search --format toon path \
  --from-module MyApp.UserController --from-function create \
  --to-module MyApp.Repo --to-function insert
```

This shows exactly how the controller action reaches the database.

### 5. Understand the Reverse Flow

Sometimes it's useful to trace backwards:
```bash
# Who calls this low-level function?
code_search --format toon reverse-trace MyApp.Repo insert --depth 3

# Find all callers (single level)
code_search --format toon calls-to MyApp.Repo insert
```

## Example: Tracing a User Registration Flow

```bash
# 1. Find the registration controller
code_search --format toon search Registration

# 2. Browse the controller
code_search --format toon browse-module MyApp.RegistrationController

# 3. Trace from the create action
code_search --format toon trace MyApp.RegistrationController create --depth 8

# 4. Output reveals the flow:
#    Controller.create
#    └── Accounts.register_user
#        ├── Accounts.validate_registration
#        ├── Repo.insert
#        └── Email.send_welcome
#            └── Mailer.deliver
```

## Tips

- **Start shallow, go deep**: Begin with `--depth 3`, increase if needed
- **Follow the data**: Combine with `accepts`/`returns` to trace data types
- **Check multiple paths**: Different inputs may take different code paths
- **Watch for recursion**: Some functions call themselves (GenServer loops)

## Related Commands

| Command | Use For |
|---------|---------|
| `trace` | Forward traversal from a starting point |
| `reverse-trace` | Backward traversal to a target |
| `calls-from` | Single-level forward (what does X call?) |
| `calls-to` | Single-level backward (what calls X?) |
| `path` | Find route between two functions |
| `location` | Find where a function is defined |

## See Also

- [understand-feature.md](understand-feature.md) - Broader feature analysis
- [impact-analysis.md](impact-analysis.md) - Finding what's affected by changes
