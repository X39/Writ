# 1. Writ Language Specification
## 20. Concurrency

All function calls implicitly yield if needed (coroutine-based). Script authors do not think about async/await for
normal sequential code. Explicit concurrency primitives are provided for background tasks.

### 20.1 Execution Model

Every function is implicitly a coroutine. When the runtime encounters a blocking operation (`wait()`, `say()`, player
input), it yields control to the game engine. The engine resumes execution when appropriate. This is invisible to the
script author.

### 20.2 Concurrency Primitives

| Primitive        | Syntax                | Behavior                                                                                        |
|------------------|-----------------------|-------------------------------------------------------------------------------------------------|
| `spawn`          | `spawn expr`          | Starts a background task, returns a handle. Scoped to parent — auto-cancelled when parent ends. |
| `spawn detached` | `spawn detached expr` | Independent background task. Outlives parent scope.                                             |
| `join`           | `join handle`         | Wait for a spawned task to complete.                                                            |
| `cancel`         | `cancel handle`       | Hard-terminate a task. Runs `defer` blocks.                                                     |
| `defer`          | `defer { ... }`       | Cleanup code that runs on normal return or cancellation.                                        |

```
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

### 20.3 Task Lifetime Rules

Scoped tasks (`spawn`) are automatically cancelled when their parent scope exits (normal return, `->` transition, or
cancellation). Detached tasks (`spawn detached`) run independently and must be explicitly cancelled or run to
completion.

---

