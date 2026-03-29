# Technology Stack

**Project:** Writ v13.0 Standard Library & Language Ergonomics
**Researched:** 2026-03-29
**Confidence:** HIGH — all findings derived from direct codebase inspection; no external library additions required

---

## Context: What This Is NOT

This is not a new project and not a question of "which framework to use." Every technology choice is constrained by the existing 10-crate Rust workspace. All v13.0 work is **purely internal source changes** to existing crates. No new Cargo.toml entries are expected or recommended.

The question this document answers: what existing stack components need to change, what's already in place, and what is explicitly out of scope.

---

## Existing Stack (Unchanged)

All of the following are already locked and working. Do not change versions.

| Crate | Version (locked) | Role | Consuming Crate(s) |
|-------|-----------------|------|--------------------|
| `chumsky` | 0.12.0 | Recursive-descent parser combinators | `writ-parser`, `writ-diagnostics` |
| `logos` | 0.16.1 | Lexer codegen | `writ-parser` |
| `ariadne` | 0.6.0 | Terminal diagnostic rendering with colored spans | `writ-diagnostics`, `writ-parser` (dev) |
| `thiserror` | 2.0.18 | Structured error enum derivation | `writ-compiler`, `writ-diagnostics`, `writ-runtime` |
| `rustc-hash` | 2.1.1 | `FxHashMap` for O(1) dispatch tables and def maps | `writ-compiler`, `writ-runtime` |
| `id-arena` | 2.3.0 | Arena-allocated `DefId` handles for name resolution | `writ-compiler` |
| `ena` | 0.14.4 | Union-find for type unification (`InferVar`) | `writ-compiler` |
| `byteorder` | 1.5.0 | Little-endian IL binary encoding | `writ-module` |
| `insta` | 1.46.3 | Snapshot testing for golden lowering output | `writ-compiler` (dev) |
| `strsim` | 0.11.1 | Fuzzy name suggestions in resolver errors | `writ-compiler` |
| `walkdir` | 2.5.0 | Directory traversal for `writ build` | `writ-compiler` |
| `toml` + `serde` | 0.9 / 1.x | `writ.toml` project config parsing | `writ-compiler` |
| `tower-lsp` | 0.20.0 | LSP server protocol | `writ-lsp` |
| `tokio` | 1.x | Async runtime for LSP/DAP servers | `writ-lsp` |
| `dashmap` | 6.1.0 | Concurrent document store in LSP | `writ-lsp` |
| `lsp-types` | 0.94 | LSP protocol types | `writ-lsp` |
| `clap` | 4.5.60 | CLI argument parsing | `writ-cli` |

---

## What Must Change for v13.0

The six feature areas map to five crate-level work areas. None require a new external dependency.

---

### Area 1: Generic Constraints — `writ-compiler`

**Current state (verified by inspection):**

The full pipeline from syntax to constraint enforcement is already present but incomplete at the IL emission step:

- Parser (`writ-parser/src/parser/generic_params.rs`): Parses `<T: Bound + Other>` syntax. `GenericParam.bounds: Vec<Spanned<TypeExpr>>` is populated.
- Lowering (`writ-compiler/src/lower/mod.rs`): `lower_generic_param` converts CST bounds to `AstGenericParam.bounds: Vec<AstType>`.
- Type environment (`writ-compiler/src/check/env_build.rs`): `build_generic_bounds` converts bounds to `Vec<Vec<DefId>>` stored on `FnSig.bounds`.
- Type checker (`writ-compiler/src/check/check_expr/call.rs`): `check_contract_bounds` enforces bounds at call sites; emits `TypeError::UnsatisfiedBound` (E0112) when unmet.
- IL builder (`writ-compiler/src/emit/module_builder.rs`): `add_generic_constraint()` exists but is **never called** from collect passes.

**What is missing:** Constraint emission to IL (table 14 `GenericConstraintRow`). The type checker enforces bounds correctly at the Rust level; the emitted binary does not encode the constraints, so runtime-level tooling (reflection, future toolchain) cannot read them.

**Changes needed:**

