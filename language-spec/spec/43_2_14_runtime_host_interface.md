# Writ IL Specification
## 2.14 Runtime-Host Interface

**Status:** Resolved.

The Writ runtime is embedded inside a host game engine. The runtime owns script state (IL execution, GC, entity
registry) but depends on the host for native capabilities (rendering, physics, audio, input). This interface defines the
contract between the two.

### 2.14.1 Architecture

```
+-----------------------------------------------------------+
|  Host Engine (Godot, Unity, custom, etc.)                  |
|    - Rendering (Sprite)          - Physics (Collider)      |
|    - Audio                       - Input                   |
|    - Native entity storage       - Platform services       |
+----------------------- Host API --------------------------+
|  Writ Runtime                                              |
|    - IL interpreter              - GC                      |
|    - Script state (heap)         - Task scheduler          |
|    - Entity registry (script)    - Contract dispatch       |
+-----------------------------------------------------------+
```

### 2.14.2 Runtime -> Host (requests — runtime suspends until host confirms)

The runtime does not fire-and-forget notifications. When the runtime needs the host to perform an action, it **suspends
execution** until the host has processed the request and confirmed the result. This ensures consistency with the game
engine's logic loop — the host processes changes on its own tick, not asynchronously.

| Request                   | Data Provided                                           | Host Responsibility                                                                           |
|---------------------------|---------------------------------------------------------|-----------------------------------------------------------------------------------------------|
| **Entity spawned**        | type info, initial field values, component field values | Create native representation (sprite, physics body, etc.). Confirm when ready.                |
| **Entity destroyed**      | entity handle                                           | Clean up native resources. Confirm when done.                                                 |
| **Entity unreferenced**   | entity handle, is_singleton                             | Script has no more references. Host decides whether to keep or destroy native side.           |
| **Component field write** | entity handle, component type, field id, new value      | Update native state (e.g., `sprite.visible = false`). Confirm when applied.                   |
| **Component field read**  | entity handle, component type, field id                 | Return current native value to the runtime.                                                   |
| **Extern function call**  | extern index, arguments                                 | Execute native implementation, return result.                                                 |
| **Say / choice / wait**   | speaker, text/options, duration                         | Display dialogue, present choices, wait for time/input.                                       |
| **Save requested**        | —                                                       | Runtime is about to serialize. Host should prepare (flush buffers, etc.). Confirm when ready. |

### 2.14.3 Host -> Runtime (commands the host sends)

| Command            | Data                                                      | Purpose                                           |
|--------------------|-----------------------------------------------------------|---------------------------------------------------|
| **Tick**           | delta time                                                | Advance script execution (resume suspended tasks) |
| **Fire event**     | entity handle, event type (interact/create/destroy), args | Trigger entity lifecycle hooks                    |
| **Start dialogue** | dlg method index, arguments                               | Begin a dialogue sequence                         |
| **Request save**   | —                                                         | Ask the runtime to serialize VM state             |
| **Load save**      | save data                                                 | Restore VM state from a save                      |
| **Hot reload**     | new IL module                                             | Replace running scripts (if supported)            |

### 2.14.4 Entity Ownership Model

**Decision:** The runtime owns all script-defined state. The host owns all native state.

- **Script fields** (fields declared in `entity { ... }`): Stored in the runtime's GC heap. Accessed via `GET_FIELD` /
  `SET_FIELD` directly.
- **Components** (always extern, e.g., `extern component Sprite { ... }`): Data-only schemas. Storage and implementation
  are host-provided. `GET_FIELD` / `SET_FIELD` on component fields are proxied through the host API. Components have no
  script-defined methods.

This means:

- `SPAWN_ENTITY` -> runtime allocates entity in its heap, then notifies host with component initial values.
- `SET_FIELD` on script field -> runtime updates heap directly.
- `SET_FIELD` on component field -> runtime proxies to host, suspends until host confirms.
- `GET_COMPONENT` -> runtime proxies to host.
- `DESTROY_ENTITY` -> runtime fires `on_destroy`, runs defers, removes from registry, notifies host.

### 2.14.5 Singleton Entities and the Host

`[Singleton]` entities have special semantics at the runtime-host boundary:

- When `getOrCreate<T>()` first creates a singleton, the runtime notifies the host so it can create the native
  representation.
- The host may pre-register a native entity that should bind to a `[Singleton]` type. On `getOrCreate`, the runtime
  binds to the existing native entity instead of asking the host to create a new one.
- When the runtime notifies the host of "entity unreferenced," the `is_singleton` flag tells the host this entity should
  likely be preserved (singletons are expected to exist for the lifetime of the game).

### 2.14.6 Scripted Entities: Runtime Requirements

The spec allows entities to be defined entirely in Writ scripts. The runtime MUST support these — it cannot refuse
script-defined entities. However, the runtime is not required to provide a rendering/physics implementation for them. A
script-only entity with no extern components is purely script state.

If a scripted entity uses extern components (`use Sprite { ... }`), the host MUST provide those components. If the host
does not support a required extern component, entity spawning fails with a crash (same semantics as a failed library
load — unrecoverable, defer unwinding).

### 2.14.7 Runtime Logging Interface

The runtime must provide a logging interface that reports events to the host with a severity level. The host decides
how to handle log messages (display to user, write to file, ignore, etc.).

**Required log levels:** `error`, `warn`, `info`, `debug`.

**Events the runtime must log:**

| Event                | Level   | Description                                                                                                                                                           |
|----------------------|---------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Lifecycle hook crash | `error` | A lifecycle hook (`on create`, `on destroy`, `on serialize`, `on deserialize`, `on finalize`) crashed. Includes the entity/struct type, hook name, and error details. |
| Task crash           | `error` | A task's call stack unwound due to an unhandled crash (`!` on None/Err, out-of-bounds, etc.).                                                                         |
| Entity unreferenced  | `debug` | GC determined no script references remain for an entity.                                                                                                              |
| Version mismatch     | `warn`  | IL module version differs between save and current load (see §2.13.2).                                                                                                |

The logging interface is the primary mechanism for the runtime to communicate errors to the host. The spec does not
prescribe the format — the runtime may use callbacks, a message queue, or any other mechanism appropriate to the
embedding environment.

### 2.14.8 Implementation Guidance

The following are **recommendations**, not requirements. Runtime implementors may deviate based on their host
environment.

- **Protocol format:** The runtime-host interface may be implemented as direct function calls (C FFI), a message queue,
  or any other IPC mechanism. For single-process embeddings (the common case), direct function calls with callback
  registration are simplest.
- **Extern function errors:** Extern functions that can fail should return `Result<T, E>` at the Writ level. The
  runtime should not silently swallow host-side errors — propagate them as Writ `Result::Err` values.
- **Hot reload:** If supported, the runtime should apply IL coexistence (§2.13.2) — existing call stacks continue on
  old IL, new calls use new IL. This is the same mechanism as save/load version mismatch handling.
- **Component field validation:** If the host rejects a component field write (invalid value, read-only field), the
  runtime should crash the calling task with a descriptive error, logged via §2.14.7.
- **Dialogue functions:** `say`, `choice`, and `wait` are transition points that suspend execution. The host is
  responsible for presenting UI and signaling completion. See spec §13.9 for the language-level semantics.

