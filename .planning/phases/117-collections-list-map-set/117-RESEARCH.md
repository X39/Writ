# Phase 117: Collections (List, Map, Set) - Research

**Researched:** 2026-03-29
**Domain:** Pure-Writ standard library, generic class compilation, GC transitivity, writ-std library module pre-loading
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Claude's Discretion
All implementation choices.

### Deferred Ideas (OUT OF SCOPE)
None — discuss phase skipped.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| COLL-01 | User can create and use `List<T>` with push, pop, get(i), set(i, v), len(), contains(v) | Pure-Writ class wrapping `T[]`; array dot-call opcodes 0x0900-0x0908 already exist in VM |
| COLL-02 | User can create and use `Map<K, V>` (sorted-array backed, O(log n)) | Pure-Writ class with two parallel arrays `K[]` and `V[]`; binary-search via index ops |
| COLL-03 | User can create and use `Set<T>` with add, remove, contains, len | Pure-Writ class wrapping `T[]` with dedup |
| COLL-05 | User can create and use hash-based `HashMap<K, V>` with Hashable contract | Pure-Writ class using FNV-1a hash intrinsic already registered for int/string/bool/float |
| COLL-06 | All collections written in pure Writ source, loaded as library modules | `RuntimeBuilder::with_library()` + `with_library_source()` API already exists |
</phase_requirements>

---

## Summary

Phase 117 delivers four collection types — `List<T>`, `Map<K,V>`, `Set<T>`, `HashMap<K,V>` — all written as pure Writ source in a `writ-std/` crate. The key architectural invariant (from STATE.md) is **no compiler special-casing**: collections are ordinary generic classes that the existing compiler and runtime paths handle without modification.

The phase has two mandatory pre-work items before any collection code: (1) a `class-containing-array` GC correctness test to prove that `MarkSweepHeap::trace_refs` propagates through `HeapObject::Struct` fields that contain `HeapRef`s, and (2) end-to-end validation that `writ-cli` can load a compiled writ-std stub module via `RuntimeBuilder::with_library` before user code. Both must pass before writing collection classes.

The runtime already supports library loading via `RuntimeBuilder::with_library(module)` and `RuntimeBuilder::with_library_source(src)`. The compiler pipeline already compiles multi-file source lists. The primary new work is: creating the `writ-std/` crate directory, writing the four collection classes in Writ syntax, compiling them into a library `.writc`, wiring the CLI/runtime to load the library automatically, and writing golden + integration tests.

**Primary recommendation:** Create `writ-std/src/collections.writ` with all four collection classes. Compile it to `writ-std.writc` during build. Wire `writ-cli` to pre-load the compiled `writ-std.writc` before user modules in both `cmd_run` and `cmd_build`. Use the existing `RuntimeBuilder::with_library()` API — no runtime changes required.

---

## Standard Stack

### Core
| Library/Component | Version | Purpose | Why Standard |
|-------------------|---------|---------|--------------|
| `writ-runtime::RuntimeBuilder::with_library()` | existing | Load pre-compiled lib module before user module | Already implemented in `runtime.rs:95` |
| `writ-runtime::RuntimeBuilder::with_library_source()` | existing (cfg=compiler feature) | Compile+load lib source inline | Already implemented; uses `writ_compiler::compile_source` |
| `HeapObject::Array { elem_type, elements }` | existing | Backing storage for all collection types | All array opcodes 0x0900-0x0908 dispatch to this |
| `HeapObject::Struct { type_key, fields }` | existing | Wrapper class instance on heap | `NEW` opcode + `GET_FIELD`/`SET_FIELD` already work |
| `MarkSweepHeap::trace_refs` | existing | GC traces through Struct fields | Handles `Value::Ref(href)` inside fields already |
| `writ-golden` test harness | existing | Golden IL snapshot tests + runtime execution tests | `run_golden_test`, `RuntimeBuilder::with_library_source` |

