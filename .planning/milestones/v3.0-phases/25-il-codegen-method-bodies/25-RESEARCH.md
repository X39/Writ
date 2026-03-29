# Phase 25: IL Codegen — Method Bodies - Research

**Researched:** 2026-03-03
**Domain:** Compiler IR-to-bytecode lowering; binary module serialization; register allocation
**Confidence:** HIGH

## Summary

Phase 25 lowers every `TypedDecl` body (functions, methods, impl methods, closures, lifecycle hooks) into spec-compliant IL instruction sequences and produces a complete `.writil` binary module as a `Vec<u8>`. The phase consumes the populated `ModuleBuilder` from Phase 24 (all 21 metadata tables, `DefId→MetadataToken` map) and the `TypedAst`/`TyInterner` from Phase 23.

The key architectural discovery is that `writ-module` — an existing crate in the workspace — already contains a complete, tested `Instruction` enum covering all 90 opcodes with `encode()` serialization, a `Module` struct with `to_bytes()` binary writer, and a `MethodBody` type with `DebugLocal`/`SourceSpan` fields. Phase 25 should add `writ-module` as a dependency of `writ-compiler` and use `Instruction` directly rather than defining a duplicate enum in the `emit/` module. The `writ-compiler` `ModuleBuilder` (Phase 24) will need a translation layer into `writ-module::Module` for serialization, OR the codegen will write bodies into a new `writ-module::ModuleBuilder` that can then call `.to_bytes()`.

Register allocation is simple: LIFO high-watermark (unlimited virtual registers, allocated sequentially). No liveness analysis needed — the spec explicitly defers register reuse to JIT. Branch targets use symbolic labels (a `u32` label ID resolved to byte offsets in a fixup pass) since branch offsets are relative byte counts that can only be computed after instruction sizes are known.

**Primary recommendation:** Add `writ-module` as a dependency of `writ-compiler`, use `writ_module::Instruction` for the typed instruction representation, and use `writ_module::Module::to_bytes()` for serialization. The codegen's job is to produce `Vec<Instruction>` per method body, plus register type tables, debug info, and then hand the completed `writ_module::Module` to `to_bytes()`.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Binary Serialization Scope:**
- Phase 25 includes full binary serialization — the output is a complete `.writil` module, not just in-memory instruction buffers
- Full spec-compliant 200-byte header with magic bytes, version, all 21 table offsets/counts, heap sizes
- `emit()` returns `Vec<u8>` — file I/O is a CLI concern (writ-cli), not codegen's job
- Method bodies stored in a single contiguous byte stream; `MethodDefRow.body_offset` is relative to stream start

**Instruction Representation:**
- Typed `Instruction` enum with all 90 opcodes — codegen produces `Vec<Instruction>` per method body
- Instructions are serialized to raw bytes as a final step after all codegen is complete
- Enum provides type safety, testability, and enables validation passes over instruction sequences before serialization

**Error Node Handling:**
- Abort entire module: if any `TypedExpr::Error` or `TypedStmt::Error` nodes exist anywhere, produce no `.writil` output
- Pre-pass check: before any codegen work, scan the TypedAst for Error nodes; if found, return immediately with diagnostics (no wasted work)
- Pipeline short-circuit: the compiler pipeline should not invoke `emit()` at all if the type checker reported any errors; the pre-pass check is a safety net, not the primary guard

**Debug Info Granularity:**
- SourceSpan: per-statement granularity — one entry per source statement/expression that generates instructions; sub-expression instructions inherit the parent span
- DebugLocal: ALL registers get entries, including compiler temporaries — temps get synthetic names; user-declared variables (let bindings, params, for-loop bindings, match arms) get their source names
- Flag-controlled: debug info emission is controlled by a compiler flag (future `--debug`), but default is debug-on — during Phase 25 all codegen produces debug info unless explicitly disabled

### Claude's Discretion

