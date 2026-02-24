# 1. Writ Language Specification
## 24. External Declarations

External declarations describe types, functions, components, and other constructs not implemented in Writ. They have no
implementation body and exist for compile-time type checking and language server support. External declarations are
placed in regular `.writ` files. By convention, projects organize them in a `decl/` directory, but this is not required.

There are two kinds of external declarations:

1. **Runtime-provided** — bare `extern` with no `[Import]` attribute. The host runtime supplies the implementation at
   embedding time.
2. **Library-imported** — `extern` with an `[Import]` attribute. The runtime loads a native library and resolves the
   symbol at call time.

### 24.1 Runtime-Provided Externals

Bare `extern` declarations are provided by the host runtime. This is the common case for game scripting — the engine
exposes core functionality to scripts.

```
// Runtime-provided functions
extern fn lerp(from: vec2, to: vec2, duration: float) -> vec2;
extern fn wait(seconds: float);
extern fn playSound(name: string);
extern fn random(min: float, max: float) -> float;

// Runtime-provided structs
extern struct vec2 {
    x: float,
    y: float,
}

extern struct Entity {
    position: vec2,
    name: string,
    fn moveTo(target: vec2, speed: float);
    fn destroy();
}

// Runtime-provided components (data-only — no methods)
extern component Sprite {
    texture: string,
    scale: float = 1.0,
    visible: bool = true,
}

extern component Speaker {
    displayName: string,
    color: string = "#FFFFFF",
    portrait: string = "",
    voice: string = "",
}

extern component Health {
    current: int,
    max: int,
}

// Entity namespace utilities
extern fn Entity.getOrCreate<T>() -> T;
extern fn Entity.findAll<T>() -> EntityList<T>;
extern fn Entity.findNearest<T>(position: vec2) -> T?;
```

### 24.2 Library Imports

The `[Import]` attribute marks an extern declaration as loaded from a native library rather than provided directly by
the runtime.

```
[Import("physics")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;
```

#### 24.2.1 Import Attribute Parameters

The `[Import]` attribute accepts one positional argument (the logical library name) and optional named arguments for
symbol naming and architecture-specific overrides.

**Library name parameters:**

| Parameter      | Type   | Description                                                                 |
|----------------|--------|-----------------------------------------------------------------------------|
| *(positional)* | string | Logical library name. Resolved by the runtime or via `writ.toml`. Required. |
| `x86`          | string | Library name override for x86 architecture.                                 |
| `x64`          | string | Library name override for x64 architecture.                                 |
| `arm`          | string | Library name override for arm architecture.                                 |
| `arm64`        | string | Library name override for arm64 architecture.                               |
| `wasm32`       | string | Library name override for wasm32 architecture.                              |

**Symbol name parameters:**

| Parameter       | Type   | Description                                                                |
|-----------------|--------|----------------------------------------------------------------------------|
| `symbol`        | string | Symbol name in the library. Defaults to the Writ function name if omitted. |
| `symbol_x86`    | string | Symbol name override for x86 architecture.                                 |
| `symbol_x64`    | string | Symbol name override for x64 architecture.                                 |
| `symbol_arm`    | string | Symbol name override for arm architecture.                                 |
| `symbol_arm64`  | string | Symbol name override for arm64 architecture.                               |
| `symbol_wasm32` | string | Symbol name override for wasm32 architecture.                              |

These parameters form a closed set. The compiler rejects unrecognized named arguments in `[Import]`.

#### 24.2.2 Examples

```
// Minimal — logical name only, symbol defaults to function name
[Import("physics")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Custom symbol name (library exports a different name than the Writ function)
[Import("physics", symbol = "phys_raycast_2d")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Architecture-specific library names
[Import("physics", x64 = "physics64", arm64 = "physics_arm")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Architecture-specific symbol names (name mangling differences)
[Import("physics", symbol = "raycast", symbol_x64 = "_raycast@24")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Full override example
[Import("audio", x64 = "fmod64", arm64 = "fmod_arm", symbol = "FMOD_PlaySound")]
extern fn playMusic(path: string, volume: float);
```

### 24.3 Architecture Identifiers

The following architecture identifiers are recognized by the compiler:

| Identifier | Architecture                        |
|------------|-------------------------------------|
| `x86`      | 32-bit Intel / AMD                  |
| `x64`      | 64-bit Intel / AMD (x86_64 / AMD64) |
| `arm`      | 32-bit ARM                          |
| `arm64`    | 64-bit ARM (AArch64)                |
| `wasm32`   | 32-bit WebAssembly                  |

Unrecognized architecture identifiers in `[Import]` named parameters are a compile error.

> **Note:** Architecture identifiers refer to instruction set architecture only. Platform concerns (operating system,
> file extensions, library search paths) are the runtime's responsibility.

### 24.4 Library Resolution

When the runtime encounters a call to an `[Import]` extern, it resolves the library in the following order:

1. **Attribute architecture override** — if the current architecture has a named override (e.g., `x64 = "physics64"`),
   use that name.
2. **`writ.toml` libraries section** — if the project defines a `[libraries.<name>]` entry (
   see [Section 2](#2-project-configuration-writtoml)), use that mapping.
3. **Logical name** — use the positional argument as-is.

The runtime appends platform-specific file extensions (`.dll`, `.so`, `.dylib`) and applies its own search path
conventions. The Writ language does not specify file extensions or search paths — these are runtime concerns.

### 24.5 Symbol Resolution

Symbol resolution follows the same precedence:

1. **Attribute architecture override** — if the current architecture has a symbol override (e.g.,
   `symbol_x64 = "_raycast@24"`), use that name.
2. **Attribute symbol parameter** — if `symbol` is specified, use that name.
3. **Function name** — default to the Writ function name as declared.

### 24.6 Crash Semantics

Library loading and symbol resolution are **not recoverable operations**. If the runtime cannot load a library or
resolve a symbol:

1. The runtime MUST terminate the current task.
2. All `defer` blocks in the call chain unwind and execute, in reverse order (same as cancellation).
3. The crash propagates through the entire task chain — parent tasks that spawned the failing task are also terminated.

This is an unrecoverable error, not a `Result`. Script code cannot catch or recover from a failed library load. The
runtime MAY reject a library load for any reason, including security policy (e.g., unsigned libraries, disallowed paths,
sandboxing). The behavior is the same: crash with defer unwinding.

> **Rationale:** Library imports are an injection surface. The runtime is the gatekeeper — it decides which libraries
> are permitted. Making failures unrecoverable prevents scripts from silently falling back to alternate code paths when
> a
> library is blocked, which could mask security violations.

---