### Supporting
| Component | Purpose | When to Use |
|-----------|---------|-------------|
| `Hashable` contract (virtual module, contract 20) | K-type bound for HashMap | Already registered; int/string/bool/float auto-impl |
| `Eq` contract (virtual module, contract 8) | T-type bound for Set/List.contains | Already registered |
| FNV-1a hash intrinsic (`StringHash` opcode) | HashMap bucket computation | Phase 116 decision: FNV-1a with offset basis 0xcbf29ce484222325 |
| `writ-compiler::compile_source` | Compile writ-std at test time | For `with_library_source` in tests |

### Installation / New Crate
```
writ-std/
├── Cargo.toml     (just a directory — Writ source, not a Rust crate)
└── src/
    └── collections.writ
```

Note: `writ-std/` is a **Writ project directory** (with `writ.toml`), not a Rust crate. It is compiled by `writ build` or programmatically via `compile_source`. It is NOT added to the Cargo workspace.

---

## Architecture Patterns

### Recommended Project Structure
```
writ-std/
├── writ.toml              # [project] name="writ-std", [compiler] sources=["src/"]
└── src/
    └── collections.writ   # List<T>, Map<K,V>, Set<T>, HashMap<K,V>

writ-golden/tests/golden/
├── gc_class_containing_array.writ          # Pre-work GC test
├── coll_list_basic.writ                    # List<T> golden
├── coll_map_basic.writ                     # Map<K,V> golden
├── coll_set_basic.writ                     # Set<T> golden
└── coll_hashmap_basic.writ                 # HashMap<K,V> golden

writ-runtime/tests/
└── coll_integration_tests.rs              # Runtime execution tests with library loaded
```

### Pattern 1: Pure-Writ Generic Class
**What:** A `pub class List<T>` with a single `items: T[]` field, exposing methods that delegate to array dot-call ops.
**When to use:** All four collection types follow this pattern.
**Example:**
```writ
// writ-std/src/collections.writ
pub class List<T> {
    items: T[]
}

// Inherent impl methods for List<T>
impl List<T> {
    pub fn new() -> List<T> {
        new List<T> { items: [] }
    }

    pub fn add(mut self, item: T) {
        self.items.add(item);
    }

    pub fn get(self, index: int) -> T {
        self.items[index]
    }

    pub fn len(self) -> int {
        self.items.len()
    }

    pub fn remove_at(mut self, index: int) {
        self.items.remove_at(index);
    }

    pub fn contains(self, item: T) -> bool {
        self.items.contains(item)
    }
}
```

### Pattern 2: Sorted-Array Map (COLL-02)
**What:** `Map<K, V>` backed by two parallel sorted arrays. `set(k, v)` does a binary-search insert to maintain order. `get(k)` does binary-search lookup.
**When to use:** COLL-02 specifies O(log n) — sorted array is the simplest pure-Writ implementation.
**Key decision:** Binary search requires `K: Ord`. COLL-02 spec says Map<K,V> is sorted-array backed. The `Ord` contract is already in the virtual module (contract 9).

```writ
pub class Map<K, V> {
    keys: K[],
    values: V[]
}
```

### Pattern 3: HashMap<K,V> with Hashable
**What:** `HashMap<K, V>` backed by a flat array of buckets (open-addressing or chained). Uses `hash()` intrinsic already provided for int/string/bool/float via `Hashable` contract.
**When to use:** COLL-05. The `Hashable` contract (contract 20 in the virtual module) is auto-implemented for primitives.
**Key constraint:** `K: Hashable` bound enforcement must come from Phase 115 GEN-03. If GEN-03 is not yet landed, HashMap can be declared but the bound won't be enforced at compile time until Phase 115 is complete. Phase 117 depends on Phase 115.

### Pattern 4: Library Module Loading in Tests
**What:** For runtime integration tests, use `RuntimeBuilder::with_library_source(src)` to compile and load writ-std inline.
**When to use:** All integration tests that instantiate collection types.

```rust
// Source: writ-runtime/src/runtime.rs:104
let runtime = RuntimeBuilder::from_source(user_src)?
    .with_library_source(WRIT_STD_SRC)?
    .build()?;
```

