# Writ Runtime Integration Guide

This guide covers how to embed the Writ runtime in a host application (game engine, app framework, etc.).

## Quick Start

### From Source (requires `compiler` feature)

```rust
use writ_runtime::*;

// 1. Build runtime directly from source
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

### From Compiled Module

```rust
use writ_runtime::*;

// 1. Load a compiled module
let bytes = std::fs::read("script.writc").unwrap();
let module = writ_module::Module::from_bytes(&bytes).unwrap();

// 2. Register extern functions
let mut registry = ExternRegistry::new();
registry.on("storage_get", |args| {
    let _key = &args[0]; // Value::Ref (string)
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

## Architecture Overview

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

**Module loading order:**
1. `writ-runtime` virtual module (built-in: Option, Result, Range, contracts, primitives)
2. Library modules (optional, via `with_library()`)
3. User module (the script being executed)

## RuntimeBuilder

The builder configures and constructs a `Runtime<H>`.

```rust
let rt = RuntimeBuilder::new(user_module)
    .with_library(base_library)    // optional: pre-compiled library modules
    .with_host(host)               // ExternHost, custom RuntimeHost, or NullHost
    .with_gc()                     // optional: enable mark-sweep GC (default: bump allocator)
    .build()?;
```

### Compiling from Source

With the `compiler` feature enabled (`writ-runtime = { features = ["compiler"] }`), you can compile and load Writ source directly:

```rust
// Compile a user script from source
let mut rt = RuntimeBuilder::from_source(r#"
    extern fn greet(name: string);

    pub fn on_open() {
        greet("world");
    }
"#)?
    .with_host(host)
    .build()?;

// Compile a base library from source, then load user script on top
let rt = RuntimeBuilder::new(user_module)
    .with_library_source(r#"
        struct Vec2 { x: float, y: float }
        struct Color { r: float, g: float, b: float, a: float }
    "#)?
    .with_host(host)
    .build()?;
```

The source string must be `'static`. For dynamic strings, use `Box::leak(s.into_boxed_str())`.

### Loading Pre-Compiled Libraries

For production, compile base libraries once into `.writc` files and load them without the compiler dependency:

```rust
let base_lib = writ_module::Module::from_bytes(&std::fs::read("base.writc")?)?;
let user = writ_module::Module::from_bytes(&std::fs::read("app.writc")?)?;

let rt = RuntimeBuilder::new(user)
    .with_library(base_lib)
    .with_host(host)
    .build()?;
```

The user module can reference types and functions from library modules via standard cross-module `ModuleRef`/`TypeRef` resolution. Library modules are loaded before the user module, so forward references resolve correctly.

Multiple libraries are supported — call `with_library()` for each.

## Extern Functions

Scripts declare extern functions that the host provides:

```writ
extern fn storage_get(key: string) -> int;
extern fn ui_rect(x: float, y: float, w: float, h: float, color: string) -> int;
```

### ExternRegistry (Recommended)

The `ExternRegistry` is a convenience builder that maps extern names to handlers:

```rust
let mut registry = ExternRegistry::new();

// Immediate: runs inline, returns a value, task does not suspend
registry.on("storage_get", |args| {
    let key = match &args[0] {
        Value::Ref(_) => "some_key", // resolve from heap
        _ => "",
    };
    Ok(Value::Int(42))
});

// Deferred: task suspends, call is queued for later processing
// Use for ECS mutations or operations that need a specific system phase
registry.defer("ui_rect");

// Logging (log::info, log::warn, etc. are handled automatically)
registry.with_log_handler(|level, msg| {
    println!("[{:?}] {}", level, msg);
});

// Entity lifecycle (optional — defaults to auto-confirm)
registry.with_entity_handler(|req_id, req| {
    match req {
        HostRequest::EntitySpawn { type_idx, .. } => {
            // Create entity in your ECS
            HostResponse::EntityHandle(EntityId::new(0, 0))
        }
        _ => HostResponse::Confirmed,
    }
});

// Validate all externs are covered (log::* and dialogue builtins excluded)
let missing = registry.validate(&module);
if !missing.is_empty() {
    eprintln!("Unhandled externs: {:?}", missing);
}

let host = registry.build(&module);
```

### Deferred Call Processing

For ECS-friendly architectures where the world isn't accessible during VM execution:

```rust
// After tick(), drain deferred calls during a safe ECS phase
let deferred = rt.host_mut().drain_deferred();
for call in deferred {
    // Process the call using your ECS world
    let result = match call.name.as_str() {
        "ui_rect" => {
            let id = create_ui_rect(&call.args);
            HostResponse::Value(Value::Int(id as i64))
        }
        _ => HostResponse::Value(Value::Void),
    };
    // Resume the suspended task
    rt.confirm(call.request_id, result).unwrap();
}
```

### RuntimeHost Trait (Full Control)

For complete control, implement `RuntimeHost` directly:

```rust
impl RuntimeHost for MyHost {
    fn on_request(&mut self, id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { extern_idx, args, .. } => {
                // Dispatch extern calls
                HostResponse::Value(Value::Void)
            }
            HostRequest::EntitySpawn { type_idx, .. } => {
                // Create entity
                HostResponse::EntityHandle(EntityId::new(0, 0))
            }
            HostRequest::FieldRead { entity, field_idx, .. } => {
                // Read component field from ECS
                HostResponse::Value(Value::Int(0))
            }
            HostRequest::FieldWrite { entity, field_idx, value, .. } => {
                // Write component field to ECS
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

Returning `HostResponse::Suspend` from `on_request()` parks the calling task. Resume it later with `rt.confirm(request_id, response)`.

## Execution Model

### Tick Loop

`tick(delta_time, limit)` runs one pass through the ready queue:

```rust
loop {
    match rt.tick(delta_time, ExecutionLimit::Instructions(10_000)) {
        TickResult::AllCompleted => break,
        TickResult::TasksSuspended(pending) => {
            // Process pending requests and confirm them
        }
        TickResult::ExecutionLimitReached => {
            // Budget exhausted — call tick() again next frame
        }
        TickResult::Empty => break,
    }
}
```

- `ExecutionLimit::Instructions(n)` — budget per task per tick
- `ExecutionLimit::None` — run until all tasks complete or suspend

### Synchronous Calls

For fire-and-forget calls that must complete immediately:

```rust
let result = rt.call_sync(method_idx, vec![Value::Int(42)])?;
```

This runs the method to completion in a tight loop. Useful for initialization or pure-computation entry points. Not suitable for methods that suspend on host requests (unless the host auto-confirms).

### Task Lifecycle

Tasks progress through states: `Ready` -> `Running` -> `Suspended` / `Completed` / `Cancelled`.

```rust
// Spawn
let task_id = rt.spawn_task(method_idx, args)?;

// Check state
if let Some(state) = rt.task_state(task_id) { ... }

// Get return value (Completed tasks)
if let Some(val) = rt.return_value(task_id) { ... }

// Get crash info (Cancelled tasks)
if let Some(crash) = rt.crash_info(task_id) {
    eprintln!("{}", crash.format_stacktrace());
}

// Cancel a task tree (e.g. on app close)
rt.cancel_app_tasks(task_id);
```

### Method Lookup

Find methods by name rather than hardcoding indices:

```rust
if let Some(idx) = rt.find_method("on_open") {
    rt.spawn_task(idx, vec![])?;
}

if let Some(idx) = rt.find_method("on_tick") {
    rt.spawn_task(idx, vec![Value::Float(delta_time)])?;
}
```

## Host Request Protocol

The runtime communicates with the host via request/response pairs. When a script performs an operation that requires host involvement, the runtime sends a `HostRequest` and expects a `HostResponse`.

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

Returning `HostResponse::Suspend` from any request parks the calling task. Confirm it later:

```rust
// During on_request:
HostResponse::Suspend  // task parks

// Later, when the operation completes:
rt.confirm(request_id, HostResponse::Value(result))?;  // task resumes
```

## Garbage Collection

The runtime supports two heap modes:

- **BumpHeap** (default): Append-only allocator. Fast, no pauses, but memory is never reclaimed. Suitable for short-lived scripts.
- **MarkSweepHeap** (via `with_gc()`): Traced GC with finalization support. Host-triggered — call `rt.collect_garbage()` during safe points.

```rust
let rt = RuntimeBuilder::new(module)
    .with_gc()           // enable mark-sweep
    .with_host(host)
    .build()?;

// Trigger GC at a safe point (e.g. between frames)
let stats = rt.collect_garbage();
println!("Collected {} objects, freed {} bytes", stats.collected, stats.freed_bytes);
```

## Debug Support

The runtime has DAP (Debug Adapter Protocol) integration for breakpoints and stepping.

```rust
impl RuntimeHost for MyDebugHost {
    fn debug_enabled() -> bool { true }

    fn before_instruction(&mut self, task_id, method_idx, pc, line, col) -> DebugAction {
        if self.breakpoints.contains(&(method_idx, line)) {
            DebugAction::Break  // suspends the task
        } else {
            DebugAction::Continue
        }
    }
}

// Resume after breakpoint
rt.resume_debug(task_id)?;

// Inspect call stack
if let Some(frames) = rt.call_stack_frames(task_id) {
    for (method_idx, pc) in &frames {
        println!("  at method {} pc {}", method_idx, pc);
    }
}

// Read registers
if let Some(regs) = rt.frame_registers(task_id, frame_idx) {
    for (i, val) in regs.iter().enumerate() {
        println!("  r{} = {:?}", i, val);
    }
}
```

## Value Types

`Value` is the runtime's universal value representation:

| Variant | Writ Type | Notes |
|---------|-----------|-------|
| `Value::Void` | `void` | Unit / no value |
| `Value::Int(i64)` | `int` | 64-bit signed integer |
| `Value::Float(f64)` | `float` | 64-bit IEEE float |
| `Value::Bool(bool)` | `bool` | Boolean |
| `Value::Ref(HeapRef)` | `string`, `T[]`, closures | Heap-allocated object handle |
| `Value::Entity(EntityId)` | `entity` types | Generation-indexed entity handle |
| `Value::Struct { type_idx, href }` | `struct`/`class` | Typed heap object |

All `Value` variants are `Copy`. Heap objects are accessed through `HeapRef` handles.

## Typical Integration Pattern

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
        registry.defer("ui_circle");
        registry.defer("ui_text");
        registry.defer("ui_set");
        registry.defer("ui_on_tap");
        registry.defer("ui_remove");
        registry.on("storage_get", |_| Ok(Value::Int(0)));
        registry.on("storage_set", |_| Ok(Value::Void));

        registry.with_log_handler(|level, msg| {
            println!("[{:?}] {}", level, msg);
        });

        let host = registry.build(&module);
        let mut rt = RuntimeBuilder::new(module)
            .with_host(host)
            .with_gc()
            .build()
            .unwrap();

        // Cache entry point indices
        let on_tick = rt.find_method("on_tick");

        // Run on_open
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

        // Run the tick task
        self.rt.tick(dt, ExecutionLimit::Instructions(50_000));
    }

    fn process_ui_call(&mut self, call: &DeferredCall) -> HostResponse {
        match call.name.as_str() {
            "ui_rect" => {
                // Create rect in your UI system using call.args
                HostResponse::Value(Value::Int(1)) // element ID
            }
            "ui_set" => {
                // Update UI element property
                HostResponse::Value(Value::Void)
            }
            _ => HostResponse::Value(Value::Void),
        }
    }
}
```
