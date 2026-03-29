# 1.25 External Declarations

External declarations describe functions and components not implemented in Writ. They have no implementation body and
exist for compile-time type checking and language server support. External declarations are placed in regular `.writ`
files. By convention, projects organize them in a `decl/` directory, but this is not required.

The `extern` keyword applies to two declaration kinds:

1. **`extern fn`** — a function whose implementation is provided externally.
2. **`extern component`** — a data schema whose storage is host-managed (see [Section 1.16](#116-components)).

> **Note:** The `extern` keyword does not apply to types (`struct`, `class`, `enum`, `entity`). Types are always
> defined in Writ — either in the `writ-runtime` module (§2.18) or in Writ library packages. If a runtime consumer
> wants to expose custom types to scripts (e.g., `vec2`, `Color`), the correct approach is to ship a Writ library and
> reference it in `writ.toml` as a dependency. The runtime understands all Writ type layouts natively.

There are two kinds of extern function declarations:

1. **Runtime-provided** — bare `extern fn` with no `[Import]` attribute. The host runtime supplies the implementation
   at embedding time.
2. **Library-imported** — `extern fn` with an `[Import]` attribute. The runtime loads a native library and resolves the
   symbol at call time.

## 1.25.1 Runtime-Provided Externals

Bare `extern` declarations are provided by the host runtime. This is the common case for game scripting — the engine
exposes core functionality to scripts.

```writ
// Runtime-provided functions
extern fn lerp(from: vec2, to: vec2, duration: float) -> vec2;
extern fn wait(seconds: float);
extern fn playSound(name: string);
extern fn random(min: float, max: float) -> float;

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
```

## 1.25.2 Overloading

Extern functions participate in the same overload resolution as regular functions (§1.13.3). Multiple `extern fn`
declarations may share the same name if they have different parameter signatures. The compiler resolves calls based on
argument types at the call site and emits `CALL_EXTERN` with the correct `ExternDef` index.

```writ
extern fn ui_set(element_id: int, key: string, value: float);
extern fn ui_set(element_id: int, key: string, value: string);

// Resolved by argument types
ui_set(rect_id, "x", 100.0);           // calls float overload
ui_set(btn_id, "color", "green");       // calls string overload
```

All Writ types — primitives, structs, classes, enums, arrays, nullable types — pass transparently through extern
function signatures. The runtime understands native Writ type layouts and can marshal them without special handling.

## 1.25.3 Library Imports

The `[Import]` attribute marks an extern declaration as loaded from a native library rather than provided directly by
the runtime.

```writ
[Import("physics")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;
```

### 1.25.3.1 Import Attribute Parameters

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

### 1.25.3.2 Examples

```writ
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

## 1.25.4 Architecture Identifiers

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

## 1.25.5 Library Resolution

When the runtime encounters a call to an `[Import]` extern, it resolves the library in the following order:

1. **Attribute architecture override** — if the current architecture has a named override (e.g., `x64 = "physics64"`),
   use that name.
2. **`writ.toml` libraries section** — if the project defines a `[libraries.<name>]` entry (
   see [Section 1.2](#12-project-configuration-writtoml)), use that mapping.
3. **Logical name** — use the positional argument as-is.

The runtime appends platform-specific file extensions (`.dll`, `.so`, `.dylib`) and applies its own search path
conventions. The Writ language does not specify file extensions or search paths — these are runtime concerns.

## 1.25.6 Symbol Resolution

Symbol resolution follows the same precedence:

1. **Attribute architecture override** — if the current architecture has a symbol override (e.g.,
   `symbol_x64 = "_raycast@24"`), use that name.
2. **Attribute symbol parameter** — if `symbol` is specified, use that name.
3. **Function name** — default to the Writ function name as declared.

## 1.25.7 Crash Semantics

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

