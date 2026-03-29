# Phase 122: Cross-Module Type Resolution - Research

**Researched:** 2026-03-29
**Domain:** Writ compiler — DefMap population from pre-compiled `.writc` modules
**Confidence:** HIGH (all findings from direct codebase inspection)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting.
Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Claude's Discretion
All implementation decisions are open.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| XMOD-01 | Compiler can load type definitions from a pre-compiled `.writc` module into DefMap | Module already readable via `Module::from_bytes`; new `inject_module_into_def_map` function needed |
| XMOD-02 | User code can reference types from a loaded library module; compiler validates them at compile time | DefMap already drives all name resolution; populating it with library types makes them available |
| XMOD-03 | Virtual module types (Type, FieldInfo, etc.) are resolvable through the same DefMap mechanism | `writ-runtime` virtual module is a `Module` object; same injection function works |
| XMOD-04 | `coll_with_library_separate_modules` test passes (un-ignored) | Test compiles user code with `List<int>` referencing separately-compiled `writ-std.writc` |
| XMOD-05 | Language spec documents cross-module type resolution and using declarations | New spec section in `03_2_project_configuration_writ_toml.md` or a new `XX_modules.md` file |
| XMOD-06 | Integration tests: successful type reference, method call, field access, type-not-found error path | New tests in `writ-compiler/tests/` or `writ-runtime/tests/` |
</phase_requirements>

---

## Summary

The Writ compiler operates on a single compilation unit: `DefMap` is populated exclusively from source AST declarations collected in Pass 1, and then all name resolution in Pass 2 and type-checking consult only this in-memory map. There is no mechanism to pre-populate `DefMap` from a compiled binary module. The result is that user code referencing `List<int>` fails at compile time with "unknown type" when `List` is defined in a separately-compiled `writ-std.writc` library rather than inlined in the source.

The fix is a targeted extension to the `resolve::resolve` entry point: accept an optional slice of pre-compiled `Module` objects (library dependencies), walk each module's `type_defs`, `method_defs`, `contract_defs`, and related tables to reconstruct `DefEntry` records, and insert them into the `DefMap` before Pass 2 begins. Because `DefMap` already uses `by_fqn` (fully-qualified names) and the `Module` format carries namespace strings and type names in its string heap, the mapping is mechanical. The same injection function handles both user library modules and the `writ-runtime` virtual module.

The compiler pipeline entry point (`resolve::resolve` or its callers in `writ-cli/src/pipeline.rs`) needs a new parameter for library modules. The `writ-cli` `cmd_build` and `cmd_run` commands need to discover and load library `.writc` files declared under a new `[dependencies]` section in `writ.toml`.

**Primary recommendation:** Extend `resolve::resolve` to accept `&[&Module]` as library modules; extract `DefEntry` stubs from each module's tables; insert them into `DefMap` before Pass 2. Wire this through `pipeline::run_pipeline` and `writ-cli`. No AST or type-checking changes are needed for the core name-resolution path.

---

## Architecture Patterns

### Existing Pipeline (current state)

```
parse -> lower -> resolve(asts, paths)
                     -> collect_declarations(asts)  [Pass 1: AST -> DefMap]
                     -> inject_log_namespace
                     -> inject_dialogue_namespace
                     -> resolve_bodies              [Pass 2: uses DefMap]
         typecheck(resolved, asts)
         emit_bodies(typed_ast, ...)
```

### Target Pipeline (with cross-module resolution)

```
parse -> lower -> resolve(asts, paths, library_modules)
                     -> inject_module_types(library_modules, def_map)  [NEW]
                     -> collect_declarations(asts)                      [Pass 1: AST -> DefMap]
                     -> inject_log_namespace
                     -> inject_dialogue_namespace
                     -> resolve_bodies                                  [Pass 2: uses augmented DefMap]
         typecheck(resolved, asts)  [unchanged — DefMap now has library types]
         emit_bodies(typed_ast, ...)
```

Library module types must be injected **before** `collect_declarations` (Pass 1) so that the scope chain and type resolver see them during body resolution. The order in the existing pipeline is: inject log/dialogue → collect → resolve bodies. Library injection must precede all three.

### DefEntry Construction from Module Tables

