# types - Examples

## All Types in a Module

```bash
code_search --format toon types Phoenix.Socket
```

Output:
```
types[5]{definition,kind,line,module,name,params,project}:
  "@type t() :: %{__struct__: Phoenix.Socket, assigns: map(), channel: atom(), ...}",type,273,Phoenix.Socket,t,"[]",default
  "@type t() :: %{__struct__: Phoenix.Socket.Broadcast, event: term(), payload: term(), topic: term()}",type,83,Phoenix.Socket.Broadcast,t,"[]",default
  "@type t() :: %{__struct__: Phoenix.Socket.Message, ...}",type,16,Phoenix.Socket.Message,t,"[]",default
  "@type t() :: %{__struct__: Phoenix.Socket.Reply, ...}",type,67,Phoenix.Socket.Reply,t,"[]",default
  "@type state() :: term()",type,99,Phoenix.Socket.Transport,state,"[]",default
module: Phoenix.Socket
```

## Filter by Type Name

```bash
code_search --format toon types Phoenix.Socket --name t
```

## Only Opaque Types

```bash
code_search --format toon types MyApp --kind opaque
```

## Only Private Types

```bash
code_search --format toon types MyApp --kind typep
```

## Understanding Type Kinds

- `type` - Public type, visible outside the module
- `typep` - Private type, only visible within the module
- `opaque` - Public but implementation hidden (abstract type)
