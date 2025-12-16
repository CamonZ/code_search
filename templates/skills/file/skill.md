# file - Examples

## List Functions in a File

```bash
code_search --format toon file --file lib/phoenix/channel.ex
```

Output:
```
files{lib/phoenix/channel.ex}[17]{arity,end_line,guard,kind,module,name,pattern,start_line}:
  0,450,defmacro,Phoenix.Channel,__using__,_,450
  1,462,"",defmacro,Phoenix.Channel,__using__,opts,450
  0,486,defmacro,Phoenix.Channel,__before_compile__,_,486
  1,524,"",defmacro,Phoenix.Channel,intercept,events,522
  6,535,"is_binary(event)",def,Phoenix.Channel,__on_definition__,"env, :def, :handle_out, [event, _payload, _socket], _, _",529
  6,540,"",def,Phoenix.Channel,__on_definition__,"_env, _kind, _name, _args, _guards, _body",540
  3,561,"",def,Phoenix.Channel,broadcast,"socket, event, message",559
  ...
file_pattern: lib/phoenix/channel.ex
```

## Search by Partial Path

```bash
code_search --format toon file --file channel.ex
```

Matches any file containing "channel.ex" in its path.

## Regex Pattern for Multiple Files

```bash
code_search --format toon file --file 'lib/.*_test\.ex' --regex
```

Find all test files.

## Understanding the Output

Functions are listed with:
- `start_line:end_line` - Line range of the function clause
- `kind` - def, defp, defmacro, defmacrop
- `pattern` - Function arguments/pattern match
- `guard` - Guard clause if present

## Use Case: File Overview

```bash
code_search --format toon file --file lib/my_app/accounts.ex
```

Quickly see what's in a file without opening it.