A `Module` contains:
- `type_defs: Vec<TypeDefRow>` — `name` (string heap offset) + `namespace` + `kind: TypeDefKind`
- `method_defs: Vec<MethodDefRow>` — `name` + `flags` (pub flag)
- `contract_defs: Vec<ContractDefRow>` — `name` + `namespace`
- `field_defs: Vec<FieldDefRow>` — `name` + `type_sig` (blob heap offset)
- `string_heap: Vec<u8>` — NUL-terminated strings indexed by offset

Mapping rules:

| Module Table | DefKind | Notes |
|---|---|---|
| `TypeDefRow { kind: Struct }` | `DefKind::Struct` | FQN = `namespace::name` |
| `TypeDefRow { kind: Class }` | `DefKind::Class` | FQN = `namespace::name` |
| `TypeDefRow { kind: Entity }` | `DefKind::Entity` | |
| `TypeDefRow { kind: Component }` | `DefKind::Component` | |
| `TypeDefRow { kind: Enum }` | `DefKind::Enum` | |
| `ContractDefRow` | `DefKind::Contract` | FQN = `namespace::name` |
| `MethodDefRow` | `DefKind::Fn` | Only top-level fns — those not owned by a TypeDef |

Methods on types do **not** need separate `DefEntry` records. Type-checking resolves methods through `TypeEnv::impl_index`, which is built from AST impl blocks. For library types, no AST impl bodies exist — but at the resolve stage we only need the type name to be recognized (so `let x: List<int> = ...` resolves the type). The `TypeEnv` population from AST is unchanged; for library type method calls to work, a parallel path reading method signatures from the module binary may be needed in a subsequent task (see Open Questions).

### Synthetic FileId for Library Entries

Library `DefEntry` records need a `file_id`. Use a deterministic sentinel scheme:
- `FileId(u32::MAX)` is already used by synthetic log/dialogue entries.
- For library modules, use `FileId(u32::MAX - 1 - lib_index)` to avoid collisions between multiple libraries and the existing synthetic sentinel.
- `name_span` and `span` are both zero-length synthetic spans (matching the log namespace injection pattern).

### writ.toml `[dependencies]` Section

The `WritConfig` in `writ-compiler/src/config.rs` needs a new optional section:

```toml
[dependencies]
writ-std = { path = "../../writ-std/build/debug/writ-std.writc" }
```

Or more simply for the integration test use case:

```toml
[dependencies]
writ-std = "path/to/writ-std.writc"
```

The config struct gets a new field:
```rust
#[serde(default)]
pub dependencies: HashMap<String, DependencyConfig>,
```

Where `DependencyConfig` is either a string path or a struct with `path`. For the Phase 122 scope, only path-based dependencies are needed.

### How `coll_with_library_separate_modules` Test Works

The test in `writ-runtime/tests/coll_integration_tests.rs` calls `run_with_library()`, which:
1. Compiles `WRIT_STD_SRC` (collections.writ) to a `Module` via `compile()`
2. Compiles a user source containing `List<int>` via `compile()`
3. Loads both into a `RuntimeBuilder` with `with_library(std_module)`

Step 2 fails today because `compile()` calls `writ_compiler::compile_source()`, which has no way to know about the std module. The fix requires `compile()` to accept library modules, or for the test to switch to a multi-module compilation path.

The cleanest fix for the test: change `run_with_library` to use a new `compile_with_libraries(user_src, &[std_module])` helper that invokes the augmented pipeline.

### Virtual Module Types (XMOD-03)

The `writ-runtime` virtual module (`build_writ_runtime_module()`) constructs contracts (Add, Sub, ..., Iterator, Iterable) and a few type stubs in memory. These types are currently in `PRELUDE_CONTRACT_NAMES` and `PRELUDE_TYPE_NAMES` — hardcoded string arrays that cause the scope chain to return `LookupResult::PreludeContract` or `LookupResult::PreludeType` without a `DefId`.

XMOD-03 says these should be resolvable through the same DefMap mechanism. The approach: call `inject_module_types` with the virtual module (obtained from `build_writ_runtime_module()`) before user-source resolution. Then contracts like `Add`, `Iterable`, `Iterator` get real `DefId` entries in `DefMap`. The prelude arrays can remain as a fast-path check, but the resolution code needs to fall through to `DefMap` for DefId-based lookups.

