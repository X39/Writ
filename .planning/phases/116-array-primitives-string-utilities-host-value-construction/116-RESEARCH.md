# Phase 116: Array Primitives, String Utilities & Host Value Construction - Research

**Researched:** 2026-03-29
**Domain:** Writ compiler dot-call resolution, runtime intrinsic dispatch, virtual module registration, host API design
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Claude's Discretion
All implementation choices are delegated.

### Deferred Ideas (OUT OF SCOPE)
None — discuss phase skipped.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STR-01 | User can call `string.split(sep)` to split a string into an array | New string method in type checker `access.rs`, new IL instruction (or intrinsic via CALL_VIRT), new `IntrinsicId` variant, new `execute_intrinsic` arm, new virtual module method |
| STR-02 | User can call `string.trim()` to remove leading/trailing whitespace | Same 4-layer pipeline as STR-01; Rust `str::trim()` |
| STR-03 | User can call `string.starts_with(prefix)` and `string.ends_with(suffix)` | Same 4-layer pipeline; Rust `str::starts_with` / `str::ends_with` |
| STR-04 | User can call `string.contains(substr)` to check substring presence | Same 4-layer pipeline; Rust `str::contains` |
| STR-05 | User can call `string.replace(from, to)` to replace occurrences | Same 4-layer pipeline; Rust `str::replace` |
| STR-06 | User can call `string.to_upper()` and `string.to_lower()` for case conversion | Same 4-layer pipeline; `to_ascii_uppercase`/`to_ascii_lowercase` per STATE.md decision |
| HOST-01 | Host Rust code can construct a Writ struct/class value by name with runtime type validation | New `Runtime::construct_value(type_name, fields)` public method; Domain lookup by name; heap allocation; field count + type validation |
| HOST-02 | Host Rust code can construct values in immediate extern handlers (heap access provided) | New `ExternHandler::ImmediateWithHeap` variant carrying `Box<dyn FnMut(&[Value], &mut dyn GcHeap) -> Result<Value, String>>` |
| HOST-03 | Runtime returns a clear error when host constructs a value with wrong field count or mismatched field types | `Err(String)` with descriptive message from `construct_value` |
</phase_requirements>

## Summary

Phase 116 has three independent workstreams that share no code: (1) array mutation method compiler wiring, (2) string utility intrinsics, and (3) host value construction API. None of the three requires changes to IL instruction encoding or the parser — they are purely compiler type-checker additions, emitter additions, runtime dispatch additions, and public API additions.

The VM opcodes for array mutation (`ArrayAdd` 0x0905, `ArrayRemove` 0x0906, `ArrayInsert` 0x0907, `ArraySlice` 0x0908) already exist in `writ-module` and are already dispatched in the runtime (`exec_array_add`, `exec_array_remove`, `exec_array_insert`, `exec_array_slice`). The type checker only recognizes `len`, `push` (partial), and `slice` on arrays; `add`, `remove_at`, `insert`, and `contains` are missing from the type checker and emitter. `contains` has no VM opcode and must be implemented in the emitter as a linear scan loop or as a new intrinsic — the simplest correct choice is a new intrinsic since the loop requires a contract-dispatch equality check.

String utilities do not have VM opcodes or intrinsic IDs. The complete pipeline (type checker arm → emitter arm → `IntrinsicId` variant → `execute_intrinsic` arm → `resolve_intrinsic_id` entry → virtual module `add_impl_def` + `add_intrinsic_method`) must be built for all 11 string methods (split, trim, starts_with, ends_with, contains, replace, to_upper, to_lower). The string utility methods are called as direct dot-call expressions (not via contract dispatch), so they follow the same direct-emit path used by `string.len()` and `string.into_int()`.

Host value construction (`Runtime::construct_value`) requires: a public `fn construct_value(&mut self, type_name: &str, fields: Vec<Value>) -> Result<Value, String>` on `Runtime`, which searches the user module's `type_defs` by name, validates field count, does a shallow value-kind check per field, and allocates on the heap. `ExternHandler::ImmediateWithHeap` requires a new variant of the `ExternHandler` enum that passes `&mut dyn GcHeap` to the closure so the handler can call `construct_value`-like logic itself.

**Primary recommendation:** Implement each workstream as a separate plan (array compiler wiring, string intrinsics, host construction API) in dependency order: string intrinsics and array wiring are independent; host API is also independent.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `writ-module` (internal) | workspace | IL instruction set, module builder | All instruction definitions live here |
| `writ-compiler` (internal) | workspace | Type checker + emitter | Dot-call resolution lives in `check_expr/access.rs` + `emit/body/expr/builtins.rs` |
| `writ-runtime` (internal) | workspace | VM dispatch, intrinsic execution, public API | `dispatch/mod.rs`, `dispatch/intrinsics.rs`, `domain_dispatch.rs`, `virtual_module.rs`, `runtime.rs`, `extern_registry.rs` |
| `std::str` (Rust stdlib) | stable | Unicode-correct string operations | As mandated by STATE.md |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `rustc_hash::FxHashMap` | workspace | Fast dispatch table | Already used throughout |

**Version verification:** All libraries are workspace-internal; no external package versions to check.