| File | Change |
|------|--------|
| `writ-compiler/src/emit/collect/types.rs` (or equivalent collect pass) | Call `builder.add_generic_constraint(param_row, constraint_token)` for each bound on each generic param during TypeDef emission |
| `writ-compiler/src/emit/collect/fn_defs.rs` (or equivalent) | Same for generic method params |

**What to NOT touch:** Parser, lowering, env_build, check_contract_bounds — all correct. The IL tables in `writ-module` already encode `GenericConstraintRow { param: u32, constraint: MetadataToken }`. No format_version bump needed (table 14 already defined).

---

### Area 2: Array Primitives — `writ-runtime` + `writ-compiler`

**Current state (verified):**

Array mutation IL instructions already exist and are implemented:

| Instruction | Opcode | Runtime dispatch |
|-------------|--------|-----------------|
| `ArrayAdd` | 0x0905 | VM executes: push to `HeapObject::Array.elements` |
| `ArrayRemove` | 0x0906 | VM executes: remove at index |
| `ArrayInsert` | 0x0907 | VM executes: insert at index |
| `ArraySlice` | 0x0908 | VM executes: range-based slice copy |
| `ArrayInit` | 0x0901 | VM executes: fill N elements with base value |
| `ArrayLen` | 0x0904 | VM executes: returns elements.len() |

Virtual module also registers: `array_add`, `array_remove_at`, `array_insert`, `array_contains`, `array_slice`, `array_iterator` as intrinsic methods on `Array<T>`.

**What is missing:** Compiler-side emission for `array_add`, `array_remove_at`, `array_insert`, `array_contains` as dot-call targets. The VM and virtual module already handle these; the type checker needs to resolve `.add(x)` on an `Array<T>` receiver to the correct intrinsic. This is a compiler type-checker / code-emitter gap, not a runtime gap.

**Changes needed:**

| File | Change |
|------|--------|
| `writ-compiler/src/check/check_expr/` (method call resolution) | Recognize `.add`, `.remove_at`, `.insert`, `.contains`, `.slice` as valid `Array<T>` methods; infer element types |
| `writ-compiler/src/emit/body/` (call emission) | Emit `ArrayAdd`/`ArrayRemove`/`ArrayInsert` instructions for these dot-calls |

**No new IL instructions, no Cargo changes needed.**

---

### Area 3: Iterator Protocol — `writ-runtime` + `writ-compiler`

**Current state (verified):**

Contracts exist in virtual module (`writ-runtime/src/virtual_module.rs`):
- `Iterable<T>` (contract 14): one method `iterator()` at slot 0
- `Iterator<T>` (contract 15): one method `next()` at slot 0
- `Array<T>` implements `Iterable<T>` via `array_iterable` intrinsic

For-in loop codegen (`writ-compiler/src/emit/body/stmt.rs`):
- Array receiver: already emits index-counter loop (correct, efficient)
- Range receiver: already emits CmpLtI counter loop (correct)
- Other receivers: emits `Nop` with comment "future: iterator protocol"

**What is missing:**
1. `for x in collection` for non-array non-range iterables: the `_ =>` branch in `emit_for_loop` needs to call `collection.iterator()` then loop `iter.next()` until `Option::None`
2. `ArrayIterator` class (or intrinsic state): `array_iterable` currently returns the array itself. A proper `Iterator<T>` implementation needs `has_next` / `next` state. The spec uses `Option<T>` for `next()` return.
3. Type checker: `for x in expr` where `expr` implements `Iterable<T>` — currently only `Array` and `Range` are accepted; `Contract(Iterable<T>)` receivers need element type extraction.

**Chainable `.map`/`.filter`/`.reduce`:** These are pure-Writ functions (stdlib), not VM intrinsics. They are written as top-level functions in the collection modules taking `Iterable<T>` parameters. No new IL instructions required.

**Changes needed:**

| File | Change |
|------|--------|
| `writ-runtime/src/virtual_module.rs` | Add `ArrayIterator` class with `cursor: int` field; register `ArrayIterator::next()` as intrinsic |
| `writ-runtime/src/dispatch/intrinsics.rs` | Implement `ArrayIteratorNext` intrinsic: returns `Option<T>` (Some(elem) or None) |
| `writ-runtime/src/dispatch/mod.rs` | Add `ArrayIteratorNext` to `IntrinsicId` |
| `writ-compiler/src/check/check_stmt.rs` | Accept `TyKind::Contract(Iterable_def_id)` in `for` loop element type resolution |
| `writ-compiler/src/emit/body/stmt.rs` | Implement the `_ =>` branch: emit `CALL_VIRT Iterable::iterator`, then loop `CALL_VIRT Iterator::next`, branch on `Option::None` |

