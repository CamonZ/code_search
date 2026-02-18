# Using code_search with Git Worktrees

Each worktree must have its own local database. The tool uses an embedded
RocksDB which only allows one process to hold the lock at a time — if multiple
worktrees share the global `~/.code_search/surrealdb.rocksdb` you will get:

```
Error: "Failed to connect to SurrealDB: IO error: While lock file: LOCK:
Resource temporarily unavailable"
```

## Setup per worktree

Run this once in each worktree root:

```sh
mix compile
ex_ast --output /tmp/call_graph.json
code_search setup
code_search code import --file /tmp/call_graph.json
rm /tmp/call_graph.json
```

`code_search setup` creates `.code_search/surrealdb.rocksdb` in the current
directory. The tool's path resolution always prefers a local `.code_search/`
over the global `~/.code_search/`, so each worktree gets its own isolated
database and lock file.

## Rebuilding a stale or corrupt database

If you see a "record already exists" or lock error on a worktree that already
has a local DB:

```sh
rm -rf .code_search/surrealdb.rocksdb
mix compile
ex_ast --output /tmp/call_graph.json
code_search setup
code_search code import --file /tmp/call_graph.json
rm /tmp/call_graph.json
```

## Important notes

- Never rely on `~/.code_search/surrealdb.rocksdb` in a multi-worktree setup
- Two processes cannot share the same RocksDB — this is an embedded database
  limitation, not a bug in code_search
- Each worktree's `.code_search/` should be in `.gitignore` (it is by default)
- The git hook installed via `code_search setup --install-hooks` handles
  incremental updates automatically after each commit, so you only need to do
  the full import once per worktree