## Architecture Patterns

### Existing Dot-Call Pipeline (must follow exactly)

Every built-in method on a primitive type goes through four layers. They must ALL be updated together or the method will be invisible, mistyped, unresolvable, or crash:

```
Layer 1: writ-compiler/src/check/check_expr/access.rs
         TyKind::Array(_) arm or TyKind::String arm
         → adds the method name and returns a Func type

Layer 2: writ-compiler/src/emit/body/expr/builtins.rs
         TyKind::Array(_) arm or TyKind::String arm in try_emit_builtin_method
         → emits the specific Instruction or CALL_VIRT

Layer 3: writ-runtime/src/dispatch/intrinsics.rs  (if using contract dispatch path)
         OR writ-runtime/src/dispatch/objects.rs / arith.rs (if direct opcode)
         → execute_intrinsic match arm

Layer 4 (if using contract dispatch path):
         writ-runtime/src/domain_dispatch.rs resolve_intrinsic_id()
         writ-runtime/src/virtual_module.rs add_impl_def + add_intrinsic_method
         writ-runtime/src/dispatch/mod.rs IntrinsicId enum
```

### Two Emission Strategies

**Strategy A — Direct opcode** (used by `ArrayAdd`, `ArrayLen`, `ArraySlice`, `StrLen`):
- Emitter directly emits `Instruction::ArrayAdd { r_arr, r_val }` etc.
- No contract dispatch; no IntrinsicId; no virtual_module entry needed.
- Requires the opcode to already exist in `writ-module/src/instruction.rs`.
- Used when the operation has no meaningful Eq/contract dependency.

**Strategy B — Intrinsic via contract dispatch** (used by `StringAdd`, `ArrayIndex`, `IntEq`, etc.):
- Emitter emits `CALL_VIRT` with contract/slot information.
- Runtime resolves to `DispatchTarget::Intrinsic(id)` via dispatch table.
- Requires all 4 layers including virtual_module entries.
- Used when the operation conceptually implements a contract (Hashable, Eq, etc.) or is needed via the contract path.

**Decision for this phase:**

| Method | Strategy | Rationale |
|--------|----------|-----------|
| `arr.add(x)` | A — `ArrayAdd` opcode exists | Already in opcodes and runtime |
| `arr.remove_at(i)` | A — `ArrayRemove` opcode exists | Already in opcodes and runtime |
| `arr.insert(i, x)` | A — `ArrayInsert` opcode exists | Already in opcodes and runtime |
| `arr.contains(x)` | A — new `ArrayContains` opcode needed | No Eq contract dispatch required in this phase; simple linear scan comparing Value bytes |
| `arr.slice(start, end)` | A — `ArraySlice` opcode exists (partial, 2 args) | Already wired for push, extend for slice |
| `s.split(sep)` | B — new intrinsic via `StringSplit` IntrinsicId | No opcode; must allocate heap array |
| `s.trim()` | A — new `StrTrim` opcode | No args, no heap alloc, simple string replacement |
| `s.starts_with(p)` | A — new `StrStartsWith` opcode | Returns bool; two-register pattern |
| `s.ends_with(s)` | A — new `StrEndsWith` opcode | Same as starts_with |
| `s.contains(sub)` | A — new `StrContains` opcode | Returns bool |
| `s.replace(from, to)` | B — new intrinsic | Must allocate heap string; 3-register |
| `s.to_upper()` | A — new `StrToUpper` opcode | Returns new string heap allocation |
| `s.to_lower()` | A — new `StrToLower` opcode | Same |

**Revised: use direct IL opcodes wherever possible.** String methods that return new strings (`split`, `replace`) need heap allocation, making them inherently non-trivial, so implement them as new opcodes dispatched in `dispatch/arith.rs` (matching the `exec_str_len` pattern). The `split` method returns an array, so it allocates on the heap via `heap.alloc_array` + multiple `heap.alloc_string` calls. All can be direct opcodes rather than full contract-dispatch intrinsics — this is simpler and more consistent with how `StrLen` was done.

### New Opcodes Required

Add to `writ-module/src/instruction.rs` in the 0x08 String section or a new 0x08 extended range:

```rust
// Source: direct inspection of writ-module/src/instruction.rs string section
/// StrTrim: trim whitespace — Shape RR (6B)
StrTrim { r_dst: u16, r_src: u16 },
/// StrToUpper: to_ascii_uppercase — Shape RR (6B)
StrToUpper { r_dst: u16, r_src: u16 },
/// StrToLower: to_ascii_lowercase — Shape RR (6B)
StrToLower { r_dst: u16, r_src: u16 },
/// StrStartsWith: returns bool — Shape RRR (8B)
StrStartsWith { r_dst: u16, r_str: u16, r_prefix: u16 },
/// StrEndsWith: returns bool — Shape RRR (8B)
StrEndsWith { r_dst: u16, r_str: u16, r_suffix: u16 },
/// StrContains: returns bool — Shape RRR (8B)
StrContains { r_dst: u16, r_str: u16, r_sub: u16 },
/// StrSplit: returns string[] heap ref — Shape RRR (8B)
StrSplit { r_dst: u16, r_str: u16, r_sep: u16 },
/// StrReplace: replace all occurrences — Shape RR+R (10B, var)
StrReplace { r_dst: u16, r_str: u16, r_from: u16, r_to: u16 },
/// ArrayContains: returns bool, Value byte equality — Shape RRR (8B)
ArrayContains { r_dst: u16, r_arr: u16, r_val: u16 },
```

