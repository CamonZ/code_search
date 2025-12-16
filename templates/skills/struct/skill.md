# struct - Examples

## Show Struct Definition

```bash
code_search --format toon struct --module Phoenix.Socket
```

Output:
```
structs[15]{default,field,inferred_type,module,project,required}:
  "%{}",assigns,"",Phoenix.Socket,default,false
  nil,channel,"",Phoenix.Socket,default,false
  nil,channel_pid,"",Phoenix.Socket,default,false
  nil,endpoint,"",Phoenix.Socket,default,false
  nil,handler,"",Phoenix.Socket,default,false
  nil,id,"",Phoenix.Socket,default,false
  nil,join_ref,"",Phoenix.Socket,default,false
  false,joined,"",Phoenix.Socket,default,false
  "%{}",private,"",Phoenix.Socket,default,false
  nil,pubsub_server,"",Phoenix.Socket,default,false
  nil,ref,"",Phoenix.Socket,default,false
  nil,serializer,"",Phoenix.Socket,default,false
  nil,topic,"",Phoenix.Socket,default,false
  nil,transport,"",Phoenix.Socket,default,false
  nil,transport_pid,"",Phoenix.Socket,default,false
module_pattern: Phoenix.Socket
```

## Find All Structs in Namespace

```bash
code_search --format toon struct --module 'Phoenix\.Socket\.' --regex
```

## Understanding the Output

- `field`: Field name
- `default`: Default value (e.g., `nil`, `%{}`, `[]`, `false`)
- `required`: Whether the field must be provided
- `inferred_type`: Type inferred from usage (when available)

## Common Patterns

- `%{}` - Empty map default
- `[]` - Empty list default
- `nil` - No default (effectively optional)
- `true`/`false` - Boolean defaults
