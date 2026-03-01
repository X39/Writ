# Writ Compiler

## What This Is

A multi-crate Writ language toolchain. Ships a complete compilation pipeline (name resolution, type checking, IL codegen) plus a spec-compliant IL runtime (register-based VM, cooperative task scheduler, entity system with GC, contract dispatch), a text IL assembler/disassembler, and a `writ` CLI for compiling and running Writ programs. `writ compile foo.writ` produces executable .writil modules; `writ run foo.writil` executes them.

## Core Value

Correct, spec-compliant implementation at every layer — lowering matches Section 28 exactly, runtime matches the IL spec exactly — structured so each layer can be extended independently.

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
- ✓ Lexer validation (raw string delimiters, unicode escapes, escape rejection) — v1.1
- ✓ CST type system (qualified paths, rooted flag, DlgDecl attrs/vis) — v1.1
- ✓ Parser v0.4 syntax (`new`, hex/binary, struct hooks, self params, bit-shift, bitwise, impl generics, operator sigs, spawn detached, defer block-only, attribute separator) — v1.1
- ✓ Dialogue lowering (namespace loc keys, slot preservation, choice labels, say/say_localized, speaker scope) — v1.1
- ✓ Entity model (AstDecl::Entity, component slots, all 6 hooks, implicit self, IndexSet) — v1.1
- ✓ Hex/binary literal lowering (radix-aware parse_int_literal, 0xFF → 255, 0b1010 → 10) — v1.2
- ✓ Tech debt cleanup (VERIFICATION.md for Phases 10-13, dead code removal, stale comment fixes) — v1.2
- ✓ IL binary module format (reader/writer, 200-byte header, 21 tables, round-trip identity) — v2.0
- ✓ IL programmatic builder API (ModuleBuilder with fluent API for all 21 table types) — v2.0
- ✓ Register-based VM (91 instructions, match-dispatch loop, typed register file) — v2.0
- ✓ Task execution model (5-state lifecycle, cooperative yielding, defer/crash, atomic sections) — v2.0
- ✓ RuntimeHost trait (NullHost, suspend-and-confirm at 9 transition points) — v2.0
- ✓ Entity system (generation-indexed handles, SPAWN/INIT/DESTROY/IS_ALIVE/GET_OR_CREATE) — v2.0
- ✓ GC (MarkSweepHeap, GcHeap trait, root collection, finalization queue) — v2.0
- ✓ Contract dispatch (CALL_VIRT, HashMap dispatch table, virtual module with 17 contracts) — v2.0
- ✓ Cross-module resolution (Domain, TypeRef/MethodRef/FieldRef name-based lookup) — v2.0
- ✓ Text assembler (lexer, recursive-descent parser, two-pass assembler, forward labels) — v2.0
- ✓ Disassembler (binary-to-text, round-trip fidelity, all 91 instruction mnemonics) — v2.0
- ✓ Runner CLI (`writ` binary with run/assemble/disasm subcommands, CliHost) — v2.0

- ✓ Name resolution (two-pass symbol collection, qualified paths, visibility, generics, fuzzy suggestions) — v3.0
- ✓ Type checking (unification-based generic inference, strict mutability, ?/!/try desugaring, enum exhaustiveness) — v3.0
- ✓ IL code generation — metadata skeleton (21 tables, CALL_VIRT slots, lifecycle hooks, attributes) — v3.0
- ✓ IL code generation — method bodies (all 90 instructions, register allocation, closures, concurrency, debug info) — v3.0
- ✓ `writ compile` CLI (5-stage pipeline, ariadne diagnostics, end-to-end validation) — v3.0
- ✓ Runtime gap closure (lifecycle hook dispatch, generic specialization, string display) — v3.0
- ✓ LocaleDef emission for [Locale] dlg overrides — v3.0

### Active

**v3.1 — Compiler Bug Fixes and Golden File Testing**

- Comprehensive E2E golden file tests for every language feature (compiled .writ -> disassembled IL -> hand-validated)
- Fix invalid IL generation (extra registers, incorrect register types, etc.)
- Fix known tech debt: TYPE-12 closure captures, register type blob offsets, dead code

### Out of Scope