- Branch/jump target representation: symbolic labels vs pre-computed offsets — Claude picks what fits the instruction set best
- Register allocator type tracking: at allocation time vs inferred from usage
- Whether Instruction enum lives in emit module (compiler-only) or a shared crate — Claude picks based on crate dependency graph
- Whether codegen emits its own warnings (e.g., constant folding revealing dead branches) is at Claude's discretion — semantic warnings like dead code and unused vars remain the type checker's responsibility

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| EMIT-07 | Compiler assigns register indices via linear allocation and emits per-register TypeRef table | Register allocator design; `MethodBody.register_types: Vec<u32>` (blob offsets); `TypeSigBuilder` reusable from Phase 24 |
| EMIT-08 | Compiler emits all arithmetic, logic, comparison, data movement, and control flow instructions | `writ_module::Instruction` variants; `TyKind` dispatch for typed ops (INT vs FLOAT); BinaryOp/PrefixOp mapping |
| EMIT-09 | Compiler emits CALL, CALL_VIRT, CALL_EXTERN, and CALL_INDIRECT with correct dispatch selection | Call dispatch decision tree; CALL_VIRT specialization via static type |
| EMIT-10 | Compiler emits NEW, GET_FIELD, SET_FIELD for struct construction and field access | `TypedExpr::New` for structs vs entities; field_idx from MetadataToken |
| EMIT-11 | Compiler emits SPAWN_ENTITY, INIT_ENTITY, DESTROY_ENTITY, ENTITY_IS_ALIVE, GET_COMPONENT, GET_OR_CREATE, FIND_ALL per spec entity construction sequence | §14.7.5 / §2.16.7 entity sequence; only explicitly-provided fields get SET_FIELD |
| EMIT-12 | Compiler emits all 9 array instructions | `TypedExpr::ArrayLit` → ARRAY_INIT; index → ARRAY_LOAD/STORE; Range expressions |
| EMIT-13 | Compiler emits all 10 Option/Result instructions | WRAP_SOME/IS_NONE/UNWRAP etc.; `TyKind::Option`/`Result` dispatch |
| EMIT-14 | Compiler emits closure/delegate with compiler-generated capture struct TypeDef, method body, and NEW_DELEGATE | Lambda lowering; capture struct TypeDef must be added to ModuleBuilder before finalize (or handled in a fixup) |
| EMIT-15 | Compiler emits SPAWN_TASK, SPAWN_DETACHED, JOIN, CANCEL, DEFER_PUSH/POP/END | `TypedExpr::Spawn`/`SpawnDetached`/`Join`/`Cancel`/`Defer` → concurrency instructions |
| EMIT-16 | Compiler emits ATOMIC_BEGIN/ATOMIC_END for atomic blocks | `TypedStmt::Atomic` → wrapping with ATOMIC_BEGIN/END |
| EMIT-17 | Compiler emits GET_TAG + SWITCH + EXTRACT_FIELD for enum pattern matching | `TypedExpr::Match` on `TyKind::Enum` → GET_TAG + SWITCH; `TypedPattern::EnumVariant` → EXTRACT_FIELD |
| EMIT-18 | Compiler emits NEW_ENUM with correct variant tag and payload registers | Variant tag = declaration order index; payload fields from TypedExpr args |
| EMIT-19 | Compiler emits I2F, F2I, I2S, F2S, B2S, CONVERT for type conversions | Type conversion detection from AST method call `.into<T>()` or explicit Cast nodes |
| EMIT-20 | Compiler emits STR_CONCAT, STR_BUILD, STR_LEN for string operations | String binary op `+` → STR_CONCAT; format strings → STR_BUILD; `.len()` → STR_LEN |
| EMIT-21 | Compiler emits BOX/UNBOX at generic call sites where value types are passed | Boxing at CALL sites when TyKind is Int/Float/Bool/Enum and param is generic TyKind::GenericParam |
| EMIT-23 | Compiler emits IS_NONE/IS_ERR + early return sequences for `?` and `try` propagation | Desugared `?`/`try` are already `TypedExpr::Match` nodes — handle as special match form |
| EMIT-24 | Compiler emits TAIL_CALL for dialogue transition returns | Dialogue `->` transitions emit TAIL_CALL; needs to be recognized at the AST level |
| EMIT-26 | Compiler emits SourceSpan and DebugLocal entries for debug info | `MethodBody.debug_locals`/`source_spans`; per-statement span tracking; synthetic register names |
| EMIT-27 | Compiler specializes CALL_VIRT to CALL when receiver's static type is known concrete | Static type check at call site: if `TyKind::Struct`/`Entity`, emit CALL; if generic, emit CALL_VIRT |
| EMIT-28 | Compiler folds constant arithmetic expressions in `const` declarations | `TypedDecl::Const` with `TypedExpr::Binary` of literals → fold to `TypedExpr::Literal` before emit |
</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `writ-module` | workspace | `Instruction` enum, `Module` struct, `to_bytes()` writer, `MethodBody` | Already implemented in workspace — complete, tested, spec-compliant |
| `writ-compiler` (internal) | — | `TypedAst`, `TyInterner`, `ModuleBuilder` (Phase 24 output) | The input to this phase |
| `writ-diagnostics` | workspace | Error/warning reporting | Established pattern from prior phases |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `rustc-hash` (FxHashMap) | 2.1.1 | Register name→index maps, label→offset fixup tables | Already in `writ-compiler` Cargo.toml |
| `byteorder` | 1.5 | Already used by `writ-module` for serialization | Transitive via `writ-module` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `writ-module::Instruction` | New `Instruction` enum in `emit/instructions.rs` | Duplicates an existing tested implementation; violates DRY; synchronization burden |
| `writ-module::Module::to_bytes()` | Custom binary serializer in `emit/` | The writer is already tested with round-trip tests; prefer reuse |
| Symbolic label fixup pass | Pre-computed byte offsets | Labels are necessary because instruction sizes vary (2B–14B); can't know offsets until all instructions for a block are sized |

