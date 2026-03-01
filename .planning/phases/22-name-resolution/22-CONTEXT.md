# Phase 22: Name Resolution - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Every identifier in every source file is bound to a declaration. The compiler produces a NameResolved IR where no identifier is an unresolved string. This phase consumes the lowered AST (from Phase 21) and produces a resolved representation. Scope: two-pass symbol collection, using/path/visibility resolution, type name resolution, speaker/attribute validation, and error quality (ambiguity + fuzzy suggestions). Requirements: RES-01 through RES-12.

</domain>

<decisions>
## Implementation Decisions

### Error Diagnostic Style
- Rust-style rich diagnostics: colored spans, source context, multi-span annotations, numbered error codes
- Error codes use E-series numbering (E0001, E0002, etc.) for errors; warning/error code series distinction is Claude's discretion
- ANSI color output from the start, with a `--no-color` flag for CI/piping (red=errors, yellow=warnings, blue=notes, bold=emphasis)
- Every error includes a `help:` note suggesting a fix — always, not just when obvious
- Ambiguous name errors show ALL candidate spans with file:line location and namespace
- Multi-file errors use inline multi-file rendering (all spans in one diagnostic, labeled by file)
- Report all errors — no max error cap, never stop early
- Emit both errors AND warnings (e.g., unused imports, unreachable `using`)
- Warnings suppressible via `[allow(unused_import)]` attributes AND CLI flags (`--no-warn XYZ0000`)
- Writ-specific terminology in error messages (entity, contract, component, dialogue), with flexibility to use "type" for entity where clearer
- Shared `writ-diagnostics` crate: used by parser, compiler, and runtime for unified diagnostic types and rendering
- Diagnostic rendering library: Claude's discretion (evaluate codespan-reporting, ariadne, miette)

### writ-runtime Type Availability
- Implicit prelude: all writ-runtime types (Option, Result, Range, Array, Entity) AND all 17 contracts are always in scope without `using`
- Prelude types are NOT shadowable — defining a type with a prelude name is a compile error with a specific "reserved prelude type" error message
- Unqualified-only access: no `writ_runtime::` namespace prefix. Prelude types have no namespace
- `null` is a hard keyword that always resolves to `Option::None` — independent of prelude mechanism
- Primitives (int, float, string, bool, void) and prelude types use the same built-in scope mechanism in the resolver
- Reserved prelude names are hard-coded in the compiler source (const array)
- Entity is an implicit base type: every `entity Foo { }` implicitly has Entity as its base type; `Entity` parameters accept any entity
- Track original syntax through lowering: errors for `Option<T>` (desugared from `T?`) show both forms: "`Option<string>` (written as `string?`)"
- Name resolution verifies `Array<T>` exists and `T[]` desugars correctly; method resolution (`.push()`, `.pop()`) deferred to type checking
- Generic bounds checking (e.g., `E: Error` on Result) and component-type validation in `use` clauses: Claude's discretion on phase boundary

### Fuzzy Suggestions ("Did you mean?")
- Suggestion count, scope (current vs. unimported namespaces), case sensitivity, and context-awareness: all Claude's discretion
- Must implement RES-12: unresolved names produce "did you mean `survival::HealthPotion`?" style suggestions

### Shadowing Policy
- Local-to-local shadowing (`let x = 1; { let x = 2; }`) is allowed silently — no warning (core language feature)
- Import shadowing: local declaration shadowing a name from `using` produces a warning
- Generic type parameter shadowing outer type names: allowed with warning
- Top-level declaration shadowing a `using`-imported name: Claude's discretion

### Multi-file Compilation Model
- Full `writ.toml` parsing: read `[compiler].sources` directories, recursively discover `.writ` files
- File processing order: non-deterministic (two-pass collection handles it; correctness doesn't depend on order)
- Namespace/path mismatch: emit a warning when file path doesn't match namespace declaration (spec says convention, not requirement)
- Duplicate definitions: two declarations with the same name in the same namespace from different files is a compile error

### Claude's Discretion
- Diagnostic rendering library choice (codespan-reporting vs ariadne vs miette)
- Warning/error code series format (separate E/W series vs unified)
- Prelude injection mechanism (virtual `using` vs direct root scope injection)
- Top-level declaration shadowing `using`-imported names (warning vs silent)
- Fuzzy suggestion implementation details (edit distance, count, scope, case sensitivity, context-awareness)
- Generic bounds checking phase boundary (resolution vs type checking)
- Component-type validation in entity `use` clauses (resolution vs type checking)
- Output IR structure (annotated AST vs new IR type with symbol IDs)
- Impl block association details (orphan rules, cross-namespace impls)

</decisions>

<specifics>
## Specific Ideas

- Error messages should feel like Rust's compiler — high quality, helpful, never cryptic
- "I want the compiler to teach you the language through its errors" — errors should guide developers
- Desugared syntax should always show the original form in parentheses when relevant (e.g., `Option<string>` written as `string?`)
- `.editorconfig` support for diagnostic configuration: noted as a TODO for future work
- Reserved prelude names should be documented in the language spec (follow-up task, outside this phase)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-parser` crate: complete lexer + parser producing full-fidelity CST with spans (lexer.rs, parser.rs, cst.rs)
- `writ-compiler` crate: CST-to-AST lowering pipeline (`lower/mod.rs`) with LoweringContext for error accumulation
- `Ast` type (`ast/mod.rs`): flat list of `AstDecl` items — the input to name resolution
- `AstType` (`ast/types.rs`): Named, Generic, Array, Func, Void — the types that need resolution
- `AstDecl` (`ast/decl.rs`): all declaration forms including Namespace, Using, Fn, Struct, Entity, Enum, Contract, Impl, Component, Extern, Const, Global
- `LoweringContext` (`lower/context.rs`): namespace stack tracking, error accumulation pattern — similar context will be needed for resolution
- `SimpleSpan` from chumsky: byte-offset spans throughout all AST nodes

### Established Patterns
- Error accumulation: all passes append errors to a context, never halt (LoweringContext pattern)
- Owned strings: AST uses `String` not `&str` — no lifetime parameter needed in resolver
- Span preservation: every AST node carries `SimpleSpan` for precise error reporting
- Module structure: `ast/` has mod.rs + per-concern files (expr.rs, stmt.rs, decl.rs, types.rs) — resolver should follow similar organization

### Integration Points
- Input: `Ast` from `lower()` function — flat list of `AstDecl` items
- Output: NameResolved IR (new type or annotated AST — Claude's discretion)
- New crate: `writ-diagnostics` will be a new workspace member for shared diagnostic types
- `writ.toml` parsing: new code needed for project configuration reading
- Workspace: `Cargo.toml` workspace members list needs updating for new crate(s)

</code_context>

<deferred>
## Deferred Ideas

- Document reserved prelude names in the language spec — spec update, not compiler work
- `.editorconfig` support for diagnostic configuration — future CLI/tooling phase
- Language server warning for namespace/path mismatch — language server phase
- `writ-std` standard library types — future, not part of core spec (G3 in IL TODO)

</deferred>

---

*Phase: 22-name-resolution*
*Context gathered: 2026-03-02*