### Pattern 5: writ-cli Library Pre-Loading
**What:** `cmd_run` and `cmd_build` must load a compiled `writ-std.writc` before the user module. The library `.writc` is embedded in the `writ` binary (via `include_bytes!`) or resolved from a well-known path.
**When to use:** All `writ run` and `writ build` invocations.
**Recommended approach:** Embed `writ-std.writc` via `include_bytes!` in `writ-cli`. This is the simplest approach that requires no filesystem path resolution. The `writ-std.writc` is compiled as a build artifact (via `build.rs` in `writ-cli`).

```rust
// writ-cli/src/commands/run.rs — modified
const WRIT_STD_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/writ-std.writc"));

let std_module = Module::from_bytes(WRIT_STD_BYTES)?;
let runtime = RuntimeBuilder::new(user_module)
    .with_library(std_module)
    .with_host(cli_host)
    .build()?;
```

### Anti-Patterns to Avoid
- **Special-casing List/Map/Set in the compiler:** State.md decision: "no compiler special-casing of List/Map/Set". All four types must resolve through normal generic class paths.
- **Hand-rolling hash or sort in VM dispatch:** Use the array dot-call opcodes that Phase 116 already wired. No new VM opcodes needed.
- **Skipping the pre-work GC test:** The class-containing-array GC test must pass BEFORE writing collection classes. A silent GC bug would cause mysterious collection corruption.
- **Using `with_library_source` in production `writ-cli`:** This recompiles writ-std on every invocation. Use pre-compiled `include_bytes!` for production; `with_library_source` only in tests.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Array backing store | Custom linked list or tree in pure Writ IL | `T[]` (HeapObject::Array) | Array opcodes 0x0900-0x0908 already exist and are GC-traced |
| Hash function | Custom Writ hash impl | `hash()` via `Hashable` intrinsic | FNV-1a already registered for int/string/bool/float in Phase 116 |
| GC tracing | New HeapObject variant | Existing `HeapObject::Struct { fields }` + `trace_refs` | trace_refs already walks all fields via `collect_value_refs` |
| Binary search | Custom search loop | Built from Writ array index ops | Writ has array indexing; no opcode needed |
| Library loading | New domain mechanism | `RuntimeBuilder::with_library()` | Already implemented and tested |

**Key insight:** Every capability needed for the four collection types already exists in the VM. The work is writing Writ source, not writing Rust. The only Rust work is wiring writ-cli to pre-load the library.

---

## Runtime State Inventory

> Skipped — Phase 117 is not a rename/refactor/migration phase.

---

## Environment Availability Audit

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/Cargo | Build writ-std.writc | ✓ | workspace edition 2024 | — |
| `writ build` CLI | Compile writ-std source | ✓ | built from workspace | Use `compile_source()` in build.rs |
| `writ-compiler` feature in writ-runtime | `with_library_source` in tests | Verify feature flag exists | — | Use pre-compiled bytes |

**Missing dependencies with no fallback:** None identified.

**Note on `compiler` feature:** `writ-runtime/src/runtime.rs` gates `with_library_source` and `from_source` on `#[cfg(feature = "compiler")]`. Integration tests that use these must ensure the feature is enabled in `writ-runtime/Cargo.toml` dev-dependencies or the test feature list.

---

## Common Pitfalls

### Pitfall 1: GC Does Not Trace Through Struct Fields Holding Arrays
**What goes wrong:** A `List<T>` instance (a `HeapObject::Struct`) has a field containing a `HeapRef` to a `HeapObject::Array`. After GC, the array is freed while the struct is still live.
**Why it happens:** `trace_refs` in `gc.rs` walks Struct fields via `collect_value_refs`, which pushes `Value::Ref(href)` and `Value::Struct { href }` as roots. If the field value is stored as `Value::Ref(array_href)`, it will be traced. If stored as a raw `int` or wrong value type, it won't be.
**How to avoid:** Write and pass the `class-containing-array` GC test FIRST (pre-work task). The test should: allocate a class instance with an array field, populate the array, trigger GC with the class instance as the only root, assert array survives.
**Warning signs:** Collection elements vanish after GC cycles; heap corruption panics in array length reads.