However, there is a subtlety: the scope chain currently returns `LookupResult::PreludeContract(name)` (no DefId) for contracts. The type checker and resolver use this result to produce `TyKind::Contract(def_id)` — but where does the `def_id` come from? Inspection shows the type checker (`resolve_ast_type_inner` in `env_build.rs`) handles `"Option"`, `"Result"`, `"Array"` specially by name and produces built-in types without a DefId. For contracts, the resolver produces `ResolvedType::Contract(name)` by name. The `TypeEnv::contract_methods` table is keyed by `DefId` — which means contract method validation (for `impl Iterable<T> for Foo`) currently works by looking up the contract's DefId via `def_map.get("Iterable")`. But since `Iterable` is in `PRELUDE_CONTRACT_NAMES`, it has no DefId in the current DefMap — so `def_map.get("Iterable")` returns `None` and `TypeEnv::contract_methods` for prelude contracts is never populated.

This reveals the current behavior: prelude contract impl completeness checking silently passes for prelude contracts (because `contract_def_id` comes back None and the check is skipped). XMOD-03 can be implemented by injecting the virtual module, which gives prelude contracts real DefIds — without breaking existing behavior, since the None-check already guards against the missing entry.

---

## Standard Stack

No new third-party libraries are needed. All changes are internal to the existing Rust crates.

| Crate | Role | Change Required |
|---|---|---|
| `writ-module` | Module deserialization (`Module::from_bytes`) | None — already handles `.writc` reading |
| `writ-compiler` | DefMap population, type resolution | Core change: `inject_module_types`, augmented `resolve::resolve` signature |
| `writ-compiler/config` | `WritConfig` parsing | Add `[dependencies]` support |
| `writ-cli` | Build/run pipeline | Load library `.writc` files, pass to pipeline |
| `writ-runtime` | Virtual module | No change — already exposes `build_writ_runtime_module()` |

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| Reading type names from binary module | Custom parser | `writ_module::heap::read_string(heap, offset)` | Already handles NUL-terminated string heap reads |
| Iterating module type tables | Custom iteration | `module.type_defs.iter()`, `module.contract_defs.iter()` | Tables are plain `Vec<Row>` |
| Module loading from disk | Custom file reader | `Module::from_bytes(&std::fs::read(path)?)` | Already the established pattern |
| FQN construction | Custom formatting | `format!("{}::{}", namespace, name)` | Matches existing DefMap FQN conventions |

---

## Common Pitfalls

### Pitfall 1: Injecting Library Types After Pass 1

**What goes wrong:** If library types are injected after `collect_declarations`, user-defined types that reference library types will fail in Pass 2 body resolution. Pass 2 uses the fully-populated DefMap.

**Why it happens:** `resolve_bodies` runs after `collect_declarations`. Both passes read from the same `DefMap`. Types must exist in `DefMap` before any reference resolution happens.

**How to avoid:** Inject library modules into `DefMap` as the first step in `resolve()`, before `collect_declarations`.

**Warning signs:** "unknown type" errors for library types even after injection is added.

---

### Pitfall 2: impl Block Association for Library Types

**What goes wrong:** Type-checking resolves method calls via `TypeEnv::impl_index`, which is built exclusively from AST `AstDecl::Impl` nodes. Library types have no AST — so `impl_index` will have no entry for them, and method calls on library types will fail with "no method `add` found on type `List<int>`".

**Why it happens:** The `TypeEnv::build` function loops over `resolved.decls` (all AST-derived declarations). Library types are only stubs in DefMap — they have no corresponding `ResolvedDecl::Class` or `ResolvedDecl::Impl` entry.

**How to avoid:** For XMOD-04 (the integration test), the test user code calls `list.add(42)` and `list.get(0)`. These methods are defined in the stdlib's impl blocks compiled into `writ-std.writc`. The test needs method resolution to work.

**Resolution approach:** Extend `TypeEnv::build` to also accept library modules and populate `impl_index` + `fn_sigs` from the module's `method_defs`, `impl_defs`, and type tables. This is more complex than just DefMap injection — it requires reconstructing `FnSig` (parameter types, return type) from binary signatures.

