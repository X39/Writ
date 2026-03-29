# Phase 106: Read-Only Introspection Integration Tests and LSP - Research

**Researched:** 2026-03-28
**Domain:** Writ test infrastructure (writ-golden, writ-runtime tests), LSP hover, Type equality
**Confidence:** HIGH

## Summary

Phase 106 is an integration test and LSP polish phase. All runtime intrinsics for REFL-03 through REFL-09
are already implemented in writ-runtime (TypeFields, TypeMethods, TypeAttributes, TypeContracts,
TypeImplements, FieldInfoGet — verified in `writ-runtime/src/dispatch/intrinsics.rs`). The compiler
already lowers `typeof(expr)` to `TypedExpr::TypeOf` emitting `Instruction::TypeOf` (REFL-01, done).
Phase 105 wired Reflectable auto-impl (REFL-02, done). This phase needs to write tests that exercise
all these paths together, add deferred E2E tests from Phases 104/105, and update LSP hover for
`TypedExpr::TypeOf`.

**Primary recommendation:** Write golden .writ files for compiler pipeline tests, write
`writ-runtime` integration tests for runtime behaviour, and add a `TypedExpr::TypeOf` arm to
`hover_text_for_expr` in `writ-lsp/src/queries/hover.rs`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- typeof(Animal) on polymorphic Dog returns Animal (static); dog.get_type() returns Dog (dynamic)
- typeof(T) == typeof(T) must be true (interned by TypeDef)
- typeof(T) == typeof(U) must be false for different types
- GC survival: cached Type objects persist after GC with no script-side roots
- LSP shows "Type" as the hover type for typeof expressions

### Claude's Discretion
All implementation choices are at Claude's discretion — integration test phase.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REFL-03 | Type.fields() returns Array of FieldInfo for all pub fields | TypeFields intrinsic exists in intrinsics.rs:420; test pattern from reflection_tests.rs:test_type_fields_returns_array |
| REFL-04 | Type.methods() returns Array of MethodInfo for all pub methods | TypeMethods intrinsic exists in intrinsics.rs:447; analogous to REFL-03 test pattern |
| REFL-05 | Type.attributes() returns Array of AttributeInfo | TypeAttributes intrinsic in intrinsics.rs:472; test pattern from reflection_tests.rs:test_type_attributes_from_module_attribute_view |
| REFL-06 | Type.contracts() returns Array of ContractInfo | TypeContracts intrinsic in intrinsics.rs:521 |
| REFL-07 | Type.implements(contract) returns bool | TypeImplements intrinsic in dispatch/mod.rs |
| REFL-08 | FieldInfo.get(instance) returns field value dynamically (boxed) | FieldInfoGet intrinsic; test pattern from reflection_tests.rs:test_field_info_get |
| REFL-09 | Type equality by identity — typeof(T) == typeof(T) is true, interned per TypeDef | CmpEqI on HeapRef is correct because Type singletons are interned; no new instruction needed |
| LSP-01 | Standard diagnostics for reflection type usage | Diagnostics flow through normal type checker; TyKind::ReflectionType already has display "Type" |
| LSP-02 | Hover display for typeof() and reflection type members | TypedExpr::TypeOf has no arm in hover_text_for_expr; falls to catch-all showing "Type"; needs explicit arm per CONTEXT.md locked decision |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| writ-golden (crate) | local | Compiler pipeline golden tests (compile → disassemble → compare .writil) | Only golden test framework in project |
| writ-runtime (crate) | local | Runtime integration tests (hand-assemble IL, spawn task, tick, assert return value) | All existing reflection tests use this pattern |
| writ-lsp (crate) | local | LSP hover unit tests (build_typed_ast_full helper in hover.rs tests module) | Established pattern in hover.rs:456 |

### Test Commands
```bash
cargo test -p writ-golden          # golden file pipeline tests
cargo test -p writ-runtime         # runtime integration tests
cargo test -p writ-lsp             # LSP unit tests
BLESS=1 cargo test -p writ-golden  # regenerate .writil snapshots
```

## Architecture Patterns

### Pattern 1: Golden Test (compile-disassemble-compare)

**What:** Write `.writ` source, run through full compiler pipeline, compare disassembled IL against checked-in `.writil` snapshot.