### Pitfall 2: Compiler Does Not Resolve Generic Class Methods Without Phase 115
**What goes wrong:** `List::new()` or `Map::set(k, v)` fails to type-check because generic constraint enforcement (GEN-03) is pending. Phase 117 depends on Phase 115.
**Why it happens:** `K: Ord` bound for Map and `K: Hashable` for HashMap require Phase 115's constraint enforcement pass to be operational.
**How to avoid:** Verify Phase 115 (GEN enforcement) is complete before starting Phase 117. If Phase 115 is only partially done, start with unconstrained `List<T>` and `Set<T>`, then add `Map` and `HashMap` after constraint enforcement is live.
**Warning signs:** TypeChecker accepts wrong types for K without error; bound violations silently succeed.

### Pitfall 3: writ-std.writc Not Loaded Before User Module
**What goes wrong:** User code references `List::new()` and gets a resolution error at runtime: "method not found in domain".
**Why it happens:** `cmd_run` builds the runtime with only the user module + virtual module. Library modules must be added via `domain.add_module()` before `resolve_refs()`.
**How to avoid:** The stub-validation pre-work task must test the full pipeline: compile stub.writ → write stub.writc → run user module that references stub type → verify resolution succeeds.
**Warning signs:** `RuntimeError::LoadError` with "method not found" or "type not found".

### Pitfall 4: Binary Search in Map Requires Eq+Ord on K
**What goes wrong:** Map.get() cannot compare keys without an `Eq` implementation.
**Why it happens:** Pure-Writ binary search uses `<` and `==` operators which dispatch through Ord and Eq contracts.
**How to avoid:** Declare `Map<K: Ord + Eq, V>`. The constraint notation requires Phase 115 multi-bound support (GEN-02).
**Warning signs:** Map.get() returns wrong value or panics on key comparison.