**Alternative for Phase 122 scope:** The test `coll_with_library_separate_modules` may be satisfiable if the user source is augmented to also inline the impl block — but the test is specifically designed to test the `with_library()` path. A pragmatic approach: implement DefMap injection (so types resolve) AND method signature reconstruction from module binary (so method calls type-check).

**Warning signs:** Types resolve but method calls produce "unknown method" errors.

---

### Pitfall 3: Duplicate Definition Collisions

**What goes wrong:** If a user file inlines a class definition that already exists in a library module, the DefMap `insert` call emits E0001 "duplicate definition".

**Why it happens:** Library types are inserted with their FQN. If user code also declares `pub class List<T>`, the FQN `List` collides.

**How to avoid:** Library-injected `DefEntry` records should be treated as lower-priority — either (a) skip insertion if the FQN already exists after Pass 1, or (b) insert before Pass 1 and let Pass 1 collisions become an error (which is correct: the user is re-declaring a library type). For the integration test, this is not an issue since the user source does NOT inline List. The approach in (a) is too permissive. Use (b): insert library types first, report duplicates with a clear message like "type `List` is already defined in library module `writ-std`".

---

### Pitfall 4: Span/FileId Assignment for Library Entries

**What goes wrong:** Diagnostic labels that reference a library entry's `file_id` will fail if the renderer's `sources` slice does not contain that FileId. The `STATE.md` notes: "ariadne panics if secondary label references a FileId absent from the renderer's sources slice".

**Why it happens:** Library entries use a synthetic `FileId`. The `render_diagnostics` call in `pipeline.rs` only includes the user's source files.

**How to avoid:** Use synthetic sentinel FileIds (the existing pattern from `inject_log_namespace` uses `FileId(u32::MAX)`). Never produce secondary diagnostic labels pointing to library entry spans — the spans are zeroed and the FileId is not in the renderer's sources. For duplicate-definition errors involving library types, only label the user-code declaration site (primary), and describe the library origin in the message text.

---

### Pitfall 5: TypeEnv Method Signature Reconstruction from Binary

**What goes wrong:** Reconstructing `FnSig` from binary `MethodDefRow + ParamDefRow` requires decoding type signatures from the blob heap. The type signature encoding format needs to match how `emit/type_sig.rs` encodes them.

**Why it happens:** The blob heap stores type signatures as encoded bytes. Without a decoder that mirrors the encoder, signatures will be misread.

**How to avoid:** Read and understand `writ-compiler/src/emit/type_sig.rs` before implementing the decoder. The decoder must be in `writ-compiler` (not `writ-module`) since it needs access to `TyInterner` and `DefMap` to construct `Ty` values.

---

## Code Examples

### Existing: Synthetic DefEntry injection pattern (from `resolve/mod.rs`)

```rust
// Source: writ-compiler/src/resolve/mod.rs — inject_log_namespace
fn inject_log_namespace(def_map: &mut def_map::DefMap) {
    use chumsky::span::SimpleSpan;
    use def_map::{DefEntry, DefKind, DefVis};

    let synthetic_span = SimpleSpan { start: 0, end: 0, context: () };
    for &level_name in prelude::LOG_NAMESPACE_LEVELS {
        let fqn = format!("log::{}", level_name);
        if def_map.by_fqn.contains_key(&fqn) {
            continue;
        }
        let entry = DefEntry {
            id: None,
            kind: DefKind::ExternFn,
            vis: DefVis::Pub,
            file_id: FileId(u32::MAX),
            namespace: "log".to_string(),
            name: level_name.to_string(),
            name_span: synthetic_span,
            generics: Vec::new(),
            span: synthetic_span,
        };
        let id = def_map.arena.alloc(entry);
        def_map.by_fqn.insert(fqn, id);
    }
}
```

The new `inject_module_types` function follows exactly this pattern, iterating `module.type_defs` instead of a static list.

### Existing: String heap read (from `writ-module/src/heap.rs`)

```rust
// Source: writ-module/src/heap.rs — read_string
pub fn read_string(heap: &[u8], offset: u32) -> Option<&str> { ... }
```

Used to decode `TypeDefRow.name` and `TypeDefRow.namespace` from the string heap.

### Existing: Module loading pattern (from `writ-runtime/tests/coll_integration_tests.rs`)