**When to use:** To lock that `typeof(T)` lowers correctly and the TYPEOF instruction appears with the right type token; to lock that reflection method calls lower to CALL_VIRT with the correct contract tokens.

**Entry point:** `run_golden_test("name")` in `writ-golden/tests/golden_tests.rs`.

**File layout:**
```
writ-golden/tests/golden/
├── refl_typeof_basic.writ     # source
├── refl_typeof_basic.writil   # snapshot (created with BLESS=1)
├── refl_fields.writ
├── refl_fields.writil
└── ...
```

**Example invocation in golden_tests.rs:**
```rust
#[test]
fn test_refl_typeof_basic() {
    run_golden_test("refl_typeof_basic");
}
```

**Source format (plain Writ, not runnable — compile only):**
```writ
struct Point { x: int, y: int }

fn main() {
    let t = typeof(Point);
}
```

**Important:** Golden tests validate the compiler pipeline (IL structure). They do NOT run the compiled code. The harness is `compile → Module::from_bytes → disassemble`. There are no runtime assertions in golden tests.

**Existing TYPEOF evidence:** `.writil` snapshots from Phases 104/105 already include `get_type()` auto-impl with `TYPEOF r1, <token>`. The format is established.

### Pattern 2: Runtime Integration Test (hand-assembled IL, VM run)

**What:** Construct a `ModuleBuilder`, manually assemble instructions, spawn a task, tick, assert return value or heap state.

**When to use:** To validate that `Type.fields()` returns correct FieldInfo array, `typeof(T) == typeof(T)` is true, GC survival, `FieldInfo.get(instance)` returns correct value.

**Entry point:** `writ-runtime/tests/reflection_tests.rs` (existing file, add more `#[test]` functions).

**Key helpers:**
```rust
fn encode(instrs: &[Instruction]) -> Vec<u8>  // in reflection_tests.rs
fn typedef_token(typedef_idx: usize) -> u32    // table_id=2, row=typedef_idx+1
```

**Pattern (from existing tests):**
```rust
let mut builder = ModuleBuilder::new("test");
builder.add_type_def("MyStruct", "", TypeDefKind::Struct, 0);
// ...add_field_def, add_module_ref, add_type_ref as needed...
let body = MethodBody { register_types: vec![0; N], code: encode(&[...]), ... };
builder.add_method("main", &[0], 0, N, body);
let module = builder.build();
let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();
let tid = runtime.spawn_task(0, vec![]).unwrap();
runtime.tick(0.0, ExecutionLimit::None);
assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));
assert_eq!(runtime.return_value(tid), Some(Value::Bool(true)));
```

**HeapRef equality (for REFL-09):** `Value::Ref(HeapRef)` — two `Value::Ref` with the same underlying `HeapRef` are equal because `HeapRef` derives `PartialEq`. Since `ReflectionIndex` returns the same cached `HeapRef` for the same TypeDef, asserting `return_value == Some(Value::Bool(true))` after a `typeof(T) == typeof(T)` expression is sufficient.

**CmpEqI is correct for Type equality:** `TyKind::ReflectionType` falls into the `_` catch-all in `emit_binary` (binary.rs:140), emitting `CmpEqI`. This performs pointer/value identity on the `i64` representation of `HeapRef`. Since Type singletons are permanently cached in `ReflectionIndex`, the same `TypeDef → HeapRef` mapping guarantees identity equality. No new instruction is needed.

### Pattern 3: LSP Unit Test (typed AST hover)

**What:** Build a typed AST from source string, find an expression by offset, assert hover text.

**Entry point:** `writ-lsp/src/queries/hover.rs` (tests module at line 449).

**Helper:**
```rust
fn build_typed_ast_full(src: &str) -> (TypedAst, TyInterner, TypeEnv)
```

**Pattern:**
```rust
let src = r#"struct Foo {} fn main() { let t = typeof(Foo); }"#;
let (ast, interner, type_env) = build_typed_ast_full(src);
let typeof_offset = src.find("typeof").unwrap();
let expr = expr_at_offset(&ast, typeof_offset, FileId(0)).expect("found expr");
let hover = hover_text_for_expr(expr, &ast.def_map, &interner, &type_env, src, &ast);
assert!(hover.contains("Type"), "hover should show 'Type'");
```

### Pattern 4: Writ Source Calling Reflection API (golden + runtime)

For testing that Writ source code can call `t.fields()`, `t.methods()`, etc., two approaches are needed:

1. **Golden test:** Write `.writ` calling `t.fields()` and check the IL shows `CALL_VIRT` with the `Type.fields` contract token.
2. **Runtime test:** Hand-assemble IL using `TypeRef` pointing to `"Type.fields"` contract in `writ-runtime` module, run, assert result is `Value::Ref` (array).

**IMPORTANT: Writ source can call reflection methods only if the compiler resolves the Type API.** Phase 104 added `TyKind::ReflectionType` and compiler type-checking for `typeof()`. Whether the compiler supports calling `.fields()` on a `Type` value depends on Phase 104's method resolution implementation. If it does, golden tests with Writ source are the primary approach. If not, runtime integration tests using hand-assembled IL are the fallback.

**Verification step:** Write a minimal Writ source calling `typeof(Foo).fields()` and run `compile_and_disassemble`. If it compiles, golden tests work. If it fails with a type error, the compiler's method resolution for `TyKind::ReflectionType` needs to be extended first — this may be a Phase 106 implementation task, not just testing.

### Anti-Patterns to Avoid

- **Writing golden tests that call runtime:** Golden tests are compile-only. Do not add `RuntimeBuilder` to `golden_tests.rs`. Runtime assertions belong in `writ-runtime/tests/`.
- **Hand-assembling golden file IL:** Golden tests compile Writ source. Do not write `.writil` files by hand for reflection tests — let `BLESS=1` create them.
- **Asserting specific HeapRef addresses:** HeapRefs are GC-assigned. Assert `Value::Ref(_)` shape-matches, then check field values separately using `heap.get_field()`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Type singleton caching | Custom equality intrinsic | Existing CmpEqI on HeapRef | ReflectionIndex already ensures same TypeDef → same HeapRef; CmpEqI does pointer identity |
| LSP hover for new expression type | New hover dispatch mechanism | Add arm to `hover_text_for_expr` match in hover.rs | All expression types handled in one match; catch-all already returns type string |
| Runtime E2E compile-and-run helper | Custom compile+run harness | `compile_to_module` + RuntimeBuilder pattern | `compile_to_module` already exists in golden_tests.rs; for runtime tests, use ModuleBuilder directly |
| Attribute data duplication | Second attribute scan path | Existing unified `TypeAttributes` intrinsic | RT-05 unified attribute path already shared with ModuleAttributeView |

## Key Implementation Gap: Compiler method resolution for Type API

The critical unknown is whether the compiler's type checker and emitter handle method calls on `TyKind::ReflectionType` values. Specifically: does `typeof(Foo).fields()` parse, type-check, and lower to `CALL_VIRT` with the `Type.fields` contract?

**Evidence suggesting it may not work yet:** Phase 104 added `TyKind::ReflectionType` for type checking `typeof()` itself, but the CONTEXT.md notes COMP-04 (BOX/UNBOX at reflection API boundaries) is deferred, and Phase 104's deferred test for "typeof passes to Type parameter" is listed as a specific item to address in Phase 106.

**Consequence for test design:** If the compiler cannot resolve `.fields()` on a `Type` value, then:
1. The compiler extension (method resolution for `TyKind::ReflectionType`) is a Phase 106 implementation task.
2. Runtime tests use hand-assembled IL directly (proven pattern from reflection_tests.rs).
3. Golden tests prove the compiler correctly emits CALL_VIRT for type method calls after the extension.

**Verification approach (Wave 0):** Start by attempting `compile_and_disassemble("struct Foo {} fn main() { let t = typeof(Foo); let f = t.fields(); }")` before writing other tests.

## Common Pitfalls

### Pitfall 1: typeof(T) == typeof(T) test requires two separate TypeOf calls

**What goes wrong:** Writing `let a = typeof(T); let b = a;` and comparing — this compares the same register, not two TypeOf calls.

**Why it happens:** Testing interning requires two independent TYPEOF instructions, not register aliasing.

**How to avoid:** Emit two separate `Instruction::TypeOf` instructions with the same `type_idx`. Both should return the same `HeapRef` from the `ReflectionIndex` cache. Then `CmpEqI` on those two refs returns `true`.

**Warning signs:** Test passes trivially because it compares the same value.

### Pitfall 2: Golden test expects no TYPEOF in output for non-type-declaring .writ files

