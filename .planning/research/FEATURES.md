# Feature Landscape

**Domain:** Standard library and language ergonomics for an existing Writ game scripting toolchain (v13.0 milestone)
**Researched:** 2026-03-29
**Confidence:** HIGH — primary sources are the Writ codebase (direct inspection), official C#/.NET docs, Kotlin docs, Swift docs, and ariadne crate docs; WebSearch used for ecosystem cross-checks.

---

## Context: What Already Exists

These capabilities are already shipped and form the hard dependency foundation for all v13.0 features.

| Existing Capability | Relevance to v13.0 |
|---|---|
| `T[]` fixed-size array with NewArray / ArrayInit / ArrayLen / ArrayAdd / ArrayRemove / ArrayInsert / ArraySlice instructions (v2.0) | Array primitives are the backing store for List; no alloc/copy instructions are missing — the VM instruction set is already sufficient |
| `Iterable<T>` and `Iterator<T>` contracts declared in virtual module with `iterator()` and `next()` methods (v2.0) | `for x in collection` desugaring requires these to be in the dispatch table with concrete implementations; the contracts exist but no user-facing collection type implements them yet |
| `for … in` loop parsed by CST, `AstStmt::For` lowered, type-checked for `TyKind::Array` and `Range` (v3.0) | The for-in path already compiles; extending it to user-defined `Iterable<T>` is a type-checker + emitter change, not a parser change |
| C-style `for` loop works; `for x in array` and `for x in range` work (v3.0) | Iterator protocol for `List<T>` is a pure extension to the existing for-in path |
| Generics with type parameters, no bounds (v3.0) | Generic constraints require adding bound-checking to resolver and type-checker; the generic parameter table already exists |
| Contract system with CALL_VIRT O(1) dispatch (v2.0) | `<T: Contract>` bounds enforce this existing dispatch mechanism at a new point in the pipeline |
| `string.len()` intrinsic (v12.0 fix) | Other string utilities (split, trim, etc.) extend the same IntrinsicId pattern |
| `writ-diagnostics` Diagnostic struct with `secondary_labels`, `help`, `notes` fields — struct fully supports multi-span (v5.0–v12.0 incremental) | The data model is complete; gaps are in which errors actually use secondary labels and in LSP partial-parse recovery |
| ariadne crate — multi-span rendering already wired up in `render.rs` | ariadne supports arbitrary multi-span, multi-file labels; the render path already calls `with_label()` for secondary labels |
| Reflection runtime (typeof, FieldInfo, MethodInfo, dynamic invocation) (v11.0) | Iterator intrinsics follow the same IntrinsicId extension pattern established for reflection |

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features a developer reasonably expects in a scripting language that has generics and a contract system. Missing any of these makes the language feel incomplete for writing real programs.

