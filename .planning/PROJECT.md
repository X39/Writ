# Writ Compiler

## What This Is

A multi-pass compiler for the Writ programming language. Currently ships a complete CST-to-AST lowering pipeline that desugars all Writ higher-level constructs — dialogue blocks, entities, operator overloads, optional sugar, formattable strings, and compound assignments — into their primitive equivalents (functions, structs, dispatch tables) following the language spec Section 28. The architecture is extensible: new desugaring passes require one file and one call site.

## Core Value

Correct, spec-compliant desugaring of all Writ constructs while preserving source span information for useful error messages — structured so new lowering passes can be added without restructuring existing ones.

## Requirements

### Validated

- ✓ AST type hierarchy (AstExpr/AstStmt/AstDecl/AstType, owned types, span preservation) — v1.0
- ✓ Pipeline infrastructure (LoweringContext, error accumulation, pass ordering, public API) — v1.0
- ✓ Optional sugar lowering (`T?` → `Option<T>`, `null` → `Option::None`) — v1.0
- ✓ Formattable string lowering (`$"Hello {name}!"` → concatenation chain) — v1.0
- ✓ Compound assignment desugaring (`+=`/`-=`/`*=`/`/=`/`%=` → expanded form) — v1.0
- ✓ Operator lowering (operator overloads → contract impls, derived operators auto-generated) — v1.0
- ✓ Concurrency pass-through (spawn/join/cancel/defer/detached → AST-level nodes) — v1.0
- ✓ Dialogue lowering (`dlg` → `fn`, three-tier speaker resolution, choice scoping, transitions) — v1.0
- ✓ Localization key generation (FNV-1a auto-keys, `#key` overrides, collision detection) — v1.0
- ✓ Entity lowering (`entity` → struct + ComponentAccess impls + lifecycle hooks + [Singleton]) — v1.0
- ✓ Span preservation (all AST nodes carry source spans, no tombstones) — v1.0
- ✓ Snapshot testing (69 insta tests, integration coverage, determinism verification) — v1.0

### Active

(none — next milestone pending `/gsd:new-milestone`)

### Out of Scope

- Type checking / name resolution — separate compiler phase after lowering
- Code generation (LLVM, WASM, bytecode) — downstream of AST
- Runtime implementation — separate crate (`writ-runtime`)
- Macro system — no macros in current spec
- Optimization passes — premature at this stage
- `?` propagation / `!` unwrap — spec §18 features, deferred beyond v1.0
- Escaped brace de-escaping (`{{`/`}}`) — lexer gap in writ-parser, not lowering

## Context

**Shipped v1.0** with 3,493 LOC Rust (src), 69 snapshot tests, 7 phases, 13 plans in 2 days.
**Tech stack:** Rust 2024 edition, chumsky, logos, insta, thiserror.
**Workspace:** `writ-parser` (lexer+CST), `writ-compiler` (lowering pipeline), `writ-cli`, `writ-runtime`.
**Language spec:** `language-spec/spec.md` — Section 28 is the lowering reference.

**Known tech debt from v1.0:**
- Namespace not threaded to FNV-1a key generation (`DlgLowerState.namespace` always empty)
- `lower_dlg_text` duplicates `lower_fmt_string` fold logic (code duplication, not correctness)
- SUMMARY.md frontmatters have empty `requirements_completed` fields

## Constraints

- **Tech stack**: Rust 2024 edition, must integrate with existing chumsky/logos-based parser output
- **CST dependency**: Lowering consumes `writ-parser::cst` types directly — no intermediate format
- **Spec compliance**: All lowerings must match Section 28 of the language spec exactly
- **Error quality**: Lowering errors must reference original source spans, not lowered positions

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Pipeline lives in `writ-compiler` crate | Natural home — parser produces CST, compiler consumes it | ✓ Good |
| Multi-pass architecture over single-pass | Each construct's lowering is independent; passes are testable in isolation | ✓ Good |
| AST is a separate type hierarchy from CST | CST preserves all syntax; AST only has what semantic analysis needs | ✓ Good |
| Preserve spans through lowering | Error messages after lowering should point to original source | ✓ Good |
| Owned AST types (no `'src` lifetime) | Decouples AST from CST source lifetime; `String`, `Box<T>`, `Vec<T>` | ✓ Good |
| Manual fold pattern over visitor framework | Simpler, direct control; no visitor boilerplate for 7 passes | ✓ Good |
| LoweringContext as shared mutable state | Errors and speaker stack threaded through all passes via `&mut` | ✓ Good |
| Expression helpers before structural passes | Optional, fmt_string, compound helpers shared by dialogue/entity passes | ✓ Good |
| FNV-1a for localization keys | Content-addressed, deterministic, no external crate needed | ✓ Good |
| Singleton speaker assumption for non-param names | Defers entity validation to name resolution phase | ⚠️ Revisit when name resolution is built |

---
*Last updated: 2026-02-27 after v1.0 milestone*
