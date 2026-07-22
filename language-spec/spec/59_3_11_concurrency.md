# 3.11 Concurrency

| Mnemonic         | Shape | Operands                                | Description                                                                                                                                                                                                                  |
|------------------|-------|-----------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `SPAWN_TASK`     | CALL  | r_dst, method_idx:u32, r_base, argc:u16 | Spawn a scoped task. Returns a task handle in r_dst. The task is automatically cancelled when the parent scope exits.                                                                                                        |
| `SPAWN_DETACHED` | CALL  | r_dst, method_idx:u32, r_base, argc:u16 | Spawn a detached task. The runtime writes an internal handle to r_dst, but Writ source semantics discard it and the expression has type `void`. The task outlives the parent.                                                  |
| `JOIN`           | RR    | r_dst, r_handle                         | Suspend until the target task completes. The task's return value is placed in r_dst.                                                                                                                                         |
| `CANCEL`         | R     | r_handle                                | Cancel a task. The target task's defer handlers execute in reverse order before termination.                                                                                                                                 |
| `DEFER_PUSH`     | RI32  | —, handler_offset:i32                   | Push a defer handler onto the current frame's defer stack. handler_offset points to a code block within the current method body. The register slot is unused (padding).                                                      |
| `DEFER_POP`      | N     | —                                       | Pop the topmost defer handler without executing it. Used when execution exits a defer's logical scope without returning from the function — the defer is no longer relevant.                                                 |
| `DEFER_END`      | N     | —                                       | Marks the end of a defer handler block. Signals the runtime to continue the unwind chain (execute the next defer, or complete the return/crash). Only reachable via the defer mechanism — never through normal control flow. |

For both spawn instructions, `method_idx` must be a non-null `MethodDef` or `MethodRef` token. The instruction starts
that bytecode method body directly; extern, virtual, delegate, built-in, and unresolved targets are invalid and no
wrapper thunk is implied. `r_base..r_base+argc-1` follows the direct-call ABI: concrete instance methods include `self`
first, while qualified static methods include only explicit arguments.

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