**No new Cargo dependencies.**

---

### Area 4: String Utilities — `writ-runtime` + `writ-compiler`

**Current state (verified):**

Existing string intrinsics in `IntrinsicId` (verified in `writ-runtime/src/dispatch/mod.rs`):
- `StringAdd`, `StringEq`, `StringOrd`, `StringIndexChar`, `StringIndexRange`
- `StringIntoString`, `StringIntoInt`, `StringIntoFloat`, `StringIntoBool`

Not present anywhere: `split`, `trim`, `starts_with`, `ends_with`, `contains`, `replace`, `to_upper`, `to_lower`, `len` (as a method distinct from array len).

**Implementation path:** Same pattern as existing string intrinsics — add `IntrinsicId` variant, implement in `intrinsics.rs`, register in `virtual_module.rs`, and expose in type checker for method resolution on `string` receivers.

**Changes needed:**

| File | Change |
|------|--------|
| `writ-runtime/src/dispatch/mod.rs` | Add: `StringLen`, `StringSplit`, `StringTrim`, `StringTrimStart`, `StringTrimEnd`, `StringStartsWith`, `StringEndsWith`, `StringContains`, `StringReplace`, `StringToUpper`, `StringToLower` (11 new `IntrinsicId` variants) |
| `writ-runtime/src/dispatch/intrinsics.rs` | Implement all 11: delegate to Rust `str` methods (`trim()`, `to_uppercase()`, etc.) |
| `writ-runtime/src/virtual_module.rs` | Register 11 new intrinsic methods on `string_type` after the existing 6 |
| `writ-compiler/src/check/check_expr/` | Resolve `.split(sep)`, `.trim()`, `.starts_with(prefix)`, etc. as valid `string` method calls |
| `writ-compiler/src/emit/body/` | Emit `CALL_VIRT` targeting string contract impls for these methods |

**Return type conventions:**
- `split(sep: string) -> string[]` — returns `HeapObject::Array` of string refs
- `trim()`, `trim_start()`, `trim_end()` — returns new `HeapObject::String`
- `starts_with(s)`, `ends_with(s)`, `contains(s)` — returns `bool` (native `Value::Bool`)
- `replace(from, to)` — returns new `HeapObject::String`
- `to_upper()`, `to_lower()` — returns new `HeapObject::String`
- `len()` — returns `int` (byte length, consistent with v12.0 fix)

**No new Cargo dependencies.** Rust `std::str` methods cover all of these.

---

### Area 5: Diagnostics Polish — `writ-diagnostics` + `writ-compiler` + `writ-lsp`

**Current state (verified):**

`writ-diagnostics` already supports:
- Multi-span errors: `with_secondary()` adds `SecondaryLabel` entries; ariadne renders them as separate colored spans
- Help text: `with_help()` writes to ariadne `with_help`
- Notes: `with_note()` writes to ariadne `with_note`
- Severity levels: `Severity::Error`, `Severity::Warning`, `Severity::Note` — all render correctly

**What is missing:**
1. **Fix suggestions (inline replacements):** ariadne 0.6.0 does not expose a fix-suggestion API. The current `Diagnostic` struct has no `fixes: Vec<Fix>` field. To add fix suggestions, the approach is: extend `DiagnosticBuilder` with a `with_fix(span, replacement_text)` method and render fixes as a `note:` line in ariadne output (e.g., "suggested fix: change `foo` to `bar`"). This is text-only — no IDE-side apply-fix integration without LSP `textEdit` support.
2. **LSP `textEdit` fix suggestions:** LSP `CodeAction` responses with `WorkspaceEdit` carry the actual fix. This requires `writ-lsp` to produce `CodeAction` items. The code actions file `writ-lsp/src/queries/code_actions.rs` already exists (untracked in git status) — this is the right place.
3. **Warning levels (suppression):** A `#[allow(W0006)]` or similar suppression mechanism is not yet implemented. The `[Conditional]` attribute is the closest existing suppression, but it's function-level. Per-statement suppression is out of scope for v13.0; per-file `//# suppress W0006` could be added as a comment directive if needed, but this is a diagnostic code lookup, not a new library.
4. **LSP partial parse:** chumsky 0.12 already emits partial parse results via its error recovery modes. The LSP backend (`writ-lsp/src/backend.rs`) can be hardened to continue providing completions and hover info even when the file has parse errors. No new dependency; this is an architectural threading change.