**Opcode number assignment:** Inspect the existing opcode space — strings currently use 0x0800-range; arrays use 0x09xx. Assign new string opcodes sequentially after `StrBuild` (0x0806 is the last used string opcode based on `StrLen` = 0x0806 pattern — verify exact numbers during implementation). Assign `ArrayContains` after `ArraySlice` (0x0908 → new 0x0909).

### Recommended Project Structure (no changes to directory layout)

Existing files to modify:
```
writ-module/src/instruction.rs       -- add new Instruction variants, opcode constants, encode/decode
writ-compiler/src/check/check_expr/access.rs  -- add methods to TyKind::String and TyKind::Array arms
writ-compiler/src/emit/body/expr/builtins.rs  -- add emit arms for new methods
writ-runtime/src/dispatch/mod.rs     -- add IntrinsicId variants (if any), route new instructions
writ-runtime/src/dispatch/arith.rs   -- add exec_str_* functions
writ-runtime/src/dispatch/objects.rs -- add exec_array_contains
writ-runtime/src/runtime.rs          -- add Runtime::construct_value public method
writ-runtime/src/extern_registry.rs  -- add ExternHandler::ImmediateWithHeap variant
writ-runtime/src/lib.rs              -- re-export new public types if needed
```

### Array Method Compiler Wiring

Current state of `TyKind::Array(elem_ty)` arm in `access.rs`:
- Knows: `len`, `push`, `slice`
- Missing: `add` (alias to push), `remove_at`, `insert`, `contains`

The field name `add` is the user-facing name (spec says `arr.add(x)`), while the emitter currently has `push` wired to `ArrayAdd`. The checker must accept both or the user-facing name must be `add`.

**Decision:** Use `add` as the canonical name. Remove `push` or alias it. The existing `push` arm in `builtins.rs` emits `ArrayAdd` — rename the match arm string from `"push"` to `"add"` in both the checker and emitter.

```rust
// access.rs — TyKind::Array(elem_ty) arm (verified from source)
"len"       => ctx.interner.func(vec![], int_ty),
"add"       => ctx.interner.func(vec![elem_ty], void_ty),
"remove_at" => ctx.interner.func(vec![int_ty], void_ty),
"insert"    => ctx.interner.func(vec![int_ty, elem_ty], void_ty),
"contains"  => ctx.interner.func(vec![elem_ty], bool_ty),
"slice"     => {
    let arr_ty = ctx.interner.intern(TyKind::Array(elem_ty));
    ctx.interner.func(vec![int_ty, int_ty], arr_ty)
}
```

```rust
// builtins.rs — TyKind::Array arm (verified from source)
"add" if args.len() == 1 => {
    let r_val = emit_expr(emitter, &args[0]);
    emitter.emit(Instruction::ArrayAdd { r_arr, r_val });
    let r_dst = emitter.alloc_reg(ty);
    Some(r_dst)
}
"remove_at" if args.len() == 1 => {
    let r_idx = emit_expr(emitter, &args[0]);
    emitter.emit(Instruction::ArrayRemove { r_arr, r_idx });
    let r_dst = emitter.alloc_reg(ty);
    Some(r_dst)
}
"insert" if args.len() == 2 => {
    let r_idx = emit_expr(emitter, &args[0]);
    let r_val = emit_expr(emitter, &args[1]);
    emitter.emit(Instruction::ArrayInsert { r_arr, r_idx, r_val });
    let r_dst = emitter.alloc_reg(ty);
    Some(r_dst)
}
"contains" if args.len() == 1 => {
    let r_val = emit_expr(emitter, &args[0]);
    let r_dst = emitter.alloc_reg(ty);
    emitter.emit(Instruction::ArrayContains { r_dst, r_arr, r_val });
    Some(r_dst)
}
"slice" if args.len() == 2 => {
    // already exists, keep as-is
}
```

### String Method Compiler Wiring

Current state of `TyKind::String` arm in `access.rs`:
- Knows: `len`, `into_string`, `into_int`, `into_float`, `into_bool`
- Missing all 8 new utility methods

```rust
// access.rs additions (verified pattern from existing string arm)
"split"       => ctx.interner.func(vec![string_ty], arr_string_ty),  // string[]
"trim"        => ctx.interner.func(vec![], string_ty),
"starts_with" => ctx.interner.func(vec![string_ty], bool_ty),
"ends_with"   => ctx.interner.func(vec![string_ty], bool_ty),
"contains"    => ctx.interner.func(vec![string_ty], bool_ty),
"replace"     => ctx.interner.func(vec![string_ty, string_ty], string_ty),
"to_upper"    => ctx.interner.func(vec![], string_ty),
"to_lower"    => ctx.interner.func(vec![], string_ty),
```

