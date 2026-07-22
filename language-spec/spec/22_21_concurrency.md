# 1.21 Concurrency

All function calls implicitly yield if needed (coroutine-based). Script authors do not think about async/await for
normal sequential code. Explicit concurrency primitives are provided for background tasks.

## 1.21.1 Execution Model

Every function is implicitly a coroutine. When the runtime encounters a blocking operation (`wait()`, `say()`, player
input), it yields control to the game engine. The engine resumes execution when appropriate. This is invisible to the
script author.

The full task state machine and scheduling semantics are specified in the [IL Execution Model](../il-spec/execution.md).

## 1.21.2 Concurrency Primitives

| Primitive        | Syntax                | Behavior                                                                                        |
|------------------|-----------------------|-------------------------------------------------------------------------------------------------|
| `spawn`          | `spawn call`          | Starts a background task, returns a handle. Scoped to parent — auto-cancelled when parent ends. |
| `spawn detached` | `spawn detached call` | Starts an independent background task and discards its handle. The expression has type `void`.  |
| `join`           | `join handle`         | Wait for a spawned task to complete.                                                            |
| `cancel`         | `cancel handle`       | Hard-terminate a task. Runs `defer` blocks.                                                     |
| `defer`          | `defer { ... }`       | Cleanup code that runs on normal return or cancellation.                                        |

```writ
dlg boulderScene {
    @Narrator The ground shakes...
    $ let task = spawn moveBoulder(vec2 { x: 10.0, y: 5.0 });
    @Narrator Quick, get out of the way!
    $ choice {
        "Run!" {
            $ cancel task;
            @Narrator You dodge just in time.
        }
        "Stand firm" {
            $ join task;
            @Narrator The boulder settles into place.
        }
    }
}

fn moveBoulder(target: vec2) {
    defer { boulder.animation = "idle"; }
    boulder.animation = "rolling";
    lerp(boulder.position, target, 3.0);
}
```

## 1.21.3 Task Lifetime Rules

Scoped tasks (`spawn`) are automatically cancelled when their parent scope exits (normal return, `->` transition, or
cancellation). Detached tasks (`spawn detached`) run independently until completion or runtime shutdown/cancellation.
The source expression discards the detached task's handle, so it cannot later be passed to `join` or `cancel`; use
scoped `spawn` when the script needs a handle.

## 1.21.4 Spawnable Calls

The operand of `spawn` or `spawn detached` must be a call that the compiler resolves statically to a concrete Writ
bytecode function or method (`MethodDef` or `MethodRef`). A concrete instance method passes `self` first, followed by
the explicit arguments. A qualified static method passes only its explicit arguments; its qualifier is not a receiver.

The compiler rejects non-call operands, extern/native calls, virtual contract or generic dispatch, delegate calls,
built-in operations, and unresolved calls. These forms do not identify one bytecode method body that a task can start.
The compiler does not synthesize wrapper thunks to make them spawnable.

---