**Installation / Cargo change:**
```toml
# writ-compiler/Cargo.toml — add:
writ-module = { path = "../writ-module" }
```

---

## Architecture Patterns

### Recommended Module Structure

```
writ-compiler/src/emit/
├── mod.rs              # Entry point — emit() now returns Vec<u8>
├── metadata.rs         # (existing) MetadataToken, TableId, row structs
├── heaps.rs            # (existing) StringHeap, BlobHeap
├── type_sig.rs         # (existing) Ty-to-TypeRef blob encoding
├── module_builder.rs   # (existing) Phase 24 ModuleBuilder
├── slots.rs            # (existing) CALL_VIRT slot assignment
├── collect.rs          # (existing) TypedAst → ModuleBuilder pass
├── body/               # NEW — method body emission
│   ├── mod.rs          # BodyEmitter, emit_all_bodies()
│   ├── reg_alloc.rs    # RegisterAllocator (linear LIFO)
│   ├── labels.rs       # LabelAllocator + fixup pass
│   ├── expr.rs         # emit_expr() — TypedExpr → Vec<Instruction>
│   ├── stmt.rs         # emit_stmt() — TypedStmt → Vec<Instruction>
│   ├── call.rs         # CALL/CALL_VIRT/CALL_EXTERN dispatch
│   ├── patterns.rs     # Match arm lowering (enum, option, result)
│   ├── closure.rs      # Lambda → capture struct TypeDef + NEW_DELEGATE
│   ├── debug.rs        # SourceSpan + DebugLocal emission
│   └── const_fold.rs   # Constant arithmetic folding
└── serialize.rs        # NEW — translate ModuleBuilder → writ_module::Module → Vec<u8>
```

### Pattern 1: Register Allocator (LIFO High-Watermark)

**What:** Registers are allocated sequentially. Each new value gets the next unused register index. No reuse. `reg_count` = peak register index + 1.

**When to use:** All register allocation — params, locals, temporaries.

**Example:**
```rust
// Source: writ-compiler/src/emit/body/reg_alloc.rs
pub struct RegisterAllocator {
    next: u16,
    types: Vec<Ty>, // parallel: types[i] is the Ty for register i
}

impl RegisterAllocator {
    pub fn alloc(&mut self, ty: Ty) -> u16 {
        let r = self.next;
        self.next += 1;
        self.types.push(ty);
        r
    }

    pub fn reg_count(&self) -> u16 {
        self.next
    }

    pub fn types(&self) -> &[Ty] {
        &self.types
    }
}
```

**Register layout per function:**
- `r0..rN-1` = parameters (N = param count, including self if present)
- `rN..` = locals and temporaries allocated on demand

### Pattern 2: Symbolic Label Fixup (Two-Pass Branch Resolution)

**What:** Emit instructions with placeholder branch offsets, collect fixup sites, resolve after all instructions are sized.

**When to use:** Any control flow that requires forward branches (if/else, loops, match, `?`/`try`).

**Why needed:** Branch offsets in the spec are relative byte offsets (`i32`), but instruction sizes range from 2B to 14B (SWITCH is variable). Can't compute a forward branch offset until all intervening instructions are emitted.

**Example:**
```rust
pub struct Label(pub u32); // unique label ID

pub struct LabelAllocator {
    next: u32,
    // Fixups: (byte_offset_of_branch_instruction, label_id, fixup_kind)
    fixups: Vec<(usize, Label, FixupKind)>,
    // Resolved: label_id -> byte offset in instruction stream
    resolved: FxHashMap<u32, usize>,
}

enum FixupKind { BrOffset, BrTrueOffset, BrFalseOffset, SwitchEntry(usize) }

impl LabelAllocator {
    pub fn new_label(&mut self) -> Label { ... }
    pub fn mark_here(&mut self, label: Label, byte_pos: usize) { ... }
    pub fn add_fixup(&mut self, byte_pos: usize, label: Label, kind: FixupKind) { ... }
    pub fn apply_fixups(&mut self, code: &mut Vec<u8>) { ... }
}
```

