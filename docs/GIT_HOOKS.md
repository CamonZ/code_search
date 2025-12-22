# Git Hooks for Incremental Database Updates

This guide explains how to use git hooks to automatically keep your code graph database up to date with each commit.

## Overview

The post-commit git hook automatically:
1. Compiles your Elixir project with debug info (if needed)
2. Extracts AST data for files changed in the last commit using `ex_ast --git-diff`
3. Updates the CozoDB database with the new data (using upsert to update existing records)

This provides incremental updates without the need to re-analyze your entire codebase after each change.

## Prerequisites

- [ex_ast](https://github.com/CamonZ/ex_ast) installed and available in your PATH
- `code_search` binary installed and available in your PATH
- An Elixir project with a `mix.exs` file
- Git repository

## Installation

### Basic Installation

To install the post-commit hook:

```bash
code_search setup --install-hooks
```

This will:
- Install the `post-commit` hook to `.git/hooks/`
- Configure git settings:
  - `code-search.mix-env`: `dev` (Mix environment to use)

**That's it!** The database path is automatically resolved to `.code_search/cozo.sqlite` in your project root.

### Complete Setup (Skills + Hooks)

To set up everything in one command:

```bash
code_search setup --install-skills --install-hooks
```

This will:
- Create the database schema at `.code_search/cozo.sqlite`
- Install Claude Code skills to `.claude/skills/`
- Install Claude Code agents to `.claude/agents/`
- Install the post-commit hook to `.git/hooks/`

### Optional Configuration

The hook works out of the box without any configuration. However, you can optionally customize:

#### Multi-Project Databases

If you want to namespace multiple projects in the same database:

```bash
code_search setup --install-hooks --project-name my_app
```

Or configure later:

```bash
git config code-search.project-name my_app
```

#### Mix Environment

To use a different Mix environment (default: `dev`):

```bash
git config code-search.mix-env test
```

### View Configuration

```bash
git config --get-regexp code-search
```

## How It Works

When you make a commit, the post-commit hook:

1. **Checks prerequisites**: Verifies that `mix.exs`, `ex_ast`, and `code_search` are available

2. **Compiles the project**: Runs `mix compile --debug-info` to ensure BEAM files exist
   - If compilation fails, the hook exits with an error message
   - The commit still succeeds, but the database is not updated

3. **Extracts changes**: Runs `ex_ast --git-diff HEAD~1` to extract AST data for files changed in the last commit
   - Uses the configured Mix environment
   - Outputs JSON to a temporary file

4. **Updates database**: Runs `code_search import` to update the database
   - Database path auto-resolves to `.code_search/cozo.sqlite`
   - Uses configured project name if set (optional)
   - Performs upsert operations (updates existing records, inserts new ones)

## Database Update Strategy

The import process uses **upsert** semantics:

- **Modules**: Keyed by `(project, name)` - updates existing modules
- **Functions**: Keyed by `(project, module, name, arity)` - updates existing functions
- **Calls**: Keyed by `(project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, column)` - updates existing calls
- **Struct Fields**: Keyed by `(project, module, field)` - updates existing fields
- **Function Locations**: Keyed by `(project, module, name, arity, line)` - updates existing locations
- **Specs**: Keyed by `(project, module, name, arity)` - updates existing specs
- **Types**: Keyed by `(project, module, name)` - updates existing types

This means:
- Modified functions get their data updated
- Deleted functions remain in the database (intentional - preserves history)
- New functions are added
- If you need to fully rebuild, use `code_search setup --force` and re-import

## Error Handling

The hook handles errors gracefully:

### Compilation Errors

If your code doesn't compile, you'll see:
```
[code-search] Compilation failed - changed files in the last commit don't compile
[code-search] Database update skipped
```

The commit succeeds, but the database update is skipped.

### Missing Dependencies

If `ex_ast` or `code_search` are not found:
```
[code-search] ex_ast not found in PATH, skipping database update
[code-search] Install from: https://github.com/CamonZ/ex_ast
```

The hook exits gracefully without updating the database.

### Non-Elixir Projects

If the hook runs in a non-Elixir project (no `mix.exs`):
```
[code-search] No mix.exs found, skipping database update
```

## Disabling the Hook

### Temporarily (for one commit)

Use the `--no-verify` flag:

```bash
git commit --no-verify -m "Skip hook for this commit"
```

### Permanently

Remove the hook file:

```bash
rm .git/hooks/post-commit
```

Or make it non-executable:

```bash
chmod -x .git/hooks/post-commit
```

### For specific operations

Git hooks don't run for:
- `git rebase`
- `git merge` (uses different hooks)
- `git cherry-pick`
- Operations that don't create new commits

## Troubleshooting

### Hook not running

Check if the hook is executable:
```bash
ls -la .git/hooks/post-commit
```

Should show: `-rwxr-xr-x` (executable permission)

Make it executable if needed:
```bash
chmod +x .git/hooks/post-commit
```

### Database not updating

1. Check that ex_ast is working:
```bash
ex_ast --git-diff HEAD~1 --format json
```

2. Check git configuration:
```bash
git config --get-regexp code-search
```

3. Check database exists:
```bash
ls -la .code_search/cozo.sqlite
```

### Slow commits

The hook adds some time to each commit:
- Compilation: Usually cached, fast after first compile
- AST extraction: ~1-2 seconds for typical changes
- Database import: ~100ms for typical changes

If this is problematic, consider:
- Disabling the hook temporarily during rapid development
- Using `--no-verify` for quick commits
- Running batch updates manually instead

## Manual Updates

If you prefer manual control, you can run the hook logic manually:

```bash
# Compile
mix compile --debug-info

# Extract changes from last commit
ex_ast --git-diff HEAD~1 --format json --output changes.json

# Import
code_search --db call_graph.db import --file changes.json --project my_app
```

Or for a different git reference:

```bash
# Compare against a specific commit
ex_ast --git-diff abc123 --format json --output changes.json

# Compare staged changes (before commit)
ex_ast --git-diff --staged --format json --output changes.json
```

## Integration with CI/CD

The same approach works in CI/CD pipelines. Example GitHub Actions workflow:

```yaml
- name: Update code graph
  run: |
    mix compile --debug-info
    ex_ast --git-diff HEAD~1 --format json --output changes.json
    code_search --db call_graph.db import --file changes.json --project ${{ github.repository }}
```

## Performance Characteristics

Typical performance for a commit changing 5 files with 20 functions:

- Compilation: <1s (cached) to 10s (clean)
- AST extraction: 1-2s
- Database import: 100-500ms
- **Total overhead**: 2-12s per commit

The hook is designed to be fast for incremental updates. Full project analysis would take much longer.

## See Also

- [ex_ast documentation](https://github.com/CamonZ/ex_ast)
- [Git hooks documentation](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks)
- [CozoDB documentation](https://docs.cozodb.org/)