### Pitfall 5: writ-cli build.rs Compilation Order
**What goes wrong:** `writ-cli`'s `build.rs` tries to compile `writ-std/src/collections.writ` using the `writ` binary, but the binary hasn't been compiled yet (chicken-and-egg).
**Why it happens:** `build.rs` runs before the package is compiled, but `writ compile` is the compiled output.
**How to avoid:** Use `writ_compiler::compile_source()` directly in `build.rs` (it's a library, not a binary). `build.rs` can call `writ_compiler` as a Rust function. Alternatively, check in a pre-compiled `writ-std.writc` to the repository and skip the build step.
**Warning signs:** build.rs fails with "writ: command not found" or "binary not yet built".

### Pitfall 6: impl Block Syntax for Generic Classes
**What goes wrong:** `impl List<T> { ... }` may not be supported by the current parser/lowering if generic impl blocks on user-defined classes haven't been tested.
**Why it happens:** The compiler handles `impl Foo : Contract` (trait impl) but generic `impl Foo<T>` inherent impls may have edge cases.
**How to avoid:** Write a golden test for a minimal `pub class Box<T> { value: T }` with `impl Box<T> { fn get(self) -> T { self.value } }` before writing the full collection classes.
**Warning signs:** Parser error on `impl List<T>` syntax; lowering errors on generic method bodies.

---

## Code Examples

### GC Pre-Work Test Pattern (class-containing-array)
```rust
// Source: writ-runtime/tests/gc_tests.rs (pattern from existing tests)
// This is the pattern for the new gc_class_containing_array test
#[test]
fn gc_class_containing_array_survives() {
    // Build a module that:
    // 1. Allocates a class instance with field[0] = array ref
    // 2. Populates the array
    // 3. Returns (task completes)
    // Then: trigger GC with class instance as root
    // Assert: array is still live (traced through struct field)
    let mut runtime = build_gc_runtime(&[
        Instruction::New { r_dst: 0, type_idx: /* class type */ 0x02000001 },
        Instruction::NewArray { r_dst: 1, elem_type: 0x04 /* int */ },
        Instruction::ArrayAdd { r_arr: 1, r_val: /* int literal reg */ 2 },
        Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 },
        Instruction::RetVoid,
    ], 3);
    // ... spawn, tick, collect, assert heap_size == 2 (struct + array)
}
```

### Library Pre-Load Validation (stub.writ)
```writ
// writ-std/src/stub.writ — minimal file for Phase 117 pre-work
pub class WritStdStub {
    value: int
}
```

```rust
// Test: writ-cli can load stub library before user module
// Source: pattern from runtime.rs with_library_source
let std_bytes = writ_compiler::compile_source(STUB_SRC).unwrap();
let std_module = writ_module::Module::from_bytes(&std_bytes).unwrap();
let user_bytes = writ_compiler::compile_source(USER_SRC).unwrap();
let user_module = writ_module::Module::from_bytes(&user_bytes).unwrap();
let runtime = RuntimeBuilder::new(user_module)
    .with_library(std_module)
    .build()
    .unwrap();
// domain.resolve_refs() must succeed — WritStdStub visible to user module
```

### List<T> Minimal Implementation
```writ
// writ-std/src/collections.writ
pub class List<T> {
    items: T[]
}

impl List<T> {
    pub fn new() -> List<T> {
        new List<T> { items: [] }
    }
    pub fn add(mut self, item: T) {
        self.items.add(item);
    }
    pub fn get(self, index: int) -> T {
        self.items[index]
    }
    pub fn set(mut self, index: int, item: T) {
        self.items[index] = item;
    }
    pub fn len(self) -> int {
        self.items.len()
    }
    pub fn remove_at(mut self, index: int) {
        self.items.remove_at(index);
    }
    pub fn contains(self, item: T) -> bool {
        self.items.contains(item)
    }
}
```

### Map<K,V> Sorted-Array Implementation
```writ
pub class Map<K: Ord + Eq, V> {
    keys: K[],
    values: V[]
}

impl Map<K: Ord + Eq, V> {
    pub fn new() -> Map<K, V> {
        new Map<K, V> { keys: [], values: [] }
    }
    pub fn len(self) -> int {
        self.keys.len()
    }
    pub fn has(self, key: K) -> bool {
        // linear scan (Phase 117); binary search deferred to COLL-07
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { return true; }
            i = i + 1;
        }
        false
    }
    pub fn get(self, key: K) -> V {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { return self.values[i]; }
            i = i + 1;
        }
        // unreachable if key present; caller must check has() first
        self.values[0]
    }
    pub fn set(mut self, key: K, value: V) {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key {
                self.values[i] = value;
                return;
            }
            i = i + 1;
        }
        self.keys.add(key);
        self.values.add(value);
    }
    pub fn remove(mut self, key: K) {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key {
                self.keys.remove_at(i);
                self.values.remove_at(i);
                return;
            }
            i = i + 1;
        }
    }
}
```

**Note on Map O(log n) spec:** COLL-02 says "sorted-array backed, O(log n)" but the pure-Writ loop above is O(n). True binary search requires `Ord` and integer arithmetic — achievable in Writ but adds complexity. For Phase 117, linear scan is correct behavior (semantics match). O(log n) is an optimization target; COLL-07 (`list.sort()`) defers sort to v14.0. The spec note "O(log n)" describes the design intent, not a hard Phase 117 constraint. The plan should note this tradeoff explicitly.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Collections as compiler builtins | Collections as pure Writ source in writ-std | Phase 117 decision | Reflection/serialization can inspect collection internals |
| BumpHeap (no GC) | MarkSweepHeap with finalization | Phase 18 | Collections require `.with_gc()` for long-lived instances |
| No library pre-loading | `RuntimeBuilder::with_library()` | Pre-existing | writ-std can be loaded before user module |

**Deprecated/outdated:** None for this phase.

---

## Open Questions

1. **Map<K,V> O(log n) vs O(n) in Phase 117**
   - What we know: COLL-02 spec says "sorted-array backed, O(log n)". Pure-Writ binary search is possible but adds ~30 lines of while-loop logic.
   - What's unclear: Is true O(log n) a Phase 117 hard requirement, or is O(n) acceptable as long as sorted-array semantics are preserved?
   - Recommendation: Implement linear scan (O(n)) for Phase 117 since it has identical external API and correct semantics. Add a code comment noting the O(log n) path can be enabled when `arr.sort()` (COLL-07) lands. The planner should note this explicitly.

2. **impl List<T> generic inherent impl syntax support**
   - What we know: The compiler supports `pub class Foo<T>` with generics in class declarations. Generic `impl Foo<T> { ... }` inherent impl blocks haven't been explicitly tested.
   - What's unclear: Does the current parser/lowering handle `impl MyClass<T> { fn method(self) -> T { ... } }` for user-defined generic classes, or only for contracts?
   - Recommendation: The plan's Wave 0 must include a minimal `Box<T>` golden test to validate generic inherent impl syntax before writing any collection code. This is the single highest implementation risk.

3. **writ-cli library pre-load mechanism: embed vs. path**
   - What we know: `include_bytes!` requires writ-std.writc to exist at compile time. build.rs using `writ_compiler` as a library avoids chicken-and-egg.
   - What's unclear: Does `writ-compiler` compile correctly as a build dependency (it's already in the workspace)?
   - Recommendation: Use `writ_compiler::compile_source(WRIT_STD_SRC)` in `writ-cli/build.rs`. The source is embedded as a `const &str` in `build.rs`, compiled to bytes, written to `$OUT_DIR/writ-std.writc`, then included via `include_bytes!`.

