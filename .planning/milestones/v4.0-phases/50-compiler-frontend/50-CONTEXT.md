# Phase 50: Compiler Frontend - Context

**Gathered:** 2026-03-13
**Status:** Ready for planning

<domain>
## Phase Boundary

The compiler parses `class` declarations, generates correct inline initialization for value-struct construction, generates correct heap allocation for class construction, and rejects infinitely-sized recursive structs at compile time. Covers COMP-01 through COMP-04. No runtime/VM changes (Phase 49 already handles kind-dependent NEW dispatch).

</domain>

<decisions>
## Implementation Decisions

### Lifecycle Hook Enforcement
- Parser-level enforcement: `struct_decl` parser does NOT accept `on create` or `on finalize` hooks
- `class_decl` parser accepts all hooks (create, finalize, serialize, deserialize)
- `struct_decl` parser still accepts `on serialize` and `on deserialize` hooks (value-type structs can be serializable)
- Parse-and-report approach: if `on create`/`on finalize` appears inside a struct, the parser recognizes the hook syntax, parses it fully, then emits an error diagnostic (avoids cascading parse errors)
- Error message: "lifecycle hooks are not allowed on value-type structs; use `class` for reference types with hooks"
- Note: Phase 47 spec removed ALL hooks from structs; serialize/deserialize staying requires a small spec amendment

### Recursive Struct Detection
- Runs after type checking (field types fully resolved)
- Full cycle chain in error message: "recursive struct: Foo contains Bar (field `inner`) which contains Foo (field `parent`)"
- Error includes suggestion: "help: consider using `class` instead of `struct` for reference semantics"
- Applies to value-type structs only (kind=0) — classes use heap indirection, self-reference is fine
- DFS walk of struct field types to detect direct and transitive cycles

### Type System Modeling
- New `TyKind::Class(DefId)` variant added to the type checker (separate from `TyKind::Struct(DefId)`)
- New `DefKind::Class` and `DefKind::ExternClass` variants in name resolution
- Type-checking rules are identical for struct and class — both support fields, impl blocks, generics, contracts
- The only differences are at codegen time (kind=0 vs kind=4) and at parse time (hook restrictions)
- Classes work identically to structs as generic type arguments — no boxing restrictions
- Error message display style: Claude's discretion (match existing diagnostic patterns)

### Claude's Discretion
- Exact error message display for type names (prefixed with kind or just name)
- CST/AST representation: whether ClassDecl is a separate type mirroring StructDecl, or a shared type with discriminant
- Import organization for new DefKind/TyKind variants
- Test organization for new parser and type-checking tests
- How to integrate cycle detection into the existing post-type-check pipeline

</decisions>

<specifics>
## Specific Ideas

- Phase 47 spec removed all lifecycle hooks from structs, but user wants serialize/deserialize to remain on structs — spec amendment needed (structs keep `on serialize`/`on deserialize`, lose `on create`/`on finalize`)
- Recursive struct error should read like rustc's "recursive type has infinite size" but with Writ's `class` as the suggested fix instead of Rust's `Box<T>`
- Parse-and-report pattern for hook rejection ensures good error recovery — parser doesn't bail out on seeing `on` in a struct

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Token::KwStruct` in lexer.rs: pattern for adding `Token::KwClass`
- `StructDecl` / `StructMember` in cst.rs: template for `ClassDecl` / `ClassMember`
- `struct_decl` parser combinator in parser.rs (lines 2570-2589): template for `class_decl` parser
- `AstStructDecl` in ast/decl.rs: template for `AstClassDecl`
- `lower_struct()` in lower/mod.rs: template for `lower_class()`
- `TypeDefKind::Class = 4` already exists in writ-module/src/tables.rs (Phase 48)
- `DefKind` enum in resolve/def_map.rs: add `Class` and `ExternClass` variants

### Established Patterns
- Parser uses chumsky combinators with `just(Token::KwStruct).ignore_then(...)` pattern
- CST Item enum dispatches to specific declaration types
- Lowering follows `lower_*` function naming convention per declaration type
- Name resolution uses `DefKind` enum for type discrimination
- Type checker uses `TyKind` enum with `DefId` for nominal types
- Codegen collect.rs gathers types and emits TypeDefs with kind dispatch

### Integration Points
- `writ-parser/src/lexer.rs`: Add `KwClass` token
- `writ-parser/src/cst.rs`: Add `Item::Class`, `ClassDecl`, `ClassMember`
- `writ-parser/src/parser.rs`: Add `class_decl` parser, modify `struct_decl` to reject create/finalize hooks
- `writ-compiler/src/ast/decl.rs`: Add `AstDecl::Class`, `AstClassDecl`
- `writ-compiler/src/lower/mod.rs`: Add `lower_class()`, dispatch from Item::Class
- `writ-compiler/src/resolve/def_map.rs`: Add `DefKind::Class`, `DefKind::ExternClass`
- `writ-compiler/src/resolve/collector.rs`: Handle `AstDecl::Class` variant
- `writ-compiler/src/check/ty.rs`: Add `TyKind::Class(DefId)`
- `writ-compiler/src/check/check_decl.rs`: Add Class variant handling, cycle detection post-check
- `writ-compiler/src/emit/collect.rs`: Emit `TypeDefKind::Class` for class defs
- `writ-compiler/src/emit/body/expr.rs`: `extract_type_def_id` add Class match arm

</code_context>

<deferred>
## Deferred Ideas

- Spec amendment for serialize/deserialize hooks on structs — needs a small update to Phase 47's §8 changes (structs keep `on serialize`/`on deserialize`)

</deferred>

---

*Phase: 50-compiler-frontend*
*Context gathered: 2026-03-13*