**What goes wrong:** Adding reflection calls to a new golden `.writ` file that has no struct/class/entity declarations. Reflectable auto-impl only emits for user-defined types.

**Why it happens:** TYPEOF instructions appear only as (a) auto-generated `get_type()` bodies for each user TypeDef, and (b) explicit `typeof(Expr)` calls in user code.

**How to avoid:** All reflection golden tests need at least one `struct`/`class`/`entity`/`enum` declaration to produce interesting output.

### Pitfall 3: FieldInfo.get() test needs correct r_base layout

**What goes wrong:** CALL_VIRT with wrong r_base means self (FieldInfo) and instance argument are in wrong registers.

**Why it happens:** `r_base` in CALL_VIRT is the start of the argument window; argc includes self. The pattern is `r_base = r_self`, `r_base+1 = instance_arg` for a method with one explicit argument.

**How to avoid:** Follow the pattern from `test_field_info_get` in reflection_tests.rs exactly — `r_base: r_fieldinfo_register`, `argc: 2` (self + instance).

### Pitfall 4: LSP hover catch-all already returns "Type" — test must verify explicit arm behavior

**What goes wrong:** Assuming the hover test passes because the catch-all in `hover_text_for_expr` returns the type string for unknown expression variants.

**Why it happens:** `TypedExpr::TypeOf` has no explicit arm in `hover_text_for_expr`. It falls through to the `_` catch-all which calls `interner.display_named(expr.ty(), def_map)`. Since `TyKind::ReflectionType` displays as `"Type"`, the hover already shows `"Type"` without any code change.

**How to avoid:** The locked decision says "LSP shows 'Type' as hover type". The catch-all already does this. LSP-02 may require adding an explicit arm for clarity (showing `typeof(Foo)` with the inner type name, not just `"Type"`). The plan should clarify: an explicit arm showing `"Type"` is a code quality improvement; the LSP-01 diagnostic tests (type errors in reflection usage) are more substantive work.

**How to avoid confusion:** Test that hovering over `typeof(Foo)` shows `"Type"` — this will pass with or without an explicit arm. If LSP-02 requires richer display (e.g., `"typeof(Foo): Type"`), an explicit arm in the match is needed.

### Pitfall 5: Static vs dynamic typeof test needs polymorphic assignment

**What goes wrong:** Testing static typeof on a concrete variable, not a polymorphic one. `let x: Foo = Foo {}; typeof(x)` always returns `Foo` for both static and dynamic — no distinction.

**Why it happens:** The distinction only manifests when a variable is declared with a contract type or base class type but holds a subtype instance. In Writ, this means passing to a function that accepts a contract type.

**How to avoid:** The canonical test is: function `fn test(animal: Reflectable) { typeof(animal) vs animal.get_type() }`. Pass a Dog instance. `typeof(animal)` returns the `Reflectable` contract type (static); `animal.get_type()` returns `Dog` (dynamic). This requires a function call with a contract parameter.

**Note:** Whether the compiler handles `typeof(param)` where `param: SomeContract` emits the contract's TypeRef or the parameter's declared type needs verification.

## Code Examples

### Type equality test (IL assembly)
```rust
// Source: reflection_tests.rs pattern
// Two separate TypeOf calls → same HeapRef → CmpEqI returns true
Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },  // first typeof(T)
Instruction::TypeOf { r_dst: 1, type_idx: typedef_token(0) },  // second typeof(T)
Instruction::CmpEqI { r_dst: 2, r_a: 0, r_b: 1 },             // pointer identity
Instruction::Ret { r_src: 2 },
// Expected result: Value::Bool(true)
```

### Type inequality test (IL assembly)
```rust
// Two TypeOf with DIFFERENT type_idx → different HeapRefs → CmpEqI returns false
Instruction::TypeOf { r_dst: 0, type_idx: typedef_token(0) },  // typeof(T)
Instruction::TypeOf { r_dst: 1, type_idx: typedef_token(1) },  // typeof(U)
Instruction::CmpEqI { r_dst: 2, r_a: 0, r_b: 1 },             // pointer identity
Instruction::Ret { r_src: 2 },
// Expected result: Value::Bool(false)
```

### GC survival test (existing pattern from reflection_tests.rs:test_type_object_survives_gc)
```rust
// Already implemented — reference this test, extend to cover Type.fields() cached objects
runtime.collect_garbage();
assert_eq!(stats.objects_freed, 0, "Type objects are permanent GC roots");
```

