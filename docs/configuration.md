# Configuration Guide

This guide explains how to configure the code_search tool using the `.code_search.json` configuration file.

## Overview

The code_search tool uses a JSON configuration file named `.code_search.json` to specify database settings. This configuration file is **required** and must be present in the directory where you run code_search commands.

## Configuration File Location

Place `.code_search.json` in your project root or the directory where you plan to run code_search commands. The tool will look for this file in the current working directory when you execute commands.

### Example Directory Structure

```
my_project/
├── .code_search.json      # Configuration file (required)
├── src/                   # Your source code
├── cozo.sqlite            # Database file (created by tool, SQLite only)
└── call_graph.json        # Imported call graph data
```

## Configuration File Format

All configuration files follow this structure:

```json
{
  "database": {
    "type": "<backend_type>",
    ...additional options based on type
  }
}
```

The `type` field determines which database backend to use. Currently supported backends are:
- `sqlite` - Embedded SQLite database (recommended for local development)
- `memory` - In-memory database (testing and ephemeral use)
- `postgres` - PostgreSQL database (not yet implemented)

## Database Backend Options

### SQLite (Embedded Database)

SQLite is the recommended option for local development and testing. The database is stored as a single file on your filesystem.

**When to use:**
- Local development
- Single-user analysis
- Small to medium codebases
- Portable projects (database travels with your code)

**Configuration:**

```json
{
  "database": {
    "type": "sqlite",
    "path": "./cozo.sqlite"
  }
}
```

**Fields:**
- `type` (required): Must be `"sqlite"`
- `path` (required): File system path to the SQLite database file
  - Use relative paths for portability: `./cozo.sqlite`
  - Or absolute paths: `/var/lib/code_search/myproject.sqlite`
  - The file will be created if it doesn't exist

**Example - Local Development:**

```json
{
  "database": {
    "type": "sqlite",
    "path": "./cozo.sqlite"
  }
}
```

**Example - Shared Database Location:**

```json
{
  "database": {
    "type": "sqlite",
    "path": "/var/lib/code_search/myapp.sqlite"
  }
}
```

### In-Memory Database (Testing)

The in-memory database stores all data in RAM. It's useful for testing and ephemeral analysis sessions, but data is lost when the process exits.

**When to use:**
- Unit tests
- CI/CD pipelines
- Quick analysis that doesn't need to persist
- Performance testing

**Configuration:**

```json
{
  "database": {
    "type": "memory"
  }
}
```

**Fields:**
- `type` (required): Must be `"memory"`
- No additional fields required

**Example - Test Configuration:**

```json
{
  "database": {
    "type": "memory"
  }
}
```

### PostgreSQL with Connection String

PostgreSQL support is not yet implemented but will be available in a future release. This section documents the planned interface.

**Planned Configuration:**

```json
{
  "database": {
    "type": "postgres",
    "connection_string": "postgres://user:password@localhost:5432/mydb"
  }
}
```

**Connection String Format:**

```
postgres://[user[:password]@][host][:port][/dbname][?param1=value1&...]
```

**Examples:**

```
postgres://localhost/mydb
postgres://user@localhost/mydb
postgres://user:password@localhost:5432/mydb
postgres://user:password@db.example.com:5432/mydb?sslmode=require
```

### PostgreSQL with Individual Options

PostgreSQL support is not yet implemented but will be available in a future release. This section documents the planned interface.

**Planned Configuration:**

```json
{
  "database": {
    "type": "postgres",
    "host": "localhost",
    "port": 5432,
    "user": "myuser",
    "password": "mypassword",
    "database": "mydb",
    "ssl": false,
    "graph_name": "call_graph"
  }
}
```

**Fields:**

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `type` | Yes | N/A | Must be `"postgres"` |
| `host` | Yes | N/A | PostgreSQL server hostname |
| `port` | No | 5432 | PostgreSQL server port |
| `user` | Yes | N/A | PostgreSQL username |
| `password` | No | None | PostgreSQL password |
| `database` | Yes | N/A | Database name |
| `ssl` | No | false | Enable SSL/TLS connection |
| `graph_name` | No | "call_graph" | Name of the call graph table |

**Example - Development Database:**

```json
{
  "database": {
    "type": "postgres",
    "host": "localhost",
    "user": "analyst",
    "database": "code_analysis"
  }
}
```

**Example - Production Database with SSL:**