**Flow:**
1. Emit `Instruction::BrFalse { r_cond, offset: 0 }` (placeholder offset)
2. Record fixup: (instruction_byte_start, else_label, BrFalseOffset)
3. Emit then-branch instructions
4. Emit `Instruction::Br { offset: 0 }` (placeholder)
5. Call `labels.mark_here(else_label, code.len())`
6. Emit else-branch instructions
7. After all instructions serialized, call `apply_fixups()` to patch offsets

### Pattern 3: BodyEmitter Context

**What:** Per-method codegen state struct carrying all context needed to emit one method body.

**Example:**
```rust
pub struct BodyEmitter<'a> {
    builder: &'a ModuleBuilder,
    interner: &'a TyInterner,
    labels: LabelAllocator,
    regs: RegisterAllocator,
    instructions: Vec<Instruction>,
    // name -> register mapping for locals
    locals: FxHashMap<String, u16>,
    // debug info accumulators
    source_spans: Vec<(u32, SimpleSpan)>, // (pc, span)
    debug_locals: Vec<(u16, String, u32, u32)>, // (reg, name, start_pc, end_pc)
}
```

### Pattern 4: Call Dispatch Decision Tree

**What:** Selecting the correct call instruction based on callee type.

```
TypedExpr::Call { callee, args, .. }
  ├─ callee is TypedExpr::Field { receiver, field } with receiver type = Struct/Entity:
  │    ├─ field method is in an impl block for a contract?
  │    │    ├─ receiver static type is concrete (TyKind::Struct/Entity) → CALL (EMIT-27 specialization)
  │    │    └─ receiver is generic T: SomeContract → CALL_VIRT
  │    └─ regular method (no contract) → CALL
  ├─ callee resolves to ExternDef → CALL_EXTERN
  ├─ callee is a delegate (TyKind::Func) → CALL_INDIRECT
  └─ callee is a free function (MethodDef) → CALL
```

### Pattern 5: Entity Construction Sequence (EMIT-11)

Per spec §2.16.7, `new Guard { name: "Steve" }` emits:
```
SPAWN_ENTITY  r_dst, Guard_type_token
LOAD_STRING   r_name, "Steve"_idx
SET_FIELD     r_dst, name_field_token, r_name
INIT_ENTITY   r_dst
```

**Critical rule:** Only fields explicitly listed in `TypedExpr::New { fields }` get `SET_FIELD` instructions. Default field values do NOT generate `SET_FIELD`. `INIT_ENTITY` fires `on_create`.

### Pattern 6: Closure / Lambda Lowering (EMIT-14)

For `TypedExpr::Lambda { params, captures, body, .. }`:
1. **During body codegen pass:** When a Lambda is encountered, synthesize a compiler-generated `TypeDef` for the capture struct (e.g., `__closure_0`). Add it to the `ModuleBuilder` via `add_typedef` + `add_fielddef` per capture. Also generate a synthetic `MethodDef` for the closure body.
2. **Emit closure body** as a separate method body (recursive call to body emitter).
3. **At the lambda site:**
   ```
   NEW          r_env, __closure_0_type_token    // allocate capture struct
   MOV          r_tmp, r_captured_var            // (or GET_FIELD for by-ref)
   SET_FIELD    r_env, field_token, r_tmp        // copy captured value
   NEW_DELEGATE r_delegate, closure_method_token, r_env
   ```
4. **If no captures:** `NEW_DELEGATE r_delegate, closure_method_token, r_null_reg`

**Problem:** Lambda TypeDefs must be registered in `ModuleBuilder` BEFORE `finalize()` is called, but Phase 24's `finalize()` already ran. **Resolution:** Phase 25 needs a second registration window. Options:
- (a) Re-open finalization: add a `add_closure_typedef()` method to `ModuleBuilder` that appends rows after finalization, updating `def_token_map` for the new handles.
- (b) Collect all lambdas in a pre-pass before any body codegen, register their synthetic TypeDefs in ModuleBuilder, then call `finalize()` (delaying finalize to Phase 25).
- **Recommended:** Option (b) — pre-scan all TypedDecl bodies for Lambda nodes, register synthetic TypeDefs/MethodDefs, then finalize. This preserves the clean two-pass pattern.

### Pattern 7: `?`/`try` Propagation (EMIT-23)

The type checker already desugared `?`/`try` into `TypedExpr::Match` nodes. The codegen handles these as special match patterns recognized by their structure (match on Option with IS_NONE early-return pattern, or match on Result with IS_ERR early-return pattern).