### LSP hover for TypeOf expression
```rust
// Source: hover.rs hover_text_for_expr match
TypedExpr::TypeOf { .. } => {
    // Locked decision: LSP shows "Type" as hover type for typeof expressions
    "```writ\nType\n```".to_string()
}
```

### Golden .writ file for typeof basic
```writ
// writ-golden/tests/golden/refl_typeof_basic.writ
struct Point { x: int, y: int }

fn main() {
    let t = typeof(Point);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Phase 103 returned `Value::Int(1)` sentinel | Phase 103 replaced with lazy singleton HeapRef via ReflectionIndex | Phase 103 | CmpEqI on two TypeOf results now correctly tests identity |
| No reflection method dispatch | TypeFields/TypeMethods/TypeAttributes/TypeContracts/TypeImplements/FieldInfoGet all implemented | Phase 103-04 | Runtime tests can exercise all REFL-03 through REFL-08 |

## Open Questions

1. **Can Writ source code call `.fields()`, `.methods()`, etc. on a `Type` value?**
   - What we know: `TyKind::ReflectionType` exists; the runtime intrinsics are registered via `domain_dispatch.rs` as `("Type", "type_fields") => IntrinsicId::TypeFields`
   - What's unclear: Does the compiler's method resolution look up methods on `TyKind::ReflectionType` via the virtual module? Phase 104 COMP-04 defers "BOX/UNBOX at reflection API boundaries" — if method lookup is also deferred, Writ source calls will fail type checking.
   - Recommendation: Test `compile_and_disassemble` on Writ source calling `.fields()` in Wave 0. If it fails, Phase 106 includes compiler method resolution work before writing golden tests.

2. **Does static typeof work on contract-typed parameters?**
   - What we know: `typeof(animal)` where `animal: Animal` (struct) emits `TYPEOF` with Animal's type token. The locked decision says `typeof(Animal)` on polymorphic Dog returns Animal (static).
   - What's unclear: Whether `animal` is a local with concrete type or a parameter with contract type changes the `static_ty` field on `TypedExpr::TypeOf`.
   - Recommendation: Write a small test function with a contract-typed parameter and verify the TYPEOF token.

3. **Does Type.implements(contract) take a Type argument or a contract TypeRef?**
   - What we know: `TypeImplements` is registered as an intrinsic for `("Type", "type_implements")`.
   - What's unclear: The argument type for `type_implements` — is it a `Type` object or a contract index? Verify `writ-runtime/src/dispatch/intrinsics.rs` TypeImplements arm.
   - Recommendation: Read the TypeImplements arm to determine argument protocol before writing REFL-07 test.

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — all work is within the Writ Rust workspace).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (cargo test) |
| Config file | none — standard Cargo workspace |
| Quick run command | `cargo test -p writ-runtime reflection` |
| Full suite command | `cargo test -p writ-golden && cargo test -p writ-runtime && cargo test -p writ-lsp` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REFL-03 | Type.fields() returns Array of FieldInfo | unit (IL) | `cargo test -p writ-runtime test_type_fields` | ✅ (extend reflection_tests.rs) |
| REFL-04 | Type.methods() returns Array of MethodInfo | unit (IL) | `cargo test -p writ-runtime test_type_methods` | ❌ Wave 0 |
| REFL-05 | Type.attributes() returns AttributeInfo array | unit (IL) | `cargo test -p writ-runtime test_type_attributes` | ✅ (extend reflection_tests.rs) |
| REFL-06 | Type.contracts() returns ContractInfo array | unit (IL) | `cargo test -p writ-runtime test_type_contracts` | ❌ Wave 0 |
| REFL-07 | Type.implements(contract) returns bool | unit (IL) | `cargo test -p writ-runtime test_type_implements` | ❌ Wave 0 |
| REFL-08 | FieldInfo.get(instance) returns field value | unit (IL) | `cargo test -p writ-runtime test_field_info_get` | ✅ (extend reflection_tests.rs) |
| REFL-09 | typeof(T) == typeof(T) is true; typeof(T) == typeof(U) is false | unit (IL) | `cargo test -p writ-runtime test_type_equality` | ❌ Wave 0 |
| REFL-09 | GC survival of cached Type objects | unit (IL) | `cargo test -p writ-runtime test_type_object_survives_gc` | ✅ (exists) |
| LSP-01 | Type errors in reflection usage produce diagnostics | unit (LSP) | `cargo test -p writ-lsp` | ❌ Wave 0 |
| LSP-02 | Hover for typeof() shows "Type" | unit (LSP) | `cargo test -p writ-lsp test_hover_typeof` | ❌ Wave 0 |
| (golden) | typeof() lowers to TYPEOF instruction | golden | `cargo test -p writ-golden refl_typeof` | ❌ Wave 0 |
| (golden) | .fields() call lowers to CALL_VIRT | golden | `cargo test -p writ-golden refl_fields` | ❌ Wave 0 |
| (deferred from 104) | typeof passes to Type parameter | unit | `cargo test -p writ-compiler comp_04` | ❌ Wave 0 |
| (deferred from 105) | compiled get_type() returns correct name | unit (IL) | `cargo test -p writ-runtime test_get_type_name` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime -- --test-thread=1 reflection`
- **Per wave merge:** `cargo test -p writ-golden && cargo test -p writ-runtime && cargo test -p writ-lsp`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/reflection_tests.rs` — add test functions for REFL-04, REFL-06, REFL-07, REFL-09 (file exists, add to it)
- [ ] `writ-golden/tests/golden/refl_typeof_basic.writ` + `refl_typeof_basic.writil` — REFL-01 compiler golden
- [ ] `writ-golden/tests/golden/refl_fields.writ` + `refl_fields.writil` — REFL-03 compiler golden
- [ ] `writ-lsp/src/queries/hover.rs` — add `TypedExpr::TypeOf` arm + unit test for LSP-02
- [ ] LSP diagnostic test for type errors in reflection usage (LSP-01)
- [ ] Verify compiler handles method calls on `TyKind::ReflectionType` (open question 1)

## Sources

### Primary (HIGH confidence)
- `writ-golden/tests/golden_tests.rs` — full golden test harness read; confirmed compile-disassemble only, no runtime execution
- `writ-runtime/tests/reflection_tests.rs` — all 5 existing reflection tests read; confirmed IL assembly patterns, TypeFields, FieldInfoGet, TypeAttributes, GC root tests
- `writ-lsp/src/queries/hover.rs` — full file read; confirmed TypedExpr::TypeOf has no explicit arm (falls to catch-all); display_named returns "Type" for ReflectionType
- `writ-compiler/src/emit/body/expr/binary.rs` — confirmed CmpEqI used for all non-primitive types including ReflectionType (catch-all at line 140)
- `writ-compiler/src/check/ty.rs` — confirmed `TyKind::ReflectionType` displays as "Type" in both `display` and `display_named`
- `writ-runtime/src/dispatch/intrinsics.rs` — confirmed TypeFields, TypeMethods, TypeAttributes, TypeContracts, TypeImplements, FieldInfoGet, FieldInfoGetName, FieldInfoGetDeclaredType, FieldInfoGetIsMutable all implemented
- `writ-module/src/instruction.rs` — confirmed no `CmpEqRef` instruction exists; CmpEqI (0x0500) is the only reference equality path

### Secondary (MEDIUM confidence)
- `writ-runtime/src/domain_dispatch.rs` — confirmed intrinsic name-to-ID mappings for all Type/FieldInfo methods
- `writ-runtime/src/virtual_module.rs` — confirmed `Type.fields`, `Type.methods`, etc. registered as single-method contracts
- `writ-golden/tests/golden/*.writil` — confirmed TYPEOF instruction appears in existing snapshots with token format `33554433` (0x2_000001 = table_id=2, row=1)

## Metadata

**Confidence breakdown:**
- Golden test infrastructure: HIGH — code read, patterns confirmed
- Runtime test infrastructure: HIGH — existing reflection tests read and patterns confirmed
- Type equality (CmpEqI): HIGH — binary.rs catch-all confirmed, no new instruction needed
- LSP hover gap: HIGH — TypedExpr::TypeOf has no explicit arm, catch-all produces "Type"
- Compiler method resolution for Type API: LOW — not verified (open question 1)
- Static vs dynamic typeof: MEDIUM — locked decision confirmed in CONTEXT.md, exact compiler behavior on contract parameters not verified

**Research date:** 2026-03-28
**Valid until:** 2026-04-28 (stable codebase, no external dependencies)