Note: `arr_string_ty` is `ctx.interner.intern(TyKind::Array(string_ty))`.

### Hashable Contract

STATE.md decision: "`Hashable` builtin contract registered in virtual module, auto-implemented for `int`, `string`, `bool`, `float` primitives only".

Hashable is needed by Phase 117 (HashMap, HashSet). This phase registers it but does not require the compiler to enforce it — that's Phase 115 territory. The work here is:

1. Add `hashable_contract = builder.add_contract_def("Hashable", "writ")` in `virtual_module.rs` Section 1
2. Add `builder.add_contract_method("hash", &[], 0)` (single method: `fn hash() -> int`)
3. Add `add_impl_def(int_type, hashable_contract)` + `add_intrinsic_method("int_hash")`
4. Repeat for float, bool, string
5. Add `IntrinsicId::IntHash`, `FloatHash`, `BoolHash`, `StringHash` variants
6. Add entries in `resolve_intrinsic_id`: `("Int", "int_hash") => Some(IntrinsicId::IntHash)` etc.
7. Add `execute_intrinsic` arms implementing actual hash computations (see below)

Hash implementations (locale-independent, deterministic):
- `IntHash`: identity cast `value as u64` → pack back as `Value::Int`
- `FloatHash`: `f64::to_bits()` → `Value::Int`
- `BoolHash`: `false → 0, true → 1` → `Value::Int`
- `StringHash`: `std::hash::DefaultHasher` or FNV-1a for determinism (DefaultHasher is non-deterministic across runs; use FNV-1a or a fixed seed)

**Caution:** `std::collections::hash_map::DefaultHasher` is not deterministic across Rust versions/runs. Use a simple fixed hash (FNV-1a) for game script use where reproducibility matters.

### Host Value Construction API

Three new public items in `writ-runtime`:

#### 1. `Runtime::construct_value`

```rust
// runtime.rs — new public method
pub fn construct_value(
    &mut self,
    type_name: &str,
    fields: Vec<Value>,
) -> Result<Value, String> {
    // 1. Find type in user module by name
    let user_module = &self.domain.modules[self.user_module_idx];
    let typedef_idx = user_module.module.type_defs.iter().position(|td| {
        writ_module::heap::read_string(&user_module.module.string_heap, td.name)
            .ok() == Some(type_name)
    }).ok_or_else(|| format!("type '{}' not found in module", type_name))?;

    let td = &user_module.module.type_defs[typedef_idx];
    // 2. Get field count from module metadata
    let field_start = td.field_list.saturating_sub(1) as usize;
    let field_end = if typedef_idx + 1 < user_module.module.type_defs.len() {
        user_module.module.type_defs[typedef_idx + 1].field_list.saturating_sub(1)
    } else {
        user_module.module.field_defs.len()
    };
    let expected_field_count = field_end - field_start;

    // 3. Validate field count
    if fields.len() != expected_field_count {
        return Err(format!(
            "type '{}' has {} fields but {} were provided",
            type_name, expected_field_count, fields.len()
        ));
    }

    // 4. Shallow type validation per field (optional but specified by HOST-03)
    // Check each field's type tag against metadata field_defs
    // (type tag byte 0x01=int, 0x02=float, 0x03=bool, 0x04=string, etc.)
    // Only validate discriminant kind; skip for Void (uninitialized)
    for (i, (field_val, field_idx)) in fields.iter().zip(field_start..field_end).enumerate() {
        let fd = &user_module.module.field_defs[field_idx];
        // fd.signature[0] is the type tag
        if let Some(tag) = fd.signature.first() {
            let kind_ok = match (tag, field_val) {
                (0x01, Value::Int(_)) | (0x01, Value::Void) => true,
                (0x02, Value::Float(_)) | (0x02, Value::Void) => true,
                (0x03, Value::Bool(_)) | (0x03, Value::Void) => true,
                (0x04, Value::Ref(_)) | (0x04, Value::Void) => true, // string is a heap ref
                _ => true, // for complex types (struct refs, arrays), accept any ref
            };
            if !kind_ok {
                return Err(format!(
                    "field {} of type '{}': type mismatch",
                    i, type_name
                ));
            }
        }
    }

    // 5. Allocate on heap
    let kind = writ_module::TypeDefKind::from_u8(td.kind);
    match kind {
        Some(writ_module::TypeDefKind::Struct) => {
            let href = self.heap.alloc_struct(u32::MAX, expected_field_count);
            // write fields into the struct
            for (i, val) in fields.into_iter().enumerate() {
                self.heap.set_field(href, i, val).ok();
            }
            let type_token = (2u32 << 24) | ((typedef_idx as u32) + 1);
            Ok(Value::Struct { type_idx: type_token, href })
        }
        Some(writ_module::TypeDefKind::Class) => {
            let class_type_key = ((self.user_module_idx as u32) << 16) | (typedef_idx as u32);
            let href = self.heap.alloc_struct(class_type_key, expected_field_count);
            for (i, val) in fields.into_iter().enumerate() {
                self.heap.set_field(href, i, val).ok();
            }
            Ok(Value::Ref(href))
        }
        _ => Err(format!("type '{}' is not a struct or class", type_name)),
    }
}
```