However, the typed IR uses `TypedPattern::Variable` and `TypedPattern::Wildcard` for the desugar — codegen must detect the early-return pattern from context (a `TypedArm` with a `Return` body) rather than from a special AST node.

**Sequence for `expr?` (Option unwrap):**
```
; r_opt = emit(expr)
IS_NONE  r_is_none, r_opt
BR_FALSE r_is_none, +skip_return_bytes
LOAD_NULL r_none_val
RET      r_none_val        ; propagate None
; (skip_return label here)
UNWRAP   r_unwrapped, r_opt
```

**Sequence for `try expr` (Result propagation):**
```
; r_result = emit(expr)
IS_ERR   r_is_err, r_result
BR_FALSE r_is_err, +skip_return_bytes
EXTRACT_ERR r_err_val, r_result
WRAP_ERR    r_wrapped_err, r_err_val
RET         r_wrapped_err
; (skip_return label here)
UNWRAP_OK   r_ok_val, r_result
```

### Pattern 8: Enum Match Lowering (EMIT-17)

For `TypedExpr::Match` where scrutinee type is `TyKind::Enum`:
```
; r_enum = emit(scrutinee)
GET_TAG  r_tag, r_enum
SWITCH   r_tag, N, [offset_variant_0, offset_variant_1, ..., offset_wildcard]
; variant_0_label:
;   EXTRACT_FIELD r_payload_0, r_enum, 0
;   ... arm body
;   BR end_label
; variant_1_label:
;   ... arm body
;   BR end_label
; end_label:
```

### Anti-Patterns to Avoid

- **Computing branch offsets eagerly:** The size of `SWITCH` is `6 + 4n` bytes — it can't be computed until `n` is known. Always use labels/fixups.
- **Emitting SET_FIELD for default fields during entity construction:** Only explicit field overrides in `TypedExpr::New { fields }` get SET_FIELD. Default values are handled by the runtime.
- **Registering closure TypeDefs after `ModuleBuilder::finalize()`:** The `def_token_map` is sealed after finalize. Lambda pre-scanning must happen before the finalize call.
- **Duplicating the Instruction enum:** `writ-module` already has a complete, serialization-tested `Instruction` enum. Adding `writ-module` as a dependency is the correct approach.
- **Emitting CALL_VIRT when static type is known concrete:** EMIT-27 requires specializing to CALL. Check `TyKind::Struct(def_id)` or `TyKind::Entity(def_id)` on the receiver.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Instruction enum + serialization | Custom 90-variant enum with encode() | `writ_module::Instruction` | Already exists, tested with round-trip tests in `writ-module/tests/` |
| Binary module serialization | Custom header + table writer | `writ_module::Module::to_bytes()` | Already spec-compliant, handles 200-byte header, aligned rows, heaps |
| TypeRef blob encoding for registers | New encoder | `type_sig::encode_type()` | Already exists in `writ-compiler/src/emit/type_sig.rs` |
| MethodBody struct | Custom struct | `writ_module::module::MethodBody` | Has `register_types`, `code`, `debug_locals`, `source_spans` |

**Key insight:** `writ-module` is the correct layer for IL binary concerns. `writ-compiler` is the correct layer for TypedAst-to-Instruction lowering. The boundary is `Vec<Instruction>` per method body.

---

## Common Pitfalls

### Pitfall 1: Relative Branch Offsets vs Absolute
**What goes wrong:** Emitting `BR offset` where offset is from the NEXT instruction's start, not the current instruction's start.
**Why it happens:** The spec says "relative branch — offset from the start of this instruction" (§3.6). A `BR` instruction itself is 8 bytes (I32 shape: `u16 op + u16 pad + i32 offset`). If offset = 0, the branch is a no-op jumping to itself. Forward jumps need offset = (target_byte_pos - branch_instruction_start_pos).
**How to avoid:** In the fixup pass, compute `offset = target_byte_pos - fixup_byte_pos` where `fixup_byte_pos` is the byte position of the `BR` instruction start.
**Warning signs:** Tests that do `BR` but end up in an infinite loop or jump wrong.

### Pitfall 2: Closure TypeDef Registration Timing
**What goes wrong:** Lambda lowering tries to get a MetadataToken for a compiler-generated TypeDef that was added after `finalize()` — `def_token_map` returns `None`.
**Why it happens:** Phase 24's `emit()` calls `builder.finalize()` before returning. Lambda TypeDefs added during body codegen don't have tokens.
**How to avoid:** Pre-scan all method bodies for Lambda nodes BEFORE Phase 24's finalize. Register synthetic TypeDefs during Phase 24's collection pass or delay finalize to Phase 25.
**Warning signs:** `token_for_def()` panics or returns `None` for closure capture struct TypeDefs.

