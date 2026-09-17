# 3.11 Concurrency

| Mnemonic     | Shape | Operands                                | Description                                                                                                                                                                                                                  |
|--------------|-------|-----------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `SPAWN_TASK` | CALL  | r_dst, method_idx:u32, r_base, argc:u16 | Spawn a child task and place its handle in r_dst. The child is automatically cancelled when the parent exits.                                                                                                               |
| `JOIN`       | RR    | r_dst, r_handle                         | Suspend until the target task completes. The task's return value is placed in r_dst.                                                                                                                                         |
| `CANCEL`     | R     | r_handle                                | Cancel a task. The target task's defer handlers execute in reverse order before termination.                                                                                                                                 |
| `DEFER_PUSH` | RI32  | —, handler_offset:i32                   | Push a defer handler onto the current frame's defer stack. handler_offset points to a code block within the current method body. The register slot is unused (padding).                                                      |
| `DEFER_POP`  | N     | —                                       | Pop the topmost defer handler without executing it. Used when execution exits a defer's logical scope without returning from the function — the defer is no longer relevant.                                                 |
| `DEFER_END`  | N     | —                                       | Marks the end of a defer handler block. Signals the runtime to continue the unwind chain (execute the next defer, or complete the return/crash). Only reachable via the defer mechanism — never through normal control flow. |

`method_idx` must be a non-null `MethodDef` or a `MethodRef` that resolves to a `MethodDef`. The resolved definition
must name an executable, non-intrinsic IL method body. Extern definitions, runtime-native/intrinsic definitions,
virtual or delegate dispatch, unresolved references, and empty or absent bodies are not spawnable. No wrapper thunk
or compatibility fallback is implied.

Before creating the child, the runtime must validate the target token, resolution result, bytecode-body availability,
source and destination register bounds, argument count, callee register capacity, and direct-call ABI.
`r_base..r_base+argc-1` contains the arguments: concrete instance methods include `self` first, while qualified static
methods include only explicit arguments. If any validation fails, `SPAWN_TASK` must crash the currently executing task
and must not create a child.

**Defer layout in the method body:**

Defer handler code lives after the method's main code. It is never reached through normal sequential execution — only
through the defer mechanism on return, crash, or cancellation.

```
    ; --- main code ---
    DEFER_PUSH handler_0         ; register first cleanup
    ...                          ; normal code
    DEFER_PUSH handler_1         ; register second cleanup
    ...                          ; more code
    DEFER_POP                    ; (optional) discard handler_1 if scope exits early
    ...
    RET_VOID                     ; triggers defer stack: handler_1 then handler_0

    ; --- defer handlers (after main code) ---
handler_0:
    CALL _, cleanup_fn, ...
    DEFER_END                    ; continue unwind

handler_1:
    CALL _, other_cleanup, ...
    DEFER_END                    ; continue unwind
```

**When defers execute:**

- On `RET` / `RET_VOID`: all defers on the frame's defer stack execute in reverse order (LIFO), then the return
  completes.
- On crash (`CRASH`, `UNWRAP` failure, out-of-bounds, etc.): defers execute during unwinding.
- On `CANCEL`: the target task's defers execute during cancellation.

**DEFER_POP usage:**
Writ's `defer` runs on function exit, not scope exit. However, `DEFER_POP` is available for the compiler to emit in
cases where a defer becomes logically invalid — for example, if a resource is manually cleaned up before the function
returns, the compiler can pop the defer that would have cleaned it up. This is an optimization, not a semantic
requirement. If `DEFER_POP` is never emitted, all defers simply fire on return (which is correct per the spec).