Note: `BumpHeap` and `GcHeap` need a `set_field` method if it doesn't exist. Verify in `heap.rs` — `get_field` exists; `set_field` may need to be added.

#### 2. `ExternHandler::ImmediateWithHeap`

```rust
// extern_registry.rs — new variant
pub enum ExternHandler {
    Immediate(Box<dyn FnMut(&[Value]) -> Result<Value, String> + Send + Sync>),
    ImmediateWithHeap(Box<dyn FnMut(&[Value], &mut dyn GcHeap) -> Result<Value, String> + Send + Sync>),
    Deferred,
}
```

The dispatch loop in `extern_registry.rs` `on_request` for `HostRequest::ExternCall` must handle the new variant:

```rust
ExternHandler::ImmediateWithHeap(f) => {
    // Heap access not available from on_request (only &mut self)
    // This variant requires the host to hold a separate heap reference
    // OR the ExternHost must hold a heap reference
}
```

**Critical design issue:** `on_request` only receives `id: RequestId` and `req: &HostRequest` — it does not receive the heap. The current dispatch path runs through `RuntimeHost::on_request` which has no heap access. To provide heap access to immediate handlers, the design must change.

**Two design options:**

Option A — Pass heap to `on_request`: Change `RuntimeHost::on_request` signature to include `heap: &mut dyn GcHeap`. This is a breaking change to the public trait.

Option B — New `ExternHost::dispatch_with_heap` method: Make `ExternHost` implement a separate internal method that accepts the heap. This keeps `RuntimeHost` backward compatible.

Option C — Store heap reference in `ExternHost`: Not viable (ownership issue with Runtime).

Option D — New `HostRequest` variant carrying a heap handle: Over-engineered.

**Recommended approach (Option B):** Add a secondary dispatch path specifically for `ExternHost` that accepts the heap, and wire it into the `ExternHost::on_request_with_heap(&mut self, id, req, heap)` internal method. The runtime's dispatch loop already has heap access; it can check if the host is an `ExternHost` (downcasting) or add a new `RuntimeHost` method with a default no-op implementation:

```rust
// host.rs — new optional trait method with default
fn on_extern_call_with_heap(
    &mut self,
    _id: RequestId,
    _extern_idx: u32,
    _args: &[Value],
    _heap: &mut dyn GcHeap,
) -> Option<HostResponse> {
    None // None = not handled; runtime falls through to on_request
}
```

This is backward compatible: existing hosts that don't implement it get `None`, and the runtime falls back to `on_request`. `ExternHost` overrides it to check for `ImmediateWithHeap` handlers.

The dispatch loop in `dispatch/mod.rs` for extern calls must call `host.on_extern_call_with_heap` first, then fall back to `host.on_request`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Unicode string case conversion | Custom Unicode tables | `str::to_ascii_uppercase()` / `to_ascii_lowercase()` | Game scripts are ASCII-dominant; STATE.md mandates locale independence |
| String hash function | Custom hash | FNV-1a (simple inline) | `DefaultHasher` is non-deterministic across runs; FNV-1a is ~10 lines and deterministic |
| Type-by-name lookup | New index structure | Iterate `type_defs` at construction time | Type count is small; linear scan is fine at host-call time, not hot path |
| Field-count metadata | New table | Existing `type_defs[i+1].field_list - type_defs[i].field_list` idiom | Already used in `exec_new`, `get_type_field_count` in helpers.rs |

**Key insight:** The runtime already has all array mutation opcodes wired in the dispatch loop (`dispatch/mod.rs` lines 456-462); the only missing piece is making the compiler emit those instructions for the user-facing method names.

## Common Pitfalls

### Pitfall 1: Renaming push vs. add
**What goes wrong:** The existing type checker accepts `push` but the spec says `add`. Changing only the checker without changing the emitter (or vice versa) causes a type error or "method not found" that's hard to trace.
**Why it happens:** The checker and emitter both have independent string-match arms.
**How to avoid:** Change both `access.rs` ("push" → "add") and `builtins.rs` ("push" → "add") in the same commit. Add a test that calls `arr.add(x)` before and after.
**Warning signs:** Compiler emits `UnknownField` for `add` if checker is wrong; compiler emits wrong opcode if emitter is wrong.

### Pitfall 2: String methods returning heap-allocated values
**What goes wrong:** `split` and `replace` return heap-allocated objects (array of strings / new string). The emitter must allocate a destination register for a `Value::Ref` result, and the runtime exec function must use `heap.alloc_array` / `heap.alloc_string`. Forgetting to allocate a new string on the heap returns a dangling reference or a stack-local that gets corrupted.
**Why it happens:** The simple string opcodes (`StrLen`, `StrTrim`) return scalars (Int, String where string is already heap-interned). `StrSplit` returns an array ref.
**How to avoid:** Follow the `exec_array_slice` pattern exactly — allocate new array, populate elements, return `Value::Ref(href)`. Each substring must also be individually heap-allocated via `heap.alloc_string`.
**Warning signs:** Crash "not an array" on the result; garbage results from un-allocated strings.