### Pitfall 3: Entity vs Struct Construction
**What goes wrong:** Emitting `NEW` + `SET_FIELD` for an entity instead of `SPAWN_ENTITY` + `SET_FIELD` + `INIT_ENTITY`.
**Why it happens:** Both `TypedDecl::Struct` and `TypedDecl::Entity` use `TypedExpr::New`. Must check `TyKind::Entity` vs `TyKind::Struct`.
**How to avoid:** In `emit_expr` for `TypedExpr::New { ty, target_def_id, .. }`, branch on `interner.kind(ty)`: `TyKind::Entity` → SPAWN_ENTITY sequence, `TyKind::Struct` → NEW sequence.
**Warning signs:** Entity lifecycle hooks (`on_create`) never fire; entities missing from entity registry.

### Pitfall 4: Boxing at Generic Call Sites
**What goes wrong:** Passing an `int` to a generic parameter without BOX/UNBOX, causing runtime type tag mismatch.
**Why it happens:** Generic dispatch requires boxed values so the runtime can read the type tag from the heap object header.
**How to avoid:** When emitting a CALL where a parameter type is `TyKind::GenericParam`, check if the argument's type is a value type (`TyKind::Int`, `TyKind::Float`, `TyKind::Bool`, `TyKind::Enum`). If so, emit `BOX r_boxed, r_val` before the call. If the return type is `TyKind::GenericParam` and caller expects a value type, emit `UNBOX`.
**Warning signs:** Runtime crashes on generic calls with int/float/bool/enum arguments.

### Pitfall 5: SWITCH Offset Computation with Wildcard
**What goes wrong:** A `match` with a wildcard arm emits an incorrect SWITCH table where wildcard offset points to wrong location.
**Why it happens:** `SWITCH r_tag, N, offsets[N]` falls through if `r_tag >= N`. The wildcard arm must be placed after the switch instruction (fallthrough) or a BR must jump to it. If placed after the last variant arm, need a BR before the wildcard to skip over other arms.
**How to avoid:** Structure: emit explicit variant arms with BR to end, place wildcard arm last (fallthrough from SWITCH when tag out of range). Alternatively, pre-pad the SWITCH offsets array to include all possible tags.
**Warning signs:** Wildcard arms execute for wrong variants.

