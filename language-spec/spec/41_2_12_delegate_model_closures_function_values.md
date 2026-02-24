# Writ IL Specification
## 2.12 Delegate Model (Closures & Function Values)

All function values in Writ — named function references, closures, and bound method references — are **delegates**. This
borrows from the C# delegate model.

### 2.12.1 Delegate Structure

A delegate is a GC-managed object containing:

```
Delegate {
    target: Option<object>,   // capture struct, self, or null
    method: method_index,     // resolved concrete method
}
```

### 2.12.2 Creation Scenarios

**Plain function reference** (no target):

```
// let func = add;  (IL supports this; language syntax TBD due to overloading)
NEW_DELEGATE  r_func, method_idx(add), r_null    // target = null
```

**Closure with captures:**

```
// let f = fn(x: int) -> int { x + bonus };
NEW           r_env, __closure_env_type            // capture struct
SET_FIELD     r_env, bonus_field, r_bonus          // copy captured value
NEW_DELEGATE  r_f, method_idx(__closure_body), r_env  // target = capture struct
```

**Closure without captures** (optimized — no allocation for empty env):

```
// let f = fn(x: int) -> int { x + 1 };
NEW_DELEGATE  r_f, method_idx(__closure_body), r_null   // target = null, no env needed
```

**Bound method reference:**

```
// let greet = merchant.greet;
NEW_DELEGATE  r_greet, method_idx(Merchant::greet), r_merchant  // target = self
```

### 2.12.3 Invocation

All delegates are called with `CALL_INDIRECT`:

```
CALL_INDIRECT  r_result, r_delegate, r_base, argc
```

The runtime:

1. Reads the delegate from `r_delegate`.
2. Extracts `target` and `method`.
3. If `target` is non-null, prepends it as the first argument (it becomes `r0` / self / env in the callee).
4. Calls the resolved method.

The callee does not know or care whether it was called directly, through a delegate, or through a closure.

### 2.12.4 Virtual Method References

`NEW_DELEGATE` always takes a **concrete method index**. For virtual/contract methods, the compiler resolves the
dispatch at delegate creation time. If that's not possible (rare), the compiler generates a small wrapper closure that
performs the virtual call internally.

### 2.12.5 Relationship to Function Types

The language spec's `fn(int, int) -> int` type corresponds to a delegate in the IL. Every value of a function type is a
delegate. No language spec change is needed beyond documenting this representation.