### Pitfall 3: Non-ASCII panic in string indexing
**What goes wrong:** Using `s[i..]` or `s[..i]` byte-slice on a UTF-8 string panics if `i` is not on a character boundary.
**Why it happens:** Rust strings are UTF-8; byte indexing into multi-byte chars panics.
**How to avoid:** Use `.chars()` iteration, `.char_indices()`, or Rust's `contains`/`starts_with`/`ends_with`/`split`/`replace` methods which handle UTF-8 correctly. Never use `&s[i..j]` directly.
**Warning signs:** Panic in the VM with "byte index N is not a char boundary". STATE.md: "non-ASCII test required for each method".

### Pitfall 4: ArrayContains needs Value equality, not address equality
**What goes wrong:** Using pointer comparison (`ptr::eq`) on `Value::Ref` would compare heap addresses, not content. `arr.contains("hello")` would always return false for string values even if the string content matches.
**Why it happens:** `Value` contains `HeapRef(u32)` which is just an index; two allocations of the same string have different HeapRef values.
**How to avoid:** For the `ArrayContains` implementation in `exec_array_contains`, extract the inner content for `Value::Ref` (string) comparisons using `heap.read_string`. For primitives (Int, Float, Bool), use the derived `PartialEq` on `Value` (already implemented). For now, implement deep equality for strings only (the common case); for struct refs, use address equality with a comment.
**Warning signs:** `arr.contains("hello")` returns false when the array contains a string with that content.

### Pitfall 5: ExternHandler::ImmediateWithHeap ownership
**What goes wrong:** If the heap-aware handler closure captures a reference to the runtime or domain, it creates a borrow cycle. The runtime owns the heap; the extern handler is called from within the runtime tick; borrow checker rejects `&mut runtime` inside a closure that the runtime calls.
**Why it happens:** The runtime calls `host.on_request` while borrowing `self.heap` and `self.domain`. If the handler tries to access the same runtime, double-borrow.
**How to avoid:** The `ImmediateWithHeap` handler receives `&mut dyn GcHeap` as a parameter — it does NOT need to reference the Runtime. It can only allocate/read heap objects. For anything beyond heap allocation (e.g., calling Writ methods), use the Deferred path instead.
**Warning signs:** "cannot borrow `runtime` as mutable more than once at a time" in test code that tries to use ImmediateWithHeap.

### Pitfall 6: Opcode numbering gaps
**What goes wrong:** Assigning a new opcode number that collides with an existing one causes silent decode errors at runtime — the decoder reads the wrong instruction bytes.
**Why it happens:** The opcode space has specific patterns (0x08xx for strings, 0x09xx for arrays). Adding a new opcode without checking all existing assignments creates a collision.
**How to avoid:** Before adding any new `Instruction` variant, enumerate all existing opcode constants in `instruction.rs` to find the next available slot. Search for `0x0807`, `0x0808` etc. in the file.
**Warning signs:** VM executes completely wrong instructions for string methods; debug by printing opcode bytes.

### Pitfall 7: set_field method missing on GcHeap/BumpHeap
**What goes wrong:** `construct_value` needs to write fields into the newly allocated struct. If `set_field` does not exist on `GcHeap` or `BumpHeap`, the code won't compile.
**Why it happens:** `get_field` exists (`heap.rs` line ~94) but `set_field` may not be exposed.
**How to avoid:** Before writing `construct_value`, verify `set_field` exists in `heap.rs`. If missing, add it following the `get_field` pattern.
**Warning signs:** Compile error "no method named `set_field` found for type `dyn GcHeap`".

## Code Examples

### Pattern: Adding a direct-opcode string method (StrTrim)

```rust
// writ-module/src/instruction.rs — new variant (opcode TBD, verify exact number)
/// 0x0807 — Shape RR (6B)
StrTrim { r_dst: u16, r_src: u16 },
```

```rust
// writ-compiler/src/check/check_expr/access.rs — TyKind::String arm addition
"trim" => ctx.interner.func(vec![], string_ty),
```

```rust
// writ-compiler/src/emit/body/expr/builtins.rs — TyKind::String arm addition
"trim" => {
    let r_dst = emitter.alloc_reg(ty);
    emitter.emit(Instruction::StrTrim { r_dst, r_src });
    return Some(r_dst);
}
```

```rust
// writ-runtime/src/dispatch/arith.rs — new exec function
pub(super) fn exec_str_trim(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let src_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_src as usize]);
    let trimmed = match ctx.heap.read_string(src_ref) {
        Ok(s) => s.trim().to_string(),
        Err(_) => return ExecutionResult::Crash("StrTrim: not a string".into()),
    };
    let href = ctx.heap.alloc_string(&trimmed);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}
```

```rust
// writ-runtime/src/dispatch/mod.rs — route the instruction
Instruction::StrTrim { r_dst, r_src } => arith::exec_str_trim(&mut ctx, *r_dst, *r_src),
```

### Pattern: StrSplit returning string[]

