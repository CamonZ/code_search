# Workflow: Understand a Feature

Build a complete mental model of a feature by exploring it vertically through all layers of the application.

## When to Use

- Onboarding: "How does the payments feature work?"
- Before changes: "I need to understand billing before modifying it"
- Code review: "What does this feature touch?"
- Documentation: "Map out the authentication system"

## Step-by-Step Process

### 1. Find the Feature's Modules

Start by searching for related modules:
```bash
# Search by feature name
code_search --format toon search Payment
code_search --format toon search Billing
code_search --format toon search Subscription

# Search with regex for related patterns
code_search --format toon search '^MyApp\.Payments' --regex
```

### 2. Identify the Core Module

Usually there's a central module that orchestrates the feature:
```bash
# Browse candidate modules
code_search --format toon browse-module MyApp.Payments
code_search --format toon browse-module MyApp.Billing

# Check which one has more connections (likely the core)
code_search --format toon depended-by MyApp.Payments
code_search --format toon depended-by MyApp.Billing
```

The module with more dependents is often the public API of the feature.

### 3. Map the Module's Interface

Understand what the core module provides:
```bash
# See all public functions
code_search --format toon browse-module MyApp.Payments --kind functions

# See type definitions
code_search --format toon browse-module MyApp.Payments --kind types

# See struct definitions
code_search --format toon browse-module MyApp.Payments --kind structs
```

### 4. Explore Upward (Who Uses This Feature?)

Find the entry points and consumers:
```bash
# What modules depend on this feature?
code_search --format toon depended-by MyApp.Payments

# Find specific callers of key functions
code_search --format toon calls-to MyApp.Payments process_payment

# Trace backwards to find entry points
code_search --format toon reverse-trace MyApp.Payments process_payment --depth 5
```

This reveals:
- Controllers that expose this feature
- Other features that integrate with it
- Background jobs that use it

### 5. Explore Downward (What Does This Feature Use?)

Find the dependencies:
```bash
# What does this module depend on?
code_search --format toon depends-on MyApp.Payments

# Trace forward from key functions
code_search --format toon trace MyApp.Payments process_payment --depth 5
```

This reveals:
- Database access patterns (Repo calls)
- External service integrations
- Shared utilities used

### 6. Map the Data Flow

Understand what data types flow through the feature:
```bash
# What functions accept the main struct?
code_search --format toon struct-usage Payment.t

# Or search by type pattern
code_search --format toon accepts Payment
code_search --format toon returns Payment
```

### 7. Check for Related Background Processing

Features often have async components:
```bash
# Find related workers/jobs
code_search --format toon search PaymentWorker
code_search --format toon search 'Payment.*Job' --regex

# Find GenServer patterns
code_search --format toon search PaymentServer
```

## Example: Understanding the Authentication Feature

```bash
# 1. Find auth-related modules
code_search --format toon search Auth
# Found: MyApp.Auth, MyApp.Auth.Guardian, MyApp.Auth.Pipeline

# 2. Browse the main module
code_search --format toon browse-module MyApp.Auth
# Shows: authenticate/2, register/1, verify_token/1, etc.

# 3. Who uses auth?
code_search --format toon depended-by MyApp.Auth
# Found: SessionController, ApiController, all protected controllers

# 4. What does auth depend on?
code_search --format toon depends-on MyApp.Auth
# Found: Repo, Guardian, Comeonin

# 5. Trace a key flow
code_search --format toon trace MyApp.Auth authenticate --depth 5
# Shows: authenticate → verify_password → Comeonin.check_pass
#                     → load_user → Repo.get_by
#                     → create_token → Guardian.encode_and_sign

# 6. Map the user data flow
code_search --format toon struct-usage User.t MyApp.Auth
```

## Building the Mental Model

After this exploration, you should know:

| Aspect | What You've Learned |
|--------|---------------------|
| **Interface** | Public functions, types, structs |
| **Consumers** | Controllers, other features, jobs |
| **Dependencies** | Database, external services, utilities |
| **Data Flow** | What types go in and out |
| **Architecture** | How it fits in the system |

## Tips

- **Start broad, then focus**: Search first, then browse specific modules
- **Follow the money**: The most-connected module is usually the core
- **Check both directions**: Upward (who uses) and downward (what's used)
- **Don't forget async**: Background jobs are often part of features
- **Map the types**: Data flow reveals the real architecture

## Related Commands

| Command | Use For |
|---------|---------|
| `search` | Find modules/functions by name |
| `browse-module` | See everything in a module |
| `depends-on` | Outgoing module dependencies |
| `depended-by` | Incoming module dependencies |
| `trace` | Forward call chain |
| `reverse-trace` | Backward call chain |
| `struct-usage` | Find type usage |
| `clusters` | Find related module groups |

## See Also

- [trace-execution-flow.md](trace-execution-flow.md) - Detailed execution tracing
- [impact-analysis.md](impact-analysis.md) - Before making changes