- JIT compilation — reference interpreter must be complete first; separate crate/milestone
- Code generation (LLVM, WASM) — downstream of AST
- Macro system — no macros in current spec
- Optimization passes — premature at this stage
- Escaped brace de-escaping (`{{`/`}}`) — lexer gap in writ-parser, not lowering
- Closure capture classification (by-value vs by-reference) — TYPE-12 stubbed; deferred to future milestone
- async/await for tasks — spec uses cooperative yielding; Rust async futures cannot be inspected or serialized
- Script-defined components — spec says components are extern-only, data-only
- Exception tables — spec uses crash propagation with defer unwinding, not structured exceptions
- Standard library (writ-std) — List<T>, Map<K,V>, utilities deferred to v3.x+

## Context

**Shipped v3.0** with 57,146 LOC Rust (total), 1,100+ tests, 8 phases, 24 plans in 2 days. Full source-to-execution pipeline working.
**Shipped v2.0** with 13,937 LOC Rust (src), 6 phases, 16 plans in 2 days.
**Shipped v1.0-v1.2** with 8,826 LOC Rust (src), 15 phases, 28 plans.
**Tech stack:** Rust 2024 edition, chumsky, logos, insta, thiserror, byteorder, clap, ena, id-arena, rustc-hash, ariadne, strsim.
**Workspace:** `writ-parser` (lexer+CST), `writ-compiler` (lowering + resolve + typecheck + codegen), `writ-module` (IL binary format), `writ-runtime` (VM+scheduler+entities+GC), `writ-assembler` (text IL assembler+disassembler), `writ-diagnostics` (shared error codes), `writ-cli` (`writ` binary with compile+run).
**Language spec:** `language-spec/spec/` (splatted files, v0.4) — Section 28 is the lowering reference; Sections 30-66 are the IL spec.

**Known tech debt:**
- `lower_dlg_text` duplicates `lower_fmt_string` fold logic (code duplication, not correctness)
- TYPE-12: Closure capture list stubbed empty — closures with captured variables execute incorrectly
- RES-09: Speaker validation stub — structure in place, full @Speaker resolution deferred
- Assembler lacks .export/.extern_fn/.component/.locale/.attribute directives
- Register type blob offsets stored as 0 placeholders in assembler
- Dead code: extract_callee_def_id_opt retained after Phase 28 refactor

## Constraints

- **Tech stack**: Rust 2024 edition, must integrate with existing chumsky/logos-based parser output
- **CST dependency**: Lowering consumes `writ-parser::cst` types directly — no intermediate format
- **Spec compliance**: All lowerings must match Section 28; all IL runtime behavior must match Sections 30-66
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
| Singleton speaker assumption for non-param names | Defers entity validation to name resolution phase | ⚠️ Revisit |
| Phases grouped by compiler layer | Natural dependency chain — each layer depends on the one below | ✓ Good |
| Raw string tokens carry verbatim source | CST lossless roundtrip; processing deferred to lowering/semantic | ✓ Good |
| Bracket-inner parser for contextual caret | Prevents expr from consuming `..` operator in range operands | ✓ Good |
| AstEntityDecl with component slots (not fields) | Matches spec: components are host-managed, not inline struct fields | ✓ Good |
| Namespace threading via LoweringContext push/pop/set | Clean API for localization key generation across nested scopes | ✓ Good |
| Speaker scope save/restore at branch boundaries | Prevents speaker leakage across `$ if`/`$ match` branches | ✓ Good |
| `writ-module` as pure-data crate (no VM logic) | Shared between assembler and future compiler backend | ✓ Good |
| VM + Task Execution as one phase | defer/crash/cancel/atomic share per-frame structures | ✓ Good |
| GC finalizer hooks fire as scheduler tasks after sweep | Prevents re-entrant GC corruption | ✓ Good |
| Entity registry generation-indexed handles | Stale-handle detection without UB; free-list recycling | ✓ Good |
| BumpHeap retained as no-op GcHeap for tests | Tests run fast without GC overhead; GcHeap trait makes swap seamless | ✓ Good |
| Contract dispatch table built at domain load time | O(1) CALL_VIRT; no per-call linear scans | ✓ Good |
| Virtual module constructed programmatically at startup | No file on disk; available in every domain automatically | ✓ Good |
| Two-pass assembler with placeholder method bodies | Forward references resolved cleanly; no second file scan needed | ✓ Good |
| Disassembler emits unsupported directives as comments | Round-trip fidelity preserved; parser limitations documented | ✓ Good |
| CliHost resolves extern names at construction time | No per-request heap lookups; clean separation from NullHost | ✓ Good |

---
*Last updated: 2026-03-03 — v3.1 milestone started*