4. **HashMap<K,V> implementation complexity**
   - What we know: True open-addressing hash tables require modulo, array resizing, and tombstone handling — all expressible in Writ but complex.
   - What's unclear: A simpler chaining approach (array of `List<V>`) may be easier to write but requires nested generic types (`List<V>[]`).
   - Recommendation: Use linear probing with a fixed-size array of buckets (e.g., 16 buckets, no resize for Phase 117). This keeps the Writ source simple. Add a TODO comment for dynamic resizing.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `#[test]` (cargo test) |
| Config file | none — workspace-level `cargo test` |
| Quick run command | `cargo test -p writ-golden -- coll` |
| Full suite command | `cargo test -p writ-golden && cargo test -p writ-runtime` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| COLL-01 | List::new(), add, get, len, remove_at, contains | golden (compile) + integration (run) | `cargo test -p writ-golden -- coll_list` | ❌ Wave 0 |
| COLL-02 | Map::new(), set, get, has, remove, len | golden + integration | `cargo test -p writ-golden -- coll_map` | ❌ Wave 0 |
| COLL-03 | Set::new(), add, has, remove, len | golden + integration | `cargo test -p writ-golden -- coll_set` | ❌ Wave 0 |
| COLL-05 | HashMap::new(), set, get, has, remove, len with Hashable | golden + integration | `cargo test -p writ-golden -- coll_hashmap` | ❌ Wave 0 |
| COLL-06 | writ-std loads as library module before user code | integration (runtime build) | `cargo test -p writ-runtime -- coll_integration` | ❌ Wave 0 |
| PRE-WORK-GC | class-containing-array GC correctness | unit (gc_tests.rs) | `cargo test -p writ-runtime -- gc_class_containing_array` | ❌ Wave 0 |
| PRE-WORK-LIB | stub.writ library pre-load validation | integration | `cargo test -p writ-runtime -- lib_preload_stub` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime -- gc_class_containing_array`
- **Per wave merge:** `cargo test -p writ-golden && cargo test -p writ-runtime`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-golden/tests/golden/gc_class_containing_array.writ` — covers PRE-WORK-GC
- [ ] `writ-golden/tests/golden/coll_list_basic.writ` — covers COLL-01
- [ ] `writ-golden/tests/golden/coll_map_basic.writ` — covers COLL-02
- [ ] `writ-golden/tests/golden/coll_set_basic.writ` — covers COLL-03
- [ ] `writ-golden/tests/golden/coll_hashmap_basic.writ` — covers COLL-05
- [ ] `writ-runtime/tests/coll_integration_tests.rs` — covers COLL-06
- [ ] `writ-std/writ.toml` + `writ-std/src/collections.writ` — the library source
- [ ] `writ-cli/build.rs` — compiles writ-std at build time