```rust
// Source: writ-runtime/tests/coll_integration_tests.rs — run_with_library
fn run_with_library(user_src: &str) {
    let std_bytes = compile(WRIT_STD_SRC);
    let std_module = writ_module::Module::from_bytes(&std_bytes).unwrap();
    let user_bytes = compile(user_src); // <-- this is the line that fails today
    ...
}
```

After the fix, `compile(user_src)` must be replaced with `compile_with_libraries(user_src, &[&std_module])`.

---

## Runtime State Inventory

Not applicable — this is a compiler feature addition, not a rename or migration. No stored data, live service config, OS-registered state, secrets, or build artifacts carry names that need renaming.

---

## Environment Availability

All dependencies are in-process Rust crates within this workspace. No external tools, services, or runtimes are required beyond the existing Cargo toolchain.

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| Rust / Cargo | All compilation | Yes | workspace | — |
| `writ-module` crate | Module deserialization | Yes | workspace | — |
| `writ-compiler` crate | DefMap + pipeline | Yes | workspace | — |

---

## Validation Architecture

### Test Framework

| Property | Value |
|---|---|
| Framework | Rust built-in test harness (`cargo test`) |
| Config file | `Cargo.toml` per crate |
| Quick run command | `cargo test -p writ-compiler resolve_tests 2>&1` |
| Full suite command | `cargo test --workspace 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|---|---|---|---|---|
| XMOD-01 | Load types from `.writc` into DefMap | unit | `cargo test -p writ-compiler xmod 2>&1` | No — Wave 0 |
| XMOD-02 | User code references library type | integration | `cargo test -p writ-compiler xmod 2>&1` | No — Wave 0 |
| XMOD-03 | Virtual module types via DefMap | unit | `cargo test -p writ-compiler xmod_virtual 2>&1` | No — Wave 0 |
| XMOD-04 | `coll_with_library_separate_modules` passes | integration | `cargo test -p writ-runtime coll_with_library_separate_modules 2>&1` | Yes (un-ignore) |
| XMOD-05 | Spec docs present (manual verification) | manual | — | No |
| XMOD-06 | Error path: type-not-found from library | integration | `cargo test -p writ-compiler xmod_error 2>&1` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-compiler 2>&1`
- **Per wave merge:** `cargo test --workspace 2>&1`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `writ-compiler/tests/xmod_tests.rs` — covers XMOD-01, XMOD-02, XMOD-03, XMOD-06
- [ ] Remove `#[ignore]` from `coll_with_library_separate_modules` once pipeline is wired — no new test file needed, but test must pass

---

## Open Questions

1. **Method call resolution scope**
   - What we know: `TypeEnv::impl_index` is built from AST impl blocks only. Library types in DefMap have no AST.
   - What's unclear: Whether Phase 122 scope requires method calls on library types to type-check correctly, or only type-name resolution. The `coll_with_library_separate_modules` test calls `list.add(42)` and `list.get(0)` — these require method signatures.
   - Recommendation: Plan must include a task to reconstruct method signatures from module binary. Study `writ-compiler/src/emit/type_sig.rs` to understand the binary encoding before implementing the decoder.

2. **Generics on library types**
   - What we know: `DefEntry.generics` stores generic param names (`["T"]` for `List<T>`). The `TypeDefRow` has a `generic_param_list` pointing into `generic_params` table.
   - What's unclear: Whether generic param names need to be recovered for DefEntry (for type-checking to work with `List<int>`), or whether the count alone is sufficient.
   - Recommendation: Recover generic param names from the `generic_params` table when constructing DefEntry. The count must be correct for generic type instantiation.

3. **`using` declarations for library namespaces**
   - What we know: The scope chain processes `using` declarations by looking up FQNs in `DefMap`. If library types are in `DefMap`, `using writ_std;` will work automatically.
   - What's unclear: Whether the success criterion "how `using` declarations surface library types" requires any special handling beyond having library types in `DefMap`.
   - Recommendation: No special handling needed — `using` already works through DefMap. The spec section just needs to document the behavior.

