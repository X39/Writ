# 2.12 Delegate Model (Closures & Function Values)

All function values in Writ — named function references, closures, and bound method references — are **delegates**. This
borrows from the C# delegate model.

## 2.12.1 Delegate Structure

A delegate is a GC-managed object containing:

```
Delegate {
    target: optional receiver/capture value,
    method: (module, MethodDef row),  // resolved concrete method identity
}
```

The resolved `MethodDef` requires a receiver exactly when its `owner` token is non-null and its `STATIC` flag is clear.
Such a method requires a non-null delegate target. A top-level function or static method requires a null target. The
runtime compares these two conditions; it never repairs a mismatch by treating the target as an explicit argument or
by treating an explicit argument as `self`.

`NEW_DELEGATE` validates its destination and target registers, resolves its method operand, verifies that the resolved
method and body metadata are present and consistent, and validates the target/receiver binding before allocating the
delegate. The method operand may be a local `MethodDef` token or a `MethodRef` token. A `MethodRef` is resolved at
creation time, and the delegate stores the resulting concrete `(module, MethodDef row)` identity rather than the
reference token.

An invalid register, null or unsupported method token, unresolved `MethodRef`, missing or inconsistent method body, or
target/receiver mismatch is a runtime error. Every such runtime error crashes the current task. Validation completes
before allocation, so a failing `NEW_DELEGATE` does not allocate a delegate or write a result to its destination
register.

## 2.12.2 Creation Scenarios

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

## 2.12.3 Invocation

All delegates are called with `CALL_INDIRECT`:

```
CALL_INDIRECT  r_result, r_delegate, r_base, argc
```

The runtime:

1. Validates the destination, delegate, and explicit-argument register ranges.
2. Requires `r_delegate` to refer to a delegate object and extracts its target and concrete method identity.
3. Revalidates the stored method metadata and the target/receiver binding. This second validation is required because
   delegates may enter the heap through deserialization, host integration, or programmatic runtime APIs rather than
   through `NEW_DELEGATE`.
4. Validates that `argc` plus the implicit receiver count exactly equals `MethodDef.param_count` and that the callee
   has enough registers. The implicit receiver count is one for a receiver method and zero otherwise.
5. For a receiver method, prepends the validated target as the first argument; it becomes `r0` (`self` or the closure
   environment) in the callee.
6. Pushes the callee frame and calls the resolved method.

A non-delegate value, invalid stored method identity or body, target/receiver mismatch, invalid register range,
argument-count mismatch, or insufficient callee register capacity is a runtime error. Every such runtime error crashes
the current task before a callee frame is created. The runtime does not continue the malformed call, reinterpret its
arguments, or panic the runtime process.

The callee does not know or care whether it was called directly, through a delegate, or through a closure.

## 2.12.4 Virtual Method References

The encoded `NEW_DELEGATE` operand is a method metadata token: either a local `MethodDef` or a `MethodRef`. Both forms
must resolve to a concrete `(module, MethodDef row)` before the delegate is allocated. For a virtual or contract method
reference, the compiler resolves dispatch at delegate creation time when possible. Otherwise, it generates a small
wrapper closure whose concrete body performs the virtual call.

## 2.12.5 Relationship to Function Types

The language spec's `fn(int, int) -> int` type corresponds to a delegate in the IL. Every value of a function type is a
delegate. No language spec change is needed beyond documenting this representation.