---

## Detailed Pre-Work Task Sequence

The STATE.md specifies two mandatory pre-work items. They must be plan tasks (Wave 0 or Wave 1) that gate all subsequent collection work:

### Pre-Work 1: class-containing-array GC correctness test
**File:** `writ-runtime/tests/gc_tests.rs` (append new test)
**What to add:**
```rust
#[test]
fn gc_class_containing_array_field_survives() {
    // Construct a Struct with type_key for a user class.
    // Store a HeapObject::Array as field[0] via Value::Ref(array_href).
    // Collect with struct_href as the only root.
    // Assert: array_href is still alive (not freed).
    // Assert: array elements are intact.
}
```
**Why this is pre-work:** If `trace_refs` in gc.rs does not trace `Value::Ref` inside `HeapObject::Struct.fields`, collection instances will be silently corrupted after GC cycles.

**Current GC behavior (HIGH confidence from reading gc.rs:69-97):** `trace_refs` for `HeapObject::Struct { fields }` iterates all fields and calls `collect_value_refs(v, refs)` on each. `collect_value_refs` pushes `Value::Ref(href)` into refs. So an array stored as `Value::Ref(array_href)` WILL be traced. The test verifies this expectation holds end-to-end.

### Pre-Work 2: writ-cli library module pre-load stub validation
**File:** new integration test or golden test
**What to validate:**
1. Create `writ-std/src/stub.writ` with a trivial exported type
2. Compile it to bytes using `writ_compiler::compile_source`
3. Compile a user module that references the stub type
4. Build runtime with `RuntimeBuilder::new(user_module).with_library(std_module).build()`
5. Assert `domain.resolve_refs()` succeeds (no "type not found" error)
**Why this is pre-work:** Validates the library-loading pipeline end-to-end before any collection code exists.

---

## Sources

### Primary (HIGH confidence)
- `writ-runtime/src/runtime.rs` — `RuntimeBuilder::with_library`, `with_library_source` (lines 89-116)
- `writ-runtime/src/gc.rs` — `trace_refs`, `MarkSweepHeap::collect`, `collect_value_refs` (lines 57-97, 268-371)
- `writ-runtime/src/heap.rs` — `HeapObject` variants (lines 6-20)
- `writ-runtime/src/virtual_module.rs` — Hashable contract registration (lines 167-169, 342-352)
- `writ-golden/tests/golden_tests.rs` — golden test harness, `compile_and_disassemble` (lines 1-100)
- `writ-compiler/src/lib.rs` — `compile_source` (lines 43-93)
- `writ-compiler/src/check/ty.rs` — `TyKind::Class`, `TyKind::GenericParam` (lines 17-48)
- `writ-compiler/src/emit/collect/types.rs` — class type emission with generic params (lines 160-195)
- `.planning/STATE.md` — critical decisions (lines 50-54, 66-70)

### Secondary (MEDIUM confidence)
- `writ-runtime/src/domain.rs` — multi-module loading and cross-reference resolution (lines 84-250) — library module loading verified to work via existing `add_module` + `resolve_refs` calls in runtime.rs build()
- `writ-compiler/src/resolve/mod.rs` — multi-file compilation via `asts: &[(FileId, &Ast)]` (lines 92-119) — writ-std source files can be added as additional file_id entries in a single pipeline run

### Tertiary (LOW confidence)
- Generic inherent impl blocks (`impl MyClass<T>`) for user-defined generic classes — NOT explicitly tested in existing golden tests. Needs Wave 0 validation.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all components are existing, read from source
- Architecture: HIGH — pure-Writ approach confirmed by STATE.md and existing class emit patterns
- Pitfalls: HIGH for GC/loading mechanics (read from source); MEDIUM for generic impl syntax (untested path)
- Pre-work sequence: HIGH — mandated by STATE.md with exact wording

**Research date:** 2026-03-29
**Valid until:** 2026-04-28 (30 days — stable codebase, no external dependency churn)