| Feature | Why Expected | Complexity | Depends On |
|---------|--------------|------------|------------|
| `<T: Contract>` generic bounds — enforced at call sites and instantiation sites | Every language with both generics and contracts/interfaces uses bound syntax: C# `where T : IComparable`, Kotlin `: Comparable<T>`, Swift `: Equatable`; without it, generic functions cannot call any methods on `T` | MEDIUM | Existing GenericParam table; resolver/typechecker changes only — no IL format changes |
| Multiple bounds `<T: ContractA + ContractB>` | C# multi-constraint (`where T : IComparable, IDisposable`), Kotlin intersection bounds — any user who writes a sort function will want `Eq + Ord` simultaneously | LOW (incremental once single-bound works) | Single-bound constraint pass |
| `List<T>` — ordered growable collection written in Writ | Table stakes in every scripting language: GDScript `Array`, Lua tables, Kotlin `MutableList`, C# `List<T>`, Swift `Array` (value-type) — absence forces users to hand-roll all collection logic | MEDIUM | Array primitives (all exist); `Iterable<T>` contract impl; `Eq` bound for contains() |
| `Map<K, V>` — key-value associative collection written in Writ | Lua tables, GDScript `Dictionary`, Kotlin `HashMap`, C# `Dictionary<K,V>` — the second collection users always reach for | HIGH | `Eq` + `Ord` bounds on `K`; Array primitives for bucket storage; or a Rust-backed extern approach |
| `Set<T>` — unordered unique collection written in Writ | Kotlin `HashSet`, Swift `Set`, C# `HashSet<T>` — deduplication is a fundamental game data pattern (unique inventory, faction membership) | HIGH | `Eq` bound on `T`; relies on the same backing strategy as Map |
| `for x in list` — `Iterable<T>` desugaring in the type-checker | Every scripting language that has a for-in loop and a list type makes them work together; the contracts (`Iterable<T>`, `Iterator<T>`) already exist in the virtual module — wiring them up is what users expect | MEDIUM | Iterable/Iterator contracts (exist); type-checker for-in path extension; CALL_VIRT dispatch |
| String `split(sep: string) -> string[]` | C# `String.Split`, Kotlin `String.split`, GDScript `String.split` — first thing users need for any text processing | LOW | StringSplit intrinsic |
| String `trim() -> string` | C# `String.Trim`, Kotlin `String.trim`, GDScript `String.strip_edges` — universal expectation | LOW | StringTrim intrinsic |
| String `starts_with(prefix: string) -> bool` | C# `StartsWith`, Kotlin `startsWith`, Swift `hasPrefix` | LOW | StringStartsWith intrinsic |
| String `ends_with(suffix: string) -> bool` | C# `EndsWith`, Kotlin `endsWith`, Swift `hasSuffix` | LOW | StringEndsWith intrinsic |
| String `contains(substr: string) -> bool` | C# `Contains`, Kotlin `contains`, Lua `string.find` | LOW | StringContains intrinsic |
| String `replace(from: string, to: string) -> string` | C# `Replace`, Kotlin `replace`, GDScript `replace` | LOW | StringReplace intrinsic |
| String `to_upper() -> string` and `to_lower() -> string` | C# `ToUpper/ToLower`, Kotlin `uppercase/lowercase` — expected for any NPC name display or command normalization | LOW | StringToUpper / StringToLower intrinsics |
| Multi-span diagnostics used consistently on constraint violations | Rust's compiler sets the standard — "T doesn't implement Eq" should show the call site AND the constraint declaration; rustc, C# Roslyn, and Kotlin both do this; users reading error messages expect it | LOW (data model exists, gaps are in which errors fire secondary labels) | writ-diagnostics SecondaryLabel (already shipped); just requires consistent use at new constraint-check sites |

### Differentiators (Competitive Advantage)

Features that go beyond baseline expectations and give Writ a distinct advantage in its game scripting niche.