```json
{
  "database": {
    "type": "postgres",
    "host": "db.production.internal",
    "port": 5432,
    "user": "service_account",
    "password": "secure_password",
    "database": "call_graphs",
    "ssl": true,
    "graph_name": "prod_call_graph"
  }
}
```

## Configuration Resolution

The tool uses the following priority order to find configuration:

1. **Config File** (highest priority)
   - Looks for `.code_search.json` in the current directory
   - If found, uses this configuration and stops

2. **Environment Variables** (second priority)
   - `DATABASE_URL` - Connection string format
   - `COZO_PATH` - Path to SQLite database
   - Checked only if config file is not present

3. **Default** (lowest priority)
   - Falls back to `./cozo.sqlite` if neither config file nor env vars are found

Example resolution flow:

```
Does .code_search.json exist in current directory?
  ├─ YES → Use it (stop here)
  └─ NO → Check DATABASE_URL environment variable?
          ├─ YES → Use it (stop here)
          └─ NO → Check COZO_PATH environment variable?
                  ├─ YES → Use it (stop here)
                  └─ NO → Use default ./cozo.sqlite
```

## Security Considerations

### Protecting Credentials

If your configuration contains database credentials (especially for PostgreSQL), follow these security practices:

**1. Use .gitignore to prevent accidental commits:**

Add to your `.gitignore`:

```
.code_search.json
```

This ensures the config file with credentials is never committed to version control.

**2. Use environment-specific config files:**

For different environments, use environment-specific naming:

```
.code_search.local.json      # Local development (in .gitignore)
.code_search.staging.json    # Staging (in .gitignore)
.code_search.prod.json       # Production (in .gitignore)
```

Then load the appropriate file programmatically or document which file to use.

**3. Restrict file permissions (Unix/Linux/macOS):**

After creating a config file with credentials:

```bash
chmod 600 .code_search.json
```

This restricts access to only the file owner:

```bash
-rw------- 1 user group 256 Jan 1 12:00 .code_search.json
```

**4. Use environment variables for sensitive data:**

Instead of hardcoding passwords, use environment variables in your shell:

```bash
export PGPASSWORD="secure_password"
export DATABASE_URL="postgres://user@localhost/mydb"
code_search search "function_name"
```

### Best Practices

1. **Never commit `.code_search.json` with credentials** to version control
2. **Use `.gitignore`** to ensure the file is not accidentally committed
3. **Provide `.code_search.example.json`** with safe default values for developers
4. **Use separate configs** for development, staging, and production
5. **Restrict file permissions** to owner-only (chmod 600)
6. **Rotate database passwords** regularly if credentials are in the config
7. **Consider managed secrets** for CI/CD pipelines (use environment variables instead)

### .gitignore Configuration

Your `.gitignore` should include:

```
# Configuration files with credentials
.code_search.json
.code_search.*.json

# Database files
*.sqlite
*.sqlite3

# Environment files
.env
.env.local
```

## Validation and Error Messages

### Common Configuration Errors

**Error: "Configuration file not found: .code_search.json"**

- **Cause:** The `.code_search.json` file is not in the current directory
- **Solution:** Create the file using `.code_search.example.json` as a template:
  ```bash
  cp .code_search.example.json .code_search.json
  ```

**Error: "Invalid JSON in .code_search.json"**

- **Cause:** The JSON syntax is invalid
- **Solution:** Use a JSON validator:
  ```bash
  cat .code_search.json | python -m json.tool
  ```

**Error: "SQLite database at ... failed to open"**

- **Cause:** The file path is invalid or inaccessible
- **Solution:**
  - Verify the path is correct
  - Check that parent directories exist
  - Ensure you have write permissions

**Error: "PostgreSQL backend not yet implemented"**

- **Cause:** You've configured PostgreSQL which is not yet supported
- **Solution:** Use SQLite or in-memory database for now:
  ```json
  {
    "database": {
      "type": "sqlite",
      "path": "./cozo.sqlite"
    }
  }
  ```

## Migration from Old `--db` Flag System

Previous versions of code_search used a `--db` command-line flag. This has been replaced with the configuration file system.

### Before (Old System)

```bash
code_search --db ./mydb.sqlite search "function_name"
code_search --db ":memory:" import -f call_graph.json
```

### After (New System)

Create `.code_search.json`:

```json
{
  "database": {
    "type": "sqlite",
    "path": "./mydb.sqlite"
  }
}
```

Then run commands without the `--db` flag:

```bash
code_search search "function_name"
code_search import -f call_graph.json
```

### Why Configuration Files?

The configuration file approach provides several advantages:

1. **Consistency** - Same configuration across all commands
2. **Security** - Credentials stay in a single, gitignore-able file
3. **Reproducibility** - Configuration is documented and version-controlled
4. **Flexibility** - Easy to switch between development/staging/production
5. **Clarity** - More explicit than command-line flags for complex configs

## Complete Examples

### Example 1: Local SQLite Development Setup

Directory structure:

```
project/
├── .code_search.json
├── src/
└── cozo.sqlite (created automatically)
```

Configuration:

```json
{
  "database": {
    "type": "sqlite",
    "path": "./cozo.sqlite"
  }
}
```

Usage:

```bash
# Import call graph
code_search import -f call_graph.json --project my_app

# Search for functions
code_search search "Controller"

# Find specific function
code_search location -m MyApp.Web.UserController -f show
```

### Example 2: In-Memory Database for Testing

Configuration:

```json
{
  "database": {
    "type": "memory"
  }
}
```

Usage in test script:

```bash
#!/bin/bash
# test.sh - Clean database for each test run

code_search import -f fixtures/small_call_graph.json --project test_app
code_search search "test" --format json
```

### Example 3: Multiple Environments

Create separate configuration files:

`.code_search.local.json` (in .gitignore):

```json
{
  "database": {
    "type": "sqlite",
    "path": "./local.sqlite"
  }
}
```

`.code_search.example.json` (in git):

```json
{
  "database": {
    "type": "sqlite",
    "path": "./cozo.sqlite"
  }
}
```

Usage:

```bash
# Use default
cp .code_search.example.json .code_search.json
code_search search "function"

# Use local override
cp .code_search.local.json .code_search.json
code_search search "function"
```

### Example 4: Shared Database Location

For team analysis where database is shared:

```json
{
  "database": {
    "type": "sqlite",
    "path": "/mnt/shared/code_analysis/main_project.sqlite"
  }
}
```

## Troubleshooting

### Configuration File Not Found

```
Error: Configuration file not found: .code_search.json
```

**Check:**

```bash
# Verify current directory
pwd

# Check if file exists
ls -la .code_search.json

# Check file content
cat .code_search.json
```

**Solution:**

```bash
# Copy example file
cp .code_search.example.json .code_search.json

# Edit for your setup
nano .code_search.json
```

### Database File Not Found

```
Error: Failed to open SQLite database at ./cozo.sqlite
```

**Cause:** The path is incorrect or parent directory doesn't exist

**Check:**

```bash
# Verify path in config
grep path .code_search.json

# Check parent directory exists
mkdir -p $(dirname ./cozo.sqlite)

# Test database creation
sqlite3 ./cozo.sqlite "SELECT 1;"
```

### Permission Denied

```
Error: Permission denied (os error 13)
```

**Check:**

```bash
# Check file permissions
ls -la .code_search.json

# Check directory permissions
ls -la

# Fix if needed
chmod 600 .code_search.json
chmod 755 .
```

### JSON Syntax Errors

```
Error: Invalid JSON in .code_search.json
```

**Validate JSON:**

```bash
# Using Python
python -m json.tool .code_search.json

# Using jq
jq . .code_search.json

# Using NodeJS
node -e "console.log(JSON.stringify(require('./.code_search.json'), null, 2))"
```

**Common mistakes:**

- Missing commas between fields
- Trailing commas (not allowed in JSON)
- Unquoted field names
- Single quotes instead of double quotes

### Database Connection Issues

**SQLite specific:**

```bash
# Test database is readable
sqlite3 ./cozo.sqlite ".tables"

# Check if corrupted
sqlite3 ./cozo.sqlite "PRAGMA integrity_check;"

# If corrupted, backup and recreate
mv ./cozo.sqlite ./cozo.sqlite.backup
code_search import -f call_graph.json
```

## Getting Help

If you encounter issues:

1. Check the error message and search this guide
2. Verify your JSON syntax using a JSON validator
3. Ensure the configuration file path is correct
4. Check file and directory permissions
5. Review the examples in this guide

For bug reports or feature requests related to configuration, please open an issue on the project repository.
