# unused - Examples

## Find All Unused Functions

```bash
code_search --format toon unused
```

Output:
```
functions[10]{arity,file,kind,line,module,name,project}:
  1,lib/phoenix/socket/message.ex,def,39,Inspect.Phoenix.Socket.Message,__impl__,default
  2,lib/phoenix/socket/message.ex,def,40,Inspect.Phoenix.Socket.Message,inspect,default
  1,lib/mix/phoenix.ex,def,377,Mix.Phoenix,to_text,default
  2,lib/mix/phoenix.ex,def,268,Mix.Phoenix,web_test_path,default
  ...
```

## Find Unused Public Functions

```bash
code_search --format toon unused --public-only
```

These are potential entry points or dead API surface.

## Find Orphan Private Functions

```bash
code_search --format toon unused --private-only
```

Private functions that are never called are definitely dead code.

## Exclude Generated Functions

```bash
code_search --format toon unused --public-only --exclude-generated
```

Filters out `__struct__`, `__using__`, `__before_compile__`, etc.

## Filter to Specific Module

```bash
code_search --format toon unused --module MyApp.Accounts
```

## Understanding Results

- `kind: def` - Public function, might be called externally
- `kind: defp` - Private function, definitely unused if listed
- `kind: defmacro/defmacrop` - Macros, might be compile-time only