```rust
// writ-runtime/src/dispatch/arith.rs
pub(super) fn exec_str_split(ctx: &mut ExecContext<'_>, r_dst: u16, r_str: u16, r_sep: u16) -> ExecutionResult {
    let str_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_str as usize]);
    let sep_ref = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_sep as usize]);
    let (s, sep) = match (ctx.heap.read_string(str_ref), ctx.heap.read_string(sep_ref)) {
        (Ok(s), Ok(sep)) => (s.to_string(), sep.to_string()),
        _ => return ExecutionResult::Crash("StrSplit: not a string".into()),
    };
    let parts: Vec<String> = s.split(sep.as_str()).map(|p| p.to_string()).collect();
    // elem_type for string array: use the string type tag
    let arr_ref = ctx.heap.alloc_array(0x04); // 0x04 = string type tag
    let part_refs: Vec<Value> = parts.iter()
        .map(|p| Value::Ref(ctx.heap.alloc_string(p)))
        .collect();
    if let Ok(HeapObject::Array { elements, .. }) = ctx.heap.get_object_mut(arr_ref) {
        *elements = part_refs;
    }
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Ref(arr_ref);
    ExecutionResult::Continue
}
```

### Pattern: ExternHandler::ImmediateWithHeap usage

```rust
// Host-side game engine code (user-facing API example)
let mut registry = ExternRegistry::new();
registry.register_with_heap("create_item", |args, heap| {
    let name_ref = match args.first() {
        Some(Value::Ref(r)) => *r,
        _ => return Err("expected string arg".into()),
    };
    let name = heap.read_string(name_ref)?.to_string();
    let href = heap.alloc_struct(u32::MAX, 2);
    heap.set_field(href, 0, Value::Ref(heap.alloc_string(&name)))?;
    heap.set_field(href, 1, Value::Int(0))?;
    Ok(Value::Struct { type_idx: ITEM_TYPE_IDX, href })
});
```

### Pattern: Runtime::construct_value usage