| Feature | Value Proposition | Complexity | Depends On |
|---------|-------------------|------------|------------|
| Chainable `.map<U>()`, `.filter()`, `.reduce<U>()` on `List<T>` | Kotlin/Swift/C# LINQ-style chaining is not found in GDScript or Lua — game designers scripting quest logic naturally reach for "filter inventory items then map to names"; this is the single highest user-experience differentiator for the standard library | HIGH | `<T: …>` bounds; closures (already work); `List<T>`; for-in protocol |
| `List<T>` implements `Iterable<T>` — works in `for x in` with zero extra syntax | Swift/Kotlin collections "just work" in for loops; GDScript `for x in array` works but does NOT work on user-defined collection types (GDScript limitation); Writ can do better by making the protocol first-class | MEDIUM | for-in Iterable desugaring; List Iterable impl |
| `Eq` and `Ord` auto-constraint checking — `map.get(key)` requires `K: Eq` verified at the call site with a clear error | C# and Kotlin both enforce this at compile time; GDScript and Lua are dynamic and silently fail or do pointer equality; Writ's static verification is the correct model | MEDIUM | Generic constraint enforcement pass |
| Fix suggestions in constraint errors ("add `impl Eq for Foo`") | Rust's "help: consider implementing…" text dramatically improves developer experience; the `help` field on `Diagnostic` exists and is already rendered by ariadne; paying for this at constraint-violation sites costs almost nothing extra | LOW | Constraint error messages with help text |
| `string.split_on(sep: string) -> List<string>` returning `List<T>` rather than `string[]` | Once List<T> exists, split should return the growable type for chaining; C# in modern usage prefers LINQ `Split` chains over raw arrays | LOW (incremental after List) | `List<T>`; StringSplitList intrinsic or writ-std method |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic for Writ | What to Do Instead |
|---------|---------------|--------------------------|-------------------|
| HashMap / BTreeMap backed by Rust foreign-function calls | "Performance — write the hash table in Rust, not Writ" | Creates a hard host boundary that blocks the reflection system, serialization, and pure-Writ tooling from inspecting Map internals; also forces a non-trivial extern registration story for generic types | Write Map/Set in pure Writ backed by the existing Array primitives; use a sorted array with binary search for O(log n) lookup if hash is unavailable, or accept a Writ-side open-addressing hash table once Eq/Ord constraints work |
| `any` typed collection (`List` without `<T>`) | "I want a heterogeneous list like Lua tables" | Destroys all type safety; every element access requires a type check or crash; the reflection system already provides the heterogeneous object model via `any[]` at reflection boundaries | Use `any[]` (boxed Array) for heterogeneous data at reflection API boundaries; keep List<T> strictly typed |
| `.sort()` on `List<T>` in v13.0 | "Sort is a basic collection operation" | Sort requires `Ord` bound which requires generic constraint enforcement being complete; sort also requires an O(n log n) algorithm (quicksort/mergesort) that is non-trivial to write in Writ until the standard library matures | Defer sort to v14.0 or later; users can sort via a host-side extern until then |
| Warning suppression pragmas (`#suppress W0006 on line X`) | "Too many false positives" | Introduces a new pragma parsing path and a second opt-out mechanism alongside `[Conditional]`; false positives should be fixed at the source, not suppressed | Fix false-positive warnings at the source; use `[Conditional]` for intentional conditional compilation; defer suppression syntax to a future milestone |
| `Iterator<T>` as a user-subclassable class | "I want custom lazy iterators" | Custom iterators require heap-allocated iterator state objects (like Java's `ListIterator`) which complicates GC lifetime; lazy iterator chains (like Kotlin Sequences or Java Streams) require coroutine suspension or closures — the cooperative yielding model complicates lazy iteration | Provide eager `.map()` / `.filter()` on List<T> that return new List<T>; defer lazy iterators to post-v13.0; the for-in protocol supports custom Iterable<T> impls without requiring lazy chains |
| LSP autocomplete for partially-typed generic type arguments (`List<T` mid-edit) | "IDE should autocomplete the type arg" | Partial parse of generic type arguments is ambiguous in the CST at `<` — the parser sees it as a comparison; this is a known issue (parser disambiguation for `f<T>()` is already solved for complete expressions, not partial edits) | Handle existing complete-expression completions correctly first; partial generic type arg completion requires a recovery parser pass deferred to v14.0+ |

---

## Feature Dependencies

```
[Generic bounds — <T: Contract>]
    └──required by──> [Map<K, V>] (K: Eq, K: Ord for key comparison)
    └──required by──> [Set<T>] (T: Eq)
    └──required by──> [List<T>.contains()] (T: Eq)
    └──required by──> [List<T>.map/filter/reduce] (correct bound propagation)
    └──required by──> [Fix suggestions in constraint errors]
    └──builds on──> [GenericParam table (already exists v2.0)]
    └──builds on──> [Contract CALL_VIRT dispatch (already exists v2.0)]
    └──requires──> [Resolver: bound collection on GenericParam nodes]
    └──requires──> [Type-checker: bound satisfaction check at call sites]

[Array primitives — alloc/copy/shrink]
    └──note──> ALL needed array primitives already exist in the VM:
               NewArray, ArrayInit, ArrayLen, ArrayAdd, ArrayRemove,
               ArrayInsert, ArraySlice — no new IL instructions needed
    └──required by──> [List<T> backing store]
    └──required by──> [Map<K, V> bucket array]
    └──required by──> [Set<T> slot array]

[List<T>]
    └──requires──> [Array primitives (all exist)]
    └──requires──> [Iterable<T> contract impl on List<T>]
    └──requires──> [Generic bounds (for contains, map, filter, reduce)]
    └──enables──> [for x in list]
    └──enables──> [map/filter/reduce chains]
    └──enables──> [Map<K, V> and Set<T>] (share the same backing pattern)

[Map<K, V>]
    └──requires──> [List<T> or Array primitives for bucket storage]
    └──requires──> [Generic bounds: K: Eq (at minimum), K: Ord for sorted variant]

[Set<T>]
    └──requires──> [List<T> or Array primitives]
    └──requires──> [Generic bounds: T: Eq]

[for x in collection — Iterable<T> desugaring]
    └──requires──> [Iterable<T> / Iterator<T> contracts (already in virtual module)]
    └──requires──> [Type-checker: for-in path extended from Array-only to Iterable<T>]
    └──requires──> [Emitter: CALL_VIRT to iterator() + next() instead of direct ArrayLoad loop]
    └──enables──> [for x in List<T>]
    └──builds on──> [AstStmt::For already lowered and checked for Array/Range (v3.0)]

[String utilities — split/trim/starts_with/ends_with/contains/replace/to_upper/to_lower]
    └──no new dependencies — each is a new IntrinsicId variant + virtual module method entry]
    └──builds on──> [String intrinsics pattern: StringAdd, StringLen, StringEq, etc. (v2.0–v12.0)]

[Multi-span diagnostics — consistent use on constraint violations]
    └──data model already complete: SecondaryLabel, help, notes all in Diagnostic (v5.0)]
    └──ariadne render path already wires secondary labels (v5.0)]
    └──requires──> [Generic constraint check sites to use with_secondary() and with_help()]
    └──no new infrastructure — discipline, not capability]
```

---

## MVP Definition

### Launch With (v13.0 core)

The minimum coherent set that makes v13.0 genuinely useful for real Writ programs.

- [ ] `<T: Contract>` single bound and multi-bound (`T: A + B`) — enforced at resolver and type-checker; fix suggestion in errors
- [ ] Array primitives: confirm no gaps; spec §3.9 passes for alloc/copy/shrink patterns
- [ ] `List<T>` — push, pop, get(i), set(i, v), len(), contains(v: T) (requires `T: Eq`), written in pure Writ backed by Array
- [ ] `List<T>` implements `Iterable<T>` — `for x in list` works
- [ ] `Map<K, V>` — get(k), set(k, v), contains_key(k), remove(k), keys() -> List<K>, written in Writ
- [ ] `Set<T>` — add(v), remove(v), contains(v), written in Writ
- [ ] String utilities: `split`, `trim`, `starts_with`, `ends_with`, `contains`, `replace`, `to_upper`, `to_lower` — all as intrinsics on `string`
- [ ] Multi-span diagnostics used at all new constraint-violation sites (bound not satisfied, Iterable not implemented)

### Add After Core Validates (within v13.0 later phases)

- [ ] `List<T>.map<U>(fn(T) -> U) -> List<U>` — requires closure + bounds propagation
- [ ] `List<T>.filter(fn(T) -> bool) -> List<T>`
- [ ] `List<T>.reduce<U>(initial: U, fn(U, T) -> U) -> U`
- [ ] LSP: completions for List/Map/Set methods
- [ ] LSP: hover shows inferred element type of `List<T>` variables
- [ ] Diagnostics polish: `for x in expr` where expr is not Iterable shows helpful secondary label pointing to the non-implementing type

### Future Consideration (v14+)

- [ ] `List<T>.sort()` — requires `Ord` bound enforcement complete and a sort algorithm in Writ
- [ ] `EntityList<T>` — typed entity reference collection with component query support (mentioned in spec §1.27.3 but out of scope for v13.0)
- [ ] Lazy iterator chains (Sequence protocol like Kotlin/Swift) — requires coroutine or closure-based approach
- [ ] Warning suppression pragmas
- [ ] Partial generic type arg LSP completion

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Phase Priority |
|---------|------------|---------------------|----------------|
| `<T: Contract>` bounds | HIGH | MEDIUM | P1 — blocks Map/Set/List.contains |
| Array primitive gap analysis (spec audit) | HIGH | LOW | P1 — must confirm no VM gaps before writing List |
| `List<T>` | HIGH | MEDIUM | P1 |
| `List<T>` implements `Iterable<T>` / for-in | HIGH | MEDIUM | P1 |
| String utilities (8 methods) | HIGH | LOW | P1 — independent, high payoff |
| Multi-span diagnostics at constraint sites | MEDIUM | LOW | P1 — discipline not capability |
| `Map<K, V>` | HIGH | HIGH | P2 |
| `Set<T>` | MEDIUM | HIGH | P2 |
| `List<T>.map/filter/reduce` | HIGH | MEDIUM | P2 — after List validated |
| LSP completions for List/Map/Set | MEDIUM | LOW | P2 |
| Diagnostics: non-Iterable for-in error | MEDIUM | LOW | P2 |
| `List<T>.sort()` | LOW | MEDIUM | P3 |
| Lazy iterators | LOW | HIGH | P3 — defer post-v13.0 |

**Priority key:**
- P1: Must have for v13.0 launch — forms the coherent standard library foundation
- P2: Should have — add within v13.0 phases once P1 is validated
- P3: Nice to have — defer to future milestone

---

## Comparable System Analysis

| Feature | C# (.NET) | Kotlin | Swift | GDScript (Godot 4) | Lua | Writ v13.0 Approach |
|---------|-----------|--------|-------|--------------------|-----|---------------------|
| Generic bounds syntax | `where T : IComparable<T>` | `<T : Comparable<T>>` | `<T: Comparable>` | No generics | No generics | `<T: Contract>` (already in spec §1.12, not yet enforced) |
| Multiple bounds | `where T : IA, IB` | `<T> where T : IA, T : IB` | `<T: IA & IB>` | N/A | N/A | `<T: A + B>` (spec §1.12) |
| Growable list | `List<T>` (class) | `MutableList<T>` | `Array<T>` (value) | `Array` (dynamic) | table | `List<T>` in writ-std, backed by Writ Array primitives |
| Map | `Dictionary<K,V>` | `HashMap<K,V>` | `Dictionary<K,V>` | `Dictionary` | table | `Map<K,V>` in writ-std |
| Set | `HashSet<T>` | `HashSet<T>` | `Set<T>` | No built-in Set | No built-in Set | `Set<T>` in writ-std |
| for-in protocol | `IEnumerable<T>` / `GetEnumerator()` | `Iterable<T>` / `iterator()` → `hasNext()` + `next()` | `Sequence` / `makeIterator()` → `next() -> T?` | `for x in array` (array only) | `for k,v in pairs(t)` | `Iterable<T>` / `iterator() -> Iterator<T>` / `next() -> T?` — contracts already in virtual module |
| String split | `String.Split(sep)` → `string[]` | `String.split(sep)` → `List<String>` | `String.components(separatedBy:)` → `[String]` | `String.split(delimiter)` → `Array` | `string.gmatch` (pattern-based) | `string.split(sep) -> string[]` as intrinsic |
| String trim | `String.Trim()` | `String.trim()` | `String.trimmingCharacters(in:)` | `String.strip_edges()` | No built-in | `string.trim() -> string` as intrinsic |
| String contains | `String.Contains(s)` | `String.contains(s)` | `String.contains(s)` | `String.contains(s)` | `string.find` | `string.contains(sub) -> bool` as intrinsic |
| Multi-span errors | Roslyn: primary + "related information" spans | Kotlin: primary + secondary spans | Swift: primary + secondary spans | No secondary spans | No diagnostics | ariadne `with_label()` secondary already works — use consistently |

**Key GDScript gap Writ can fill:** GDScript's `for x in` only works on built-in Array and Dictionary — user-defined types cannot implement iteration. Writ's `Iterable<T>` contract-based desugaring makes any type iterable if it implements the two-method protocol.

**Key Lua gap Writ can fill:** Lua has no type-safe collections and no bounds checking. All generics enforcement is at runtime (or absent). Writ's compile-time `<T: Eq>` bounds make `Map<K,V>` and `Set<T>` robust without runtime overhead at call sites.

---

## Sources

- Writ codebase direct inspection: `writ-runtime/src/virtual_module.rs` (Iterable/Iterator contracts), `writ-module/src/instruction.rs` (array instruction set), `writ-compiler/src/check/check_stmt.rs` (for-in type-check path), `writ-diagnostics/src/diagnostic.rs` (Diagnostic struct), `writ-diagnostics/src/render.rs` (ariadne multi-span rendering), `writ-compiler/src/check/ty.rs` (TyKind enum), `language-spec/spec/13_12_generics.md`, `language-spec/spec/28_27_standard_library_builtins.md`, `.planning/PROJECT.md` — HIGH confidence
- [Microsoft Docs: Constraints on type parameters — C#](https://learn.microsoft.com/en-us/dotnet/csharp/programming-guide/generics/constraints-on-type-parameters) — HIGH confidence (official Microsoft docs)
- [Kotlin Docs: Iterators](https://kotlinlang.org/docs/iterators.html) — HIGH confidence (official Kotlin docs)
- [Swift: IteratorProtocol — Apple Developer Documentation](https://developer.apple.com/documentation/swift/iteratorprotocol) — HIGH confidence (official Apple docs)
- [Swift's Sequence inside the compiler: how for loops work internally](https://swiftrocks.com/swift-sequence-inside-the-compiler-how-for-loops-work) — MEDIUM confidence (community, cross-checks with official docs)
- [ariadne — Rust crate docs](https://docs.rs/ariadne/latest/ariadne/) — HIGH confidence (official crate docs; multi-span and secondary labels confirmed)
- [GDScript reference — Godot Engine stable](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_basics.html) — HIGH confidence (official Godot docs)
- [Microsoft Docs: String.Split](https://learn.microsoft.com/en-us/dotnet/api/system.string.split?view=net-10.0) — HIGH confidence (official Microsoft docs)

---

*Feature research for: Writ v13.0 Standard Library & Language Ergonomics*
*Researched: 2026-03-29*
