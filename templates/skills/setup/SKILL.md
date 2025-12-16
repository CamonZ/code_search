---
name: setup
description: Create database schema for storing call graph data. Use this to initialize the database with required tables before importing data or when setting up a new project.
---

# setup

Create database schema for storing call graph data.

## Purpose

Initialize the database with the required tables and schema for storing call graph data. Use this before importing data or when setting up a new project database.

## Usage

```bash
code_search --format toon setup [OPTIONS]
```

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `--force` | Drop existing schema and recreate | false |
| `--dry-run` | Show what would be created | false |

## Output Fields (toon format)

```
created_new: true
dry_run: false
relations[N]{name,status}:
  modules,created
  functions,created
  calls,created
```

## When to Use

- Setting up a new database for the first time
- After clearing data and needing to recreate schema
- When database schema seems corrupted
- Before importing call graph data

## See Also

- [examples.md](examples.md) for detailed usage examples
- `import` - Import call graph data after setup