```rust
// Game engine code constructing a Writ struct value
let item = runtime.construct_value("Item", vec![
    Value::Ref(runtime.heap_mut().alloc_string("Sword")),
    Value::Int(10),
])?;
// Pass item as task argument or set as global
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No array mutation methods (compiler) | `ArrayAdd`/`Remove`/`Insert` opcodes exist in VM but not wired in compiler | Phase 93 (opcodes defined) | Compiler must add the wiring |
| No string utilities | Only `len`, `into_*` methods | Phase 28 (initial string support) | All 8 new methods are net-new |
| No host value construction | Hosts receive `Value` but cannot construct struct values | Inception | New public API surface |

**Deprecated/outdated:**
- `push` as the array mutation method name: now `add` per spec. The word "push" appears in `builtins.rs` and `access.rs`; both must be updated.

## Open Questions

1. **Exact opcode numbers for new string/array opcodes**
   - What we know: Strings use 0x08xx range; last confirmed is `StrBuild` (0x0806); arrays use 0x09xx; `ArraySlice` is 0x0908.
   - What's unclear: Whether 0x0807/0x0808/... are already used by opcodes not visible in grep results.
   - Recommendation: At plan time, read the full `instruction.rs` opcode table sequentially to find exact next-available slots.

2. **`set_field` availability on `GcHeap` trait and `BumpHeap`**
   - What we know: `get_field` exists on `BumpHeap`. `GcHeap` is a trait.
   - What's unclear: Whether `set_field` is on the `GcHeap` trait.
   - Recommendation: At plan time, read `heap.rs` and `gc.rs` fully to check; add if missing.

3. **FNV-1a hash vs. stdlib hash for Hashable**
   - What we know: `DefaultHasher` is non-deterministic. FNV-1a is ~10 lines inline.
   - What's unclear: Whether writ-runtime already has a hash utility or depends on `rustc_hash`.
   - Recommendation: `rustc_hash::FxHasher` uses a fixed-seed hash — check if it's deterministic enough, or use a standalone FNV-1a implementation (4 lines).

4. **`slice` API change: 2-arg index vs Range**
   - What we know: Current type checker wires `slice(int, int)` → `ArraySlice` with two separate int args.
   - What's unclear: The success criteria says `arr.slice(r)` takes a single range `r`, not two ints.
   - Recommendation: The success criterion says `arr.slice(r)` (single range arg). This conflicts with the existing 2-arg `slice` wire. Either change to single-arg `Range` or treat `r` as a shorthand. Check Phase 93 spec for `ArraySlice` semantics. If the opcode takes start+end registers, the compiler can accept a Range value and unpack its fields.

## Environment Availability

Step 2.6: SKIPPED (no external tools required — all work is internal Rust crate modifications).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`) + `cargo test` |
| Config file | `Cargo.toml` workspace |
| Quick run command | `cargo test -p writ-runtime --lib 2>&1` |
| Full suite command | `cargo test --workspace 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STR-01 | `"a,b,c".split(",")` returns `["a","b","c"]` | unit | `cargo test -p writ-runtime array_str_tests::str_split` | ❌ Wave 0 |
| STR-01 | Non-ASCII: `"café,latte".split(",")` returns 2 elements | unit | `cargo test -p writ-runtime array_str_tests::str_split_unicode` | ❌ Wave 0 |
| STR-02 | `"  hello  ".trim()` returns `"hello"` | unit | `cargo test -p writ-runtime array_str_tests::str_trim` | ❌ Wave 0 |
| STR-03 | `"hello".starts_with("he")` returns true | unit | `cargo test -p writ-runtime array_str_tests::str_starts_with` | ❌ Wave 0 |
| STR-03 | `"hello".ends_with("lo")` returns true | unit | `cargo test -p writ-runtime array_str_tests::str_ends_with` | ❌ Wave 0 |
| STR-04 | `"hello world".contains("world")` returns true | unit | `cargo test -p writ-runtime array_str_tests::str_contains` | ❌ Wave 0 |
| STR-05 | `"hello".replace("l", "r")` returns `"herro"` | unit | `cargo test -p writ-runtime array_str_tests::str_replace` | ❌ Wave 0 |
| STR-06 | `"hello".to_upper()` returns `"HELLO"` | unit | `cargo test -p writ-runtime array_str_tests::str_to_upper` | ❌ Wave 0 |
| STR-06 | `"HELLO".to_lower()` returns `"hello"` | unit | `cargo test -p writ-runtime array_str_tests::str_to_lower` | ❌ Wave 0 |
| (Hashable) | `Hashable` contract registered; int/string/bool/float have hash() | unit | `cargo test -p writ-runtime domain::hashable_contract_registered` | ❌ Wave 0 |
| HOST-01 | `runtime.construct_value("Point", vec![Int(1), Int(2)])` returns struct | unit | `cargo test -p writ-runtime runtime::construct_value_valid` | ❌ Wave 0 |
| HOST-02 | `ExternHandler::ImmediateWithHeap` closure allocates a string on heap | unit | `cargo test -p writ-runtime extern_registry::immediate_with_heap` | ❌ Wave 0 |
| HOST-03 | `construct_value` with wrong field count returns `Err(...)` | unit | `cargo test -p writ-runtime runtime::construct_value_wrong_count` | ❌ Wave 0 |
| HOST-03 | `construct_value` with unknown type name returns `Err(...)` | unit | `cargo test -p writ-runtime runtime::construct_value_unknown_type` | ❌ Wave 0 |
| Compiler | `arr.add(x)` compiles without error; `arr.push(x)` now errors | integration | `cargo test -p writ-compiler arr_method_add_compiles` | ❌ Wave 0 |
| Compiler | `s.trim()` compiles and emits correct instruction | integration | `cargo test -p writ-compiler str_trim_compiles` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime --lib 2>&1 | tail -5`
- **Per wave merge:** `cargo test --workspace 2>&1 | tail -10`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/array_str_tests.rs` — covers STR-01..06 runtime exec tests
- [ ] `writ-runtime/tests/host_construction_tests.rs` — covers HOST-01..03
- [ ] `writ-compiler` tests for new array/string method compilation (can add to existing `check_expr` tests)

*(Existing test infrastructure covers opcodes but not the new methods)*

## Sources

### Primary (HIGH confidence)
- Direct source inspection: `writ-compiler/src/check/check_expr/access.rs` — TyKind::Array and TyKind::String arms verified
- Direct source inspection: `writ-compiler/src/emit/body/expr/builtins.rs` — emit pattern verified
- Direct source inspection: `writ-module/src/instruction.rs` — all opcode assignments 0x0900-0x0908 verified
- Direct source inspection: `writ-runtime/src/dispatch/objects.rs` — exec_array_* functions verified present
- Direct source inspection: `writ-runtime/src/dispatch/arith.rs` — exec_str_len pattern verified
- Direct source inspection: `writ-runtime/src/dispatch/intrinsics.rs` — IntrinsicId enum verified
- Direct source inspection: `writ-runtime/src/domain_dispatch.rs` — resolve_intrinsic_id full table verified
- Direct source inspection: `writ-runtime/src/virtual_module.rs` — array_add/array_remove_at/array_contains already registered as intrinsic methods in Section 5
- Direct source inspection: `writ-runtime/src/extern_registry.rs` — ExternHandler::Immediate signature `&[Value]` confirmed, no heap access
- Direct source inspection: `writ-runtime/src/runtime.rs` — Runtime public API, no construct_value method exists yet
- Direct source inspection: `.planning/STATE.md` — locked decisions: `to_ascii_uppercase/lowercase`, Rust intrinsics for string utils, Hashable registration scope

### Secondary (MEDIUM confidence)
- Rust std `str::trim()`, `str::starts_with()`, `str::ends_with()`, `str::contains()`, `str::replace()`, `str::to_ascii_uppercase()`, `str::to_ascii_lowercase()`, `str::split()` — all stable API, HIGH confidence from Rust stdlib docs

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries are internal, directly inspected
- Architecture: HIGH — exact file locations, function signatures, opcode numbers verified from source
- Pitfalls: HIGH — based on direct source inspection of existing patterns and explicit STATE.md warnings
- Host API design: MEDIUM — the `ImmediateWithHeap` ownership issue requires design decision at plan time (Option A vs B detailed above)

**Research date:** 2026-03-29
**Valid until:** 2026-04-29 (stable internal codebase, no external dependencies)
