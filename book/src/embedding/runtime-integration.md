# Runtime Integration

This chapter covers how to embed the Writ runtime in a Rust host application -- a game engine, app framework, or any system that needs to run Writ scripts.

## Architecture

```
Host Application
    |
    v
RuntimeBuilder  -->  Runtime<H>
    |                    |
    |  writ-runtime      |  tick() / confirm() / spawn_task()
    |  (virtual module)  |
    |                    v
    |  library modules   Scheduler
    |  (optional)           |
    |                    Tasks --> Dispatch --> Instructions
    |  user module          |
    |                    Host <-- HostRequest / HostResponse
    v
  Domain (module resolution, dispatch table)
```

Module loading order:
1. `writ-runtime` virtual module (built-in: Option, Result, Range, contracts, primitives)
2. Library modules (optional, via `with_library()`)
3. User module (the script being executed)

## Quick Start

### From a Compiled Module

```rust
use writ_runtime::*;

// 1. Load a compiled module
let bytes = std::fs::read("script.writc").unwrap();
let module = writ_module::Module::from_bytes(&bytes).unwrap();

// 2. Register extern functions
let mut registry = ExternRegistry::new();
registry.on("storage_get", |args| {
    let _key = &args[0];
    Ok(Value::Int(0))
});
registry.with_log_handler(|level, msg| {
    println!("[{:?}] {}", level, msg);
});
let host = registry.build(&module);

// 3. Build the runtime
let mut rt = RuntimeBuilder::new(module)
    .with_host(host)
    .build()
    .unwrap();

// 4. Find and spawn the entry point
let method = rt.find_method("on_open").unwrap();
let task = rt.spawn_task(method, vec![]).unwrap();

// 5. Tick loop
loop {
    match rt.tick(0.016, ExecutionLimit::Instructions(10_000)) {
        TickResult::AllCompleted => break,
        TickResult::TasksSuspended(pending) => {
            for p in pending {
                rt.confirm(p.request_id, HostResponse::Confirmed).unwrap();
            }
        }
        TickResult::ExecutionLimitReached => continue,
        TickResult::Empty => break,
    }
}
```

### From Source (requires `compiler` feature)

```rust
use writ_runtime::*;

let mut rt = RuntimeBuilder::from_source(r#"
    extern fn greet(name: string);
    pub fn on_open() {
        greet("hello");
    }
"#).unwrap()
    .with_host(host)
    .build()
    .unwrap();
```

Enable with `writ-runtime = { features = ["compiler"] }`. The source string must be `'static` -- for dynamic strings, use `Box::leak(s.into_boxed_str())`.

## RuntimeBuilder

```rust
let rt = RuntimeBuilder::new(user_module)
    .with_library(base_library)    // optional: pre-compiled library modules
    .with_host(host)               // ExternHost, custom RuntimeHost, or NullHost
    .with_gc()                     // optional: enable mark-sweep GC (default: bump allocator)
    .build()?;
```

### Pre-Compiled Libraries

For production, compile base libraries once into `.writc` files:

```rust
let base_lib = writ_module::Module::from_bytes(&std::fs::read("base.writc")?)?;
let user = writ_module::Module::from_bytes(&std::fs::read("app.writc")?)?;

let rt = RuntimeBuilder::new(user)
    .with_library(base_lib)
    .with_host(host)
    .build()?;
```

Multiple libraries are supported -- call `with_library()` for each.

## Extern Functions

Scripts declare extern functions that the host provides:

```writ
extern fn storage_get(key: string) -> int;
extern fn ui_rect(x: float, y: float, w: float, h: float, color: string) -> int;
```

### ExternRegistry

The `ExternRegistry` maps extern names to handlers:

```rust
let mut registry = ExternRegistry::new();

// Immediate: runs inline, returns a value
registry.on("storage_get", |args| {
    Ok(Value::Int(42))
});

// Deferred: task suspends, call is queued for later processing
registry.defer("ui_rect");

// Logging
registry.with_log_handler(|level, msg| {
    println!("[{:?}] {}", level, msg);
});

// Entity lifecycle (optional -- defaults to auto-confirm)
registry.with_entity_handler(|req_id, req| {
    match req {
        HostRequest::EntitySpawn { type_idx, .. } => {
            HostResponse::EntityHandle(EntityId::new(0, 0))
        }
        _ => HostResponse::Confirmed,
    }
});

// Validate all externs are covered
let missing = registry.validate(&module);
if !missing.is_empty() {
    eprintln!("Unhandled externs: {:?}", missing);
}

let host = registry.build(&module);
```

### Deferred Call Processing

For ECS-friendly architectures where the world is not accessible during VM execution:

```rust
// After tick(), drain deferred calls during a safe ECS phase
let deferred = rt.host_mut().drain_deferred();
for call in deferred {
    let result = match call.name.as_str() {
        "ui_rect" => {
            let id = create_ui_rect(&call.args);
            HostResponse::Value(Value::Int(id as i64))
        }
        _ => HostResponse::Value(Value::Void),
    };
    rt.confirm(call.request_id, result).unwrap();
}
```

### RuntimeHost Trait

For complete control, implement `RuntimeHost` directly:

```rust
impl RuntimeHost for MyHost {
    fn on_request(&mut self, id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { extern_idx, args, .. } => {
                HostResponse::Value(Value::Void)
            }
            HostRequest::EntitySpawn { type_idx, .. } => {
                HostResponse::EntityHandle(EntityId::new(0, 0))
            }
            HostRequest::FieldRead { entity, field_idx, .. } => {
                HostResponse::Value(Value::Int(0))
            }
            HostRequest::FieldWrite { entity, field_idx, value, .. } => {
                HostResponse::Confirmed
            }
            _ => HostResponse::Confirmed,
        }
    }

    fn on_log(&mut self, level: LogLevel, message: &str) {
        println!("[{:?}] {}", level, message);
    }
}
```