**Changes needed:**

| File | Change |
|------|--------|
| `writ-diagnostics/src/diagnostic.rs` | Add `fixes: Vec<FixSuggestion>` to `Diagnostic`; `FixSuggestion { span, file_id, replacement: String }`; `DiagnosticBuilder::with_fix()` method |
| `writ-diagnostics/src/render.rs` | Render fixes as ariadne `with_note` lines: "fix: replace `X` with `Y`" |
| `writ-lsp/src/queries/code_actions.rs` | Implement LSP `textDocument/codeAction` returning `WorkspaceEdit` for fix suggestions |
| `writ-compiler/src/check/error.rs` | Attach fix suggestions to existing errors where clear (e.g., `UnsatisfiedBound` → suggest `impl ContractName for Type { ... }`) |
| `writ-lsp/src/backend.rs` | Wire partial parse: on parse failure, preserve last-good AST for completions/hover |

**No new Cargo dependencies.** ariadne 0.6.0 is sufficient for text-rendered suggestions; LSP fix delivery uses `lsp-types` (already present at 0.94).

---

### Area 6: Collections (`List<T>`, `Map<K,V>`, `Set<T>`) — Pure Writ stdlib

**Implementation approach:** These are written in Writ source, not as Rust intrinsics. They depend on the array primitives (Area 2) and generic constraints (Area 1):

- `List<T>` wraps `T[]` with `.add`, `.remove_at`, `.insert`, `.contains`, `.count` — trivial delegation
- `Map<K: Eq, V>` — a probe-sequence hash map. Requires `Eq` contract bound on `K`. Initial implementation as parallel arrays (`keys: K[]`, `values: V[]`) for correctness; hash-based optimization deferred.
- `Set<T: Eq>` — wraps `T[]` with uniqueness checks on insert.

**No new Rust crates. No new IL instructions.** The entire stdlib lives in `.writ` files loaded as a writ-runtime module or injected as pre-compiled IL.

**Compilation and loading:** Collections are compiled with `writ compile` into a `.writ.bin` module, then injected into `writ-runtime`'s `Domain` before user modules load. The existing module loading pipeline handles cross-module `TypeRef` resolution.

---

## What NOT to Add

| Do Not Add | Why | What to Do Instead |
|------------|-----|--------------------|
| Any new Cargo dependency | The entire feature set is addressable with existing Rust `std` + existing crates | All Rust-side work delegates to `std::str`, `Vec`, `HashMap` internally |
| A hash-map Rust intrinsic for `Map<K,V>` | Ties stdlib design to a specific Rust data structure before correctness is established | Write `Map<K,V>` in pure Writ using parallel arrays; optimize in v14.0 |
| Upgrade ariadne to 0.7+ for fix suggestions | ariadne 0.7 is not yet stable as of 2026-03-29; would require auditing all render callsites | Render fixes as text notes in ariadne 0.6; use `lsp-types` `WorkspaceEdit` for IDE fixes |
| `dashmap` upgrade (5.5.3 → 6.1.0 mismatch) | Both versions are already in the lock file (5.5.3 via transitive dep, 6.1.0 direct) — this is existing, harmless | Do not touch; let cargo resolver handle |
| A new `writ-stdlib` crate | Adds build complexity; stdlib writ source files can live in `writ-runtime/stdlib/` and be compiled at build time via `build.rs` | See area 6 above |
| `for x in expr` desugaring via macro | No macro system in Writ spec | Implement directly in `emit_for_loop` using `CALL_VIRT` |
| JIT or LLVM backend | Out of scope per PROJECT.md | Reference interpreter must be complete first |
| Chainable `.map`/`.filter`/`.reduce` as IL intrinsics | These are higher-order functions — they require passing lambdas, which the IL delegate system already handles | Write as normal Writ functions taking `Iterable<T>` + a lambda |

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| String utilities | Rust `std::str` intrinsics (IntrinsicId variants) | Pure-Writ string methods | String manipulation on `HeapObject::String` requires alloc; Rust is more efficient; pattern already established for `StringAdd` etc. |
| `Map<K,V>` backend | Parallel-array pure Writ | `FxHashMap` Rust intrinsic | `FxHashMap` requires Rust-side intrinsic dispatch and type tagging for keys; parallel arrays work immediately with the existing array primitives; hash optimization is a single v14.0 phase |
| Iterator protocol | `Option<T>`-returning `next()` | Separate `has_next() -> bool` + `next() -> T` | `Option<T>` is idiomatic Writ and avoids TOCTOU bugs on concurrent iterators; matches spec contract 15 already in virtual module |
| Fix suggestions rendering | ariadne `with_note` text + LSP `WorkspaceEdit` | Wait for ariadne fix API | ariadne 0.6.0 has no fix API; text notes work today; LSP `WorkspaceEdit` is the correct IDE-side mechanism |
| Generic constraint IL emission | Emit to table 14 `GenericConstraintRow` | Keep constraints type-checker-only | Table 14 exists and is already serialized; not emitting is a latent bug that blocks reflection-based introspection of bounds |