4. **writ.toml `[dependencies]` vs. test-only compile path**
   - What we know: The `coll_with_library_separate_modules` test does not go through `writ.toml` — it compiles programmatically.
   - What's unclear: Whether Phase 122 needs `writ.toml` dependency loading wired in `cmd_build`, or whether a new `compile_with_libraries` helper for tests is sufficient for XMOD-04.
   - Recommendation: XMOD-04 only requires the test to pass. The `writ.toml [dependencies]` path (for `cmd_build`/`cmd_run`) is needed for XMOD-01/XMOD-02 as a production-ready feature. Implement both — the planner should split into two tasks: (1) core injection function + pipeline signature, (2) writ.toml config + CLI wiring.

---

## Key Insight: Scope of Change

The change is narrower than it first appears:

1. **`writ-compiler/src/resolve/def_map.rs`** — No changes needed. The DefMap API already handles synthetic entries.

2. **`writ-compiler/src/resolve/mod.rs`** — Add `inject_module_types(modules: &[&Module], def_map: &mut DefMap)` function. Called at the top of `resolve()` before `collect_declarations`. The `resolve()` signature gains `library_modules: &[&Module]`.

3. **`writ-compiler/src/check/env.rs`** — Add `TypeEnv::build_library_types(modules: &[&Module], interner: &mut TyInterner)` to populate `fn_sigs` and `impl_index` from binary method signatures. This is the harder part (binary type-sig decoding).

4. **`writ-compiler/src/check/mod.rs`** — `typecheck()` gains `library_modules: &[&Module]` parameter, passes to `TypeEnv::build`.

5. **`writ-compiler/src/lib.rs`** — `compile_source()` stays unchanged (no-library path). New `compile_with_libraries(src, libs)` added.

6. **`writ-cli/src/pipeline.rs`** — `run_pipeline()` gains `library_modules: &[&Module]` parameter.

7. **`writ-compiler/src/config.rs`** — Add `DependencyConfig` and `dependencies: HashMap<String, DependencyConfig>` to `WritConfig`.

8. **`writ-cli/src/commands/build.rs`** and **`run.rs`** — Load `.writc` files from `[dependencies]`, pass to pipeline.

9. **`writ-runtime/tests/coll_integration_tests.rs`** — Update `run_with_library` to use `compile_with_libraries`, remove `#[ignore]`.

10. **Language spec** — New section documenting `[dependencies]`, cross-module resolution, and `using` with library namespaces.

---

## Sources

### Primary (HIGH confidence)
- Direct inspection of `writ-compiler/src/resolve/def_map.rs` — DefMap structure, entry types
- Direct inspection of `writ-compiler/src/resolve/mod.rs` — inject_log_namespace / inject_dialogue_namespace patterns
- Direct inspection of `writ-compiler/src/resolve/collector.rs` — Pass 1 collection
- Direct inspection of `writ-compiler/src/resolve/scope.rs` — resolve_type lookup chain
- Direct inspection of `writ-compiler/src/check/env_build.rs` — resolve_ast_type_inner, resolve_named_def_id
- Direct inspection of `writ-compiler/src/check/env.rs` — TypeEnv::build flow
- Direct inspection of `writ-module/src/module.rs` and `tables.rs` — Module structure
- Direct inspection of `writ-runtime/tests/coll_integration_tests.rs` — test to un-ignore
- Direct inspection of `writ-runtime/src/runtime.rs` — with_library, RuntimeBuilder
- Direct inspection of `.planning/STATE.md` — architectural notes about the cross-module gap
- Direct inspection of `writ-compiler/src/config.rs` — WritConfig, no dependencies section exists yet

### Secondary (MEDIUM confidence)
- `.planning/ROADMAP.md` — success criteria description
- Language spec `03_2_project_configuration_writ_toml.md` — existing writ.toml format (no `[dependencies]` section exists)

---

## Metadata

**Confidence breakdown:**
- Core injection mechanism: HIGH — pattern directly observed in `inject_log_namespace`
- Method signature reconstruction: MEDIUM — requires understanding `type_sig.rs` encoding; not inspected yet
- writ.toml config extension: HIGH — `WritConfig` structure is straightforward TOML deserialization
- Test un-ignore path: HIGH — test code directly inspected, failure reason confirmed

**Research date:** 2026-03-29
**Valid until:** 2026-04-29 (stable codebase, not fast-moving)