### Pitfall 6: DEFER_PUSH handler_offset vs DEFER_PUSH r_dst
**What goes wrong:** The `Instruction::DeferPush` in `writ-module` has `{ r_dst: u16, method_idx: u32 }` but spec §3.11 says DEFER_PUSH's RI32 shape has `r_dst` as a padding field and the `method_idx` is actually a byte offset (handler_offset), not a MetadataToken.
**Why it happens:** DEFER_PUSH is RI32 shape but the "r" field is unused padding. The second field is `handler_offset: i32`, not a method index.
**How to avoid:** When emitting DEFER_PUSH, use label fixups to resolve the handler offset (byte offset into the current method body's code section where the defer handler starts). The `DeferPush.method_idx` field in `writ-module` stores this offset as a u32.
**Warning signs:** Defer handlers never execute or execute at wrong offsets.

### Pitfall 7: Argument Register Consecutiveness
**What goes wrong:** `CALL r_dst, method_idx, r_base, argc` requires args in registers `r_base..r_base+argc-1`. Emitting args into non-consecutive registers will pass wrong values.
**Why it happens:** The naive approach emits each arg expression independently, getting non-consecutive registers.
**How to avoid:** Before emitting a call, allocate a consecutive block of registers first, then emit each arg expression MOV-ing into the consecutive slot. Pattern:
```rust
let r_base = regs.next; // record base
for (i, arg) in args.iter().enumerate() {
    let r_arg = emit_expr(arg, ctx); // may use non-consecutive regs
    let r_slot = r_base + i as u16;
    if r_arg != r_slot {
        emit(Instruction::Mov { r_dst: r_slot, r_src: r_arg });
    }
}
```
Or: emit each arg expression, collect result registers, then MOV into a freshly allocated consecutive block.
**Warning signs:** Calls with wrong argument values; hardest to debug.

---

## Code Examples

Verified patterns from the codebase and spec:

### Calling Convention (§2.6)

Method parameters are `r0..rN-1`. For a method `fn foo(self, x: int, y: float)`:
- `r0` = self (type = owning struct/entity)
- `r1` = x (type = int)
- `r2` = y (type = float)
- `r3+` = locals/temporaries

### Instruction Serialization (existing in writ-module)

```rust
// Source: writ-module/src/instruction.rs
use writ_module::Instruction;

let instr = Instruction::AddI { r_dst: 2, r_a: 0, r_b: 1 };
let mut code: Vec<u8> = Vec::new();
instr.encode(&mut code).unwrap();
// Produces: [0x00, 0x02, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00]
//           ^---opcode 0x0200--^  ^--r_dst=2--^  ^r_a=0^ ^r_b=1^
```

### Binary Module Output (existing in writ-module)

```rust
// Source: writ-module/src/module.rs + writer.rs
use writ_module::{Module, ModuleBuilder};

let module: Module = build_module(); // populated by translation from writ-compiler::ModuleBuilder
let bytes: Vec<u8> = module.to_bytes().expect("serialization");
// bytes is the complete spec-compliant .writil binary
```

### TypeRef Encoding for Register Type Table

```rust
// Source: writ-compiler/src/emit/type_sig.rs
use crate::emit::type_sig::encode_type;

let type_ref_blob = encode_type(reg_ty, interner, &|def_id| builder.token_for_def(def_id).unwrap(), &mut builder.blob_heap);
let blob_offset = builder.blob_heap.intern(&type_ref_blob);
// blob_offset is the u32 stored in MethodBody.register_types[reg_idx]
```

### Entity Construction (spec §2.16.7)

```rust
// TypedExpr::New { ty: Entity(guard_def_id), target_def_id: guard_def_id, fields: [("name", expr)] }
let type_token = builder.token_for_def(guard_def_id).unwrap();
let r_entity = regs.alloc(ty); // allocate entity register
emit(Instruction::SpawnEntity { r_dst: r_entity, type_idx: type_token.0 });

for (field_name, field_expr) in &fields {
    let r_val = emit_expr(field_expr, ctx);
    let field_token = resolve_field_token(guard_def_id, field_name, builder);
    emit(Instruction::SetField { r_obj: r_entity, field_idx: field_token.0, r_val });
}

emit(Instruction::InitEntity { r_entity });
```

### Constant Folding (EMIT-28)

```rust
// Only in TypedDecl::Const bodies
fn const_fold(expr: &TypedExpr) -> Option<TypedLiteral> {
    match expr {
        TypedExpr::Literal { value, .. } => Some(value.clone()),
        TypedExpr::Binary { left, op, right, ty, .. } => {
            let l = const_fold(left)?;
            let r = const_fold(right)?;
            fold_binary(op, l, r, *ty)
        }
        TypedExpr::UnaryPrefix { op: PrefixOp::Neg, expr, .. } => {
            let v = const_fold(expr)?;
            fold_neg(v)
        }
        _ => None, // non-constant; emit normal code
    }
}
```

---

## Key Architectural Finding: `writ-module` Dependency

The `writ-module` crate (at `D:/dev/git/Writ/writ-module/`) is a complete, independently-tested module serialization library that Phase 25 should depend on. Key assets:

| Asset | Location | What It Provides |
|-------|----------|-----------------|
| `Instruction` enum | `writ-module/src/instruction.rs` | All 90 opcodes + `encode()` + `opcode()` |
| `Module` struct | `writ-module/src/module.rs` | In-memory module with all 21 tables + `method_bodies` |
| `MethodBody` struct | `writ-module/src/module.rs` | `register_types: Vec<u32>`, `code: Vec<u8>`, `debug_locals`, `source_spans` |
| `DebugLocal` / `SourceSpan` | `writ-module/src/module.rs` | Exact debug info structs matching spec §2.16.6 |
| `Module::to_bytes()` | `writ-module/src/writer.rs` | Complete spec-compliant binary serialization |
| `ModuleBuilder` | `writ-module/src/builder.rs` | Programmatic module construction (alternative to writ-compiler's ModuleBuilder) |

**Integration approach:** Phase 25 translates the `writ-compiler::emit::module_builder::ModuleBuilder` (populated by Phase 24) into a `writ_module::Module`, adds method bodies, then calls `to_bytes()`. The translation maps the internal `ModuleBuilder`'s finalized rows into the Module's table vecs.

---

## Open Questions

1. **Lambda pre-pass timing**
   - What we know: Phase 24's `emit()` calls `builder.finalize()` before returning. Lambda TypeDefs must be registered before finalize.
   - What's unclear: Whether Phase 25 should restructure Phase 24's pipeline (pre-scan before finalize) or add a `reopen_for_closures()` method to ModuleBuilder that appends new TypeDefs after finalization.
   - Recommendation: Pre-scan all Lambda nodes during Phase 25's setup, register synthetic TypeDefs via `builder.add_typedef()`, then call `builder.finalize()` at Phase 25's end (shifting finalization from Phase 24's `emit()` to Phase 25's `emit_bodies()`). This requires a small refactor of Phase 24's `emit()` to NOT call finalize, instead returning the un-finalized builder.

2. **`writ-module::ModuleBuilder` vs translation layer**
   - What we know: `writ-module` has its own `ModuleBuilder` that produces a `Module` directly. `writ-compiler` has its own `ModuleBuilder` (Phase 24) with a different API.
   - What's unclear: Should Phase 25 translate between the two (compiler → module), or should Phase 24 be refactored to use `writ-module::ModuleBuilder` from the start?
   - Recommendation: For Phase 25, implement a `translate()` function that builds a `writ_module::Module` from the `writ-compiler::ModuleBuilder` rows + method bodies. A deeper refactor to unify the two builders is future work.

3. **TypeSpec rows for Option/Result/TaskHandle**
   - What we know: In `type_sig.rs`, `TyKind::Option/Result/TaskHandle` currently emit a placeholder TypeSpec reference (row 0). Phase 25 needs real TypeSpec rows for boxing/generic dispatch.
   - What's unclear: Whether TypeSpec rows are needed for correctness in Phase 25 or can remain as placeholders until Phase 26 (CLI integration + runtime testing).
   - Recommendation: Phase 25 should properly emit TypeSpec rows for `Option<T>` and `Result<T, E>` since `WRAP_SOME`/`IS_NONE` etc. are common and the runtime needs type information.

---

## Sources

### Primary (HIGH confidence)

- `D:/dev/git/Writ/writ-module/src/instruction.rs` — Complete Instruction enum with all 90 opcodes and encode() implementation
- `D:/dev/git/Writ/writ-module/src/module.rs` — MethodBody, DebugLocal, SourceSpan struct definitions
- `D:/dev/git/Writ/writ-module/src/writer.rs` — Module binary serialization (to_bytes)
- `D:/dev/git/Writ/language-spec/spec/67_4_2_opcode_assignment_table.md` — Complete opcode table
- `D:/dev/git/Writ/language-spec/spec/45_2_16_il_module_format.md` — Method body layout §2.16.6, entity construction §2.16.7
- `D:/dev/git/Writ/language-spec/spec/66_4_1_instruction_shape_reference.md` — Instruction encoding shapes
- `D:/dev/git/Writ/language-spec/spec/35_2_6_calling_convention.md` — Register layout for calls
- `D:/dev/git/Writ/language-spec/spec/41_2_12_delegate_model_closures_function_values.md` — Closure/delegate IL patterns
- `D:/dev/git/Writ/language-spec/spec/54_3_6_control_flow.md` — Branch instruction semantics
- `D:/dev/git/Writ/language-spec/spec/58_3_10_type_operations.md` — Option/Result/Enum instructions
- `D:/dev/git/Writ/writ-compiler/src/check/ir.rs` — TypedAst, TypedExpr, TypedStmt, Capture, CaptureMode
- `D:/dev/git/Writ/writ-compiler/src/check/ty.rs` — TyKind variants driving instruction selection
- `D:/dev/git/Writ/writ-compiler/src/emit/module_builder.rs` — Phase 24 ModuleBuilder API
- `D:/dev/git/Writ/writ-compiler/src/emit/type_sig.rs` — Reusable encode_type() for register TypeRefs
- `D:/dev/git/Writ/.planning/phases/25-il-codegen-method-bodies/25-CONTEXT.md` — User decisions

### Secondary (MEDIUM confidence)

- `D:/dev/git/Writ/language-spec/spec/36_2_7_operator_dispatch.md` — Operator lowering rules (primitive = typed IL, user-defined = CALL_VIRT)
- `D:/dev/git/Writ/language-spec/spec/63_3_15_boxing.md` — BOX/UNBOX semantics at generic call sites
- `D:/dev/git/Writ/writ-compiler/src/emit/collect.rs` — Phase 24 collection pass pattern to follow

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — `writ-module` exists and is complete; register allocator and label patterns are well-established compiler techniques
- Architecture: HIGH — Instruction enum, BodyEmitter pattern, and call dispatch decision tree are all grounded in the actual spec and codebase
- Pitfalls: HIGH — Branch offset semantics are from the spec; closure timing issue is from reading the actual ModuleBuilder finalize() code; boxing is from spec §3.15

**Research date:** 2026-03-03
**Valid until:** Indefinite — spec is frozen; codebase is the ground truth