---

## Version Compatibility

| Component | Current | After v13.0 | Notes |
|-----------|---------|-------------|-------|
| `format_version` | 4 | 4 (no change) | No new opcodes; table 14 was already in format_version 4 |
| `IntrinsicId` variant count | ~67 (post v11.0-v12.0) | ~78 (+11 string utilities; +1 ArrayIteratorNext) | Additive-only; no variant reordering |
| `writ-runtime` virtual module | ~24 contracts | ~24 contracts + `ArrayIterator` class + 11 string methods | additive |
| All Cargo.toml files | unchanged | unchanged | No new external dependencies |

---

## Sources

- `A:/dev/git/Writ/writ-parser/src/parser/generic_params.rs` — confirms `<T: Bound + Other>` parsed; bounds stored in `GenericParam.bounds`
- `A:/dev/git/Writ/writ-compiler/src/check/env_build.rs` — `build_generic_bounds` converts to `Vec<Vec<DefId>>`; bounds on `FnSig` confirmed
- `A:/dev/git/Writ/writ-compiler/src/check/check_expr/call.rs` — `check_contract_bounds` exists and enforces; `UnsatisfiedBound` emitted
- `A:/dev/git/Writ/writ-compiler/src/emit/module_builder.rs` — `add_generic_constraint()` exists but no callsite found
- `A:/dev/git/Writ/writ-module/src/instruction.rs` — `ArrayAdd`/`Remove`/`Insert`/`Slice`/`Init` all present at opcodes 0x0905-0x0908
- `A:/dev/git/Writ/writ-runtime/src/virtual_module.rs` — `Iterable<T>` contract 14, `Iterator<T>` contract 15, `Array<T>` implements `Iterable<T>` confirmed
- `A:/dev/git/Writ/writ-compiler/src/emit/body/stmt.rs` — `emit_for_loop` confirmed: array + range handled, non-array emits `Nop` stub
- `A:/dev/git/Writ/writ-runtime/src/dispatch/mod.rs` — full `IntrinsicId` enum; no string utility variants (split/trim/etc.) present
- `A:/dev/git/Writ/writ-diagnostics/src/diagnostic.rs` — `DiagnosticBuilder` confirmed: `with_secondary`, `with_help`, `with_note` present; no `with_fix`
- `A:/dev/git/Writ/writ-diagnostics/src/render.rs` — ariadne rendering confirmed; no fix suggestion output
- ariadne 0.6.0 docs (fetched 2026-03-29) — no fix/suggestion API in 0.6.0
- `A:/dev/git/Writ/Cargo.lock` — all locked versions confirmed
- `A:/dev/git/Writ/.planning/PROJECT.md` — v13.0 milestone scope and out-of-scope boundaries

---
*Stack research for: Writ v13.0 Standard Library & Language Ergonomics*
*Researched: 2026-03-29*