Returning `HostResponse::Suspend` parks the calling task. Resume it later with `rt.confirm(request_id, response)`.

## Execution Model

### Tick Loop

`tick(delta_time, limit)` runs one pass through the ready queue:

- `ExecutionLimit::Instructions(n)` -- budget per task per tick
- `ExecutionLimit::None` -- run until all tasks complete or suspend

### Synchronous Calls

For fire-and-forget calls that must complete immediately:

```rust
let result = rt.call_sync(method_idx, vec![Value::Int(42)])?;
```

Not suitable for methods that suspend on host requests.

### Task Lifecycle

Tasks progress through states: `Ready` -> `Running` -> `Suspended` / `Completed` / `Cancelled`.

```rust
let task_id = rt.spawn_task(method_idx, args)?;

if let Some(state) = rt.task_state(task_id) { /* ... */ }
if let Some(val) = rt.return_value(task_id) { /* ... */ }
if let Some(crash) = rt.crash_info(task_id) {
    eprintln!("{}", crash.format_stacktrace());
}

rt.cancel_app_tasks(task_id);
```

### Method Lookup

Find methods by name rather than hardcoding indices:

```rust
if let Some(idx) = rt.find_method("on_tick") {
    rt.spawn_task(idx, vec![Value::Float(delta_time)])?;
}
```

## Host Request Protocol

| Request | When | Expected Response |
|---------|------|-------------------|
| `ExternCall` | Script calls an `extern fn` | `Value(result)` or `Suspend` |
| `EntitySpawn` | `spawn EntityType { ... }` | `EntityHandle(id)` or `Confirmed` |
| `FieldRead` | Read a component field | `Value(field_value)` |
| `FieldWrite` | Write a component field | `Confirmed` |
| `GetComponent` | Access `entity[Component]` | `Value(component_ref)` |
| `InitEntity` | Entity construction complete | `Confirmed` |
| `DestroyEntity` | `Entity.destroy(e)` | `Confirmed` |
| `GetOrCreate` | `Entity.getOrCreate<T>()` | `EntityHandle(id)` |
| `Join` | `spawn` task join | `Confirmed` (when target completes) |

## Garbage Collection

Two heap modes:

- **BumpHeap** (default) -- append-only, fast, no pauses, memory never reclaimed. Suitable for short-lived scripts.
- **MarkSweepHeap** (via `with_gc()`) -- traced GC with finalization. Host-triggered.

```rust
let rt = RuntimeBuilder::new(module)
    .with_gc()
    .with_host(host)
    .build()?;

let stats = rt.collect_garbage();
println!("Collected {} objects, freed {} bytes", stats.collected, stats.freed_bytes);
```

## Value Types

| Variant | Writ Type | Notes |
|---------|-----------|-------|
| `Value::Void` | `void` | Unit / no value |
| `Value::Int(i64)` | `int` | 64-bit signed integer |
| `Value::Float(f64)` | `float` | 64-bit IEEE float |
| `Value::Bool(bool)` | `bool` | Boolean |
| `Value::Ref(HeapRef)` | `string`, `T[]`, closures | Heap-allocated object handle |
| `Value::Entity(EntityId)` | entity types | Generation-indexed entity handle |
| `Value::Struct { type_idx, href }` | `struct`/`class` | Typed heap object |

## Complete Example

A typical integration pattern for a game application:

```rust
use writ_runtime::*;

struct App {
    rt: Runtime<ExternHost>,
    on_tick: Option<usize>,
}

impl App {
    fn new(script_bytes: &[u8]) -> Self {
        let module = writ_module::Module::from_bytes(script_bytes).unwrap();

        let mut registry = ExternRegistry::new();
        registry.on("safe_top", |_| Ok(Value::Float(44.0)));
        registry.on("safe_bottom", |_| Ok(Value::Float(812.0)));
        registry.on("sin", |args| {
            let v = match args[0] { Value::Float(f) => f, _ => 0.0 };
            Ok(Value::Float(v.sin()))
        });
        registry.defer("ui_rect");
        registry.defer("ui_text");

        registry.with_log_handler(|level, msg| {
            println!("[{:?}] {}", level, msg);
        });

        let host = registry.build(&module);
        let mut rt = RuntimeBuilder::new(module)
            .with_host(host)
            .with_gc()
            .build()
            .unwrap();

        let on_tick = rt.find_method("on_tick");

        if let Some(idx) = rt.find_method("on_open") {
            rt.spawn_task(idx, vec![]).unwrap();
        }

        App { rt, on_tick }
    }

    fn tick(&mut self, dt: f64) {
        // Run pending tasks
        self.rt.tick(dt, ExecutionLimit::Instructions(50_000));

        // Process deferred UI calls
        let deferred = self.rt.host_mut().drain_deferred();
        for call in deferred {
            let result = self.process_ui_call(&call);
            self.rt.confirm(call.request_id, result).unwrap();
        }

        // Spawn on_tick if available
        if let Some(idx) = self.on_tick {
            let _ = self.rt.spawn_task(idx, vec![]);
        }

        self.rt.tick(dt, ExecutionLimit::Instructions(50_000));
    }

    fn process_ui_call(&mut self, call: &DeferredCall) -> HostResponse {
        match call.name.as_str() {
            "ui_rect" => HostResponse::Value(Value::Int(1)),
            _ => HostResponse::Value(Value::Void),
        }
    }
}
```
