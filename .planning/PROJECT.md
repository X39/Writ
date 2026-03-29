# Writ Compiler

## What This Is

A multi-crate Writ language toolchain. Ships a complete compilation pipeline (name resolution, type checking, IL codegen), a spec-compliant IL runtime (register-based VM, cooperative task scheduler, entity system with GC, contract dispatch), a text IL assembler/disassembler, a `writ` CLI for compiling and running Writ programs, a language server (writ-lsp) for IntelliSense, a debug adapter (writ-dap) for source-level debugging, and a VS Code extension (writ-vscode) bundling both servers with TextMate syntax highlighting.

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

- ✓ Spec cleanup (§1.2.8 removal, §26.4 inbuilt calls, writ.toml field alignment) — v3.2
- ✓ ChoiceOption rename (dialogue choice type disambiguated from Option<T>) — v3.2
- ✓ Unqualified None/Some (sub-prelude injection, parser `::*` glob expansion) — v3.2
- ✓ fn_log_say_choice golden test fix (check_path normalization, BOM handling) — v3.2
- ✓ Leveled logging (`log::trace`/`debug`/`info`/`warn`/`error` replacing `log(msg)`) — v3.2
- ✓ writ.toml project compilation (`writ build` with `--release`/`--debug` profiles) — v3.2
- ✓ Structs-as-value-types design record (struct/class split decision: YES) — v3.2

- ✓ Struct/class split — `struct`=value type (inline, copy-on-assign, structural equality), `class`=reference type (heap, GC-managed), entity remains kind=2 — v4.0
- ✓ Spec amendments — §8 struct→value type, new §9 Classes, memory/construction/IL type/module format updated — v4.0
- ✓ IL format — TypeDef.kind=4 (class), format_version=3, TypeDefKind enum at API boundaries — v4.0
- ✓ VM runtime — Value::InlineStruct inline registers, kind-dependent NEW, GC tracing through value-struct fields — v4.0
- ✓ Compiler — `class` keyword full pipeline, recursive struct detection, structural equality emission, closure→class migration — v4.0
- ✓ Golden tests updated — format_version=3, struct equality, class declarations, recursive struct error — v4.0

- ✓ VS Code extension with TextMate syntax highlighting, bundled writ-lsp and writ-dap servers — v5.0
- ✓ Language server: diagnostics, completions, hover, go-to-definition, find references, signature help (cross-file via writ.toml) — v5.0
- ✓ Debug adapter: source-level breakpoints, stepping, local variable inspection, call stack, watch expressions, tasks-as-threads — v5.0
- ✓ Debug info pipeline: SourceSpan line/column numbers, debug locals, parser error recovery, RuntimeHost debug hooks — v5.0
- ✓ Semantic highlighting: entities, components, dialogue speakers, types, functions with distinct colors — v5.0

- ✓ Zero clippy warnings across all 9 Rust crates (194 warnings eliminated) — v6.0
- ✓ 12 oversized files split into focused submodules across 6 crates — v6.0
- ✓ Duplicate code consolidated (lower_dlg_text → lower_fmt_string delegation) — v6.0
- ✓ Module boundaries tightened (explicit imports, pub(crate) narrowing, module doc headers) — v6.0
- ✓ Dead code removed and cross-phase regressions fixed (say() ABI, golden tests) — v6.0
- ✓ LSP namespace completions for log::, Option::, Result::, and user-defined enums — v6.1
- ✓ LSP dot-completion pipeline verified for struct and array receivers — v6.1
- ✓ SWITCH/DeferPush byte-offset encoding fixed in IL serializer — v6.1
- ✓ Multi-file writ.toml project launch through DAP debug adapter — v6.1
- ✓ Golden test coverage for dialogue/function mix patterns (dlg_fn_mix, dlg_quest_pattern) — v6.1

- ✓ Cross-language benchmark suite (7 benchmarks across 6 languages) — v7.0
- ✓ Docker-based reproducible benchmark runner (run.sh + run.ps1) — v7.0
- ✓ SVG chart generation (pygal) and markdown results tables — v7.0
- ✓ GitHub Actions CI workflow for automated benchmark runs — v7.0
- ✓ Benchmark results committed to `benchmark/results/` — v7.0

- ✓ VM hot-path inlining (#[inline(always)] on extract helpers, #[inline] on dispatch functions) — v7.1
- ✓ Zero-allocation call convention (direct register-to-register arg copies via split_at_mut) — v7.1
- ✓ Frame register pool (RegisterPool with acquire/release, POOL_CAP=64) — v7.1
- ✓ Inner dispatch loop (execute_batch amortizes scheduler overhead) — v7.1
- ✓ Copy-semantic Value enum (InlineStruct→HeapRef, Value derives Copy) — v7.1
- ✓ Clone cleanup and unsafe register indexing in hot arg-copy loops — v7.1
- ✓ Profile-guided optimization (PGO) pipeline — fib(40) 83.297s→27.103s (-67.5%) — v7.1

- ✓ mdBook documentation site with language reference, getting started guide, and architecture overview — v9.0
- ✓ Rust API docs (cargo doc) hosted alongside mdBook on gh-pages — v9.0
- ✓ GitHub Actions CI workflow for auto-deploy on push to master — v9.0
- ✓ Language spec reformatted as browsable mdBook chapters — v9.0
- ✓ Getting started guide (installation, hello world, first script, writ CLI) — v9.0
- ✓ Architecture overview (compiler pipeline, crate structure, contribution guide) — v9.0

- ✓ Attribute argument blob encoding (AttrValue, ATTR_TAG_*, round-trip encoder/decoder in writ-module) — v10.0
- ✓ User-defined attribute declarations (`attribute Name(args);`) through full pipeline — v10.0
- ✓ [Deprecated("msg")] compiler warning W0006 with same-file suppression + LSP diagnostic/hover — v10.0
- ✓ [Conditional("name")] emit-time function elision via --condition flag + fallback verification — v10.0
- ✓ @speaker validation E0007 for non-[Singleton] entities, E0003 for non-existent — v10.0
- ✓ Builtin attributes in writ-runtime virtual module namespace — v10.0
- ✓ Runtime query_attributes API (query by name, by type, by value) + pre-load callback — v10.0
- ✓ LSP E2E tests for W0006/E0007 + language spec sections 1.17.5-1.17.7 — v10.0

- ✓ Reflection spec (§1.28) with 6 reflection types, typeof/get_type semantics, Reflectable contract, dynamic invocation rules — v11.0
- ✓ TypeOf opcode (0x0A30), format_version 4, §2.18.9 Reflection Types in writ-runtime virtual module — v11.0

- ✓ writ-module TypeOf instruction (0x0A30, RI32), format_version=4, assembler/disassembler typeof mnemonic — v11.0

- ✓ Virtual module reflection types (6 class TypeDefs, Reflectable contract 19, 4 primitive intrinsics) — v11.0

- ✓ ReflectionIndex lazy cache, GC roots, 22 reflection intrinsics, TypeOf/GetType dispatch, integration tests — v11.0

- ✓ typeof() compiler pipeline: lexer→parser→AST→lowering→TyKind::ReflectionType→TypeOf emission — v11.0

- ✓ Reflectable auto-impl: every user TypeDef gets ImplDef + get_type() TYPEOF+RET body — v11.0

- ✓ Read-only introspection E2E tests (12 runtime + 6 golden + 2 LSP), typeof equality, GC survival, static-vs-dynamic — v11.0

- ✓ Dynamic invocation: FieldInfo.set (mut/readonly enforcement), MethodInfo.invoke (scheduler-driven), 6 integration tests — v11.0

- ✓ Generic reflection: is_generic, type_args(), MethodInfo/FieldInfo.attributes(), spec documentation — v11.0

- ✓ Closure capture fix (closures with captured variables execute correctly) — v12.0
- ✓ s.len() returns byte length, not heap slot index — v12.0
- ✓ ::choice with fn() {} lambdas serializes without UnexpectedEof — v12.0
- ✓ Assembler directive completeness (.export, .extern_fn, .component_slot, .locale, .attribute) + real register type blob offsets — v12.0
- ✓ Spec §26.4 TOC entry + `using log::*` limitation documented — v12.0
- ✓ test_fn_optional registered and passing in golden test suite — v12.0
- ✓ LSP Option/Result completions driven by TypeEnv.prelude_enum_variants + orphaned re-export removed — v12.0
- ✓ DAP per-frame source file attribution via method_file_ids + dialogue {expr} interpolation golden test — v12.0

- ✓ Generic constraints (`<T: Contract>` bounds enforcement, multi-span diagnostics, IL GenericConstraint table emission) — v13.0
- ✓ Array primitives (dot-call methods on `T[]`, ArrayContains opcode) — v13.0
- ✓ String utilities (split, trim, starts_with, ends_with, contains, replace, to_upper, to_lower as Rust intrinsics) — v13.0
- ✓ Hashable builtin contract (auto-implemented for int, string, bool, float) — v13.0
- ✓ Host value construction API (type-validated struct/class construction from Rust, ImmediateWithHeap handler) — v13.0
- ✓ Collections (List<T>, Map<K,V>, Set<T>, HashMap<K,V> in pure Writ, loaded as library module) — v13.0
- ✓ Iterator protocol (Iterable<T>/Iterator<T> contracts, for-in desugaring, map/filter/reduce on List<T>) — v13.0
- ✓ Diagnostics polish (multi-span constraint errors, fix suggestions, --deny-warnings, LSP partial-parse recovery) — v13.0

- ✓ Array semantics correction (T[] is allocation-explicit, growth methods removed, resize/copy_from added, format_version 5) — v14.0
- ✓ Stdlib rewrite (List/Map/Set/HashMap internals use resize+indexed assignment, 72 golden tests pass) — v14.0
- ✓ Cross-module type resolution (DefMap injection from .writc, compile_with_libraries API, virtual module injection, writ.toml [dependencies]) — v14.0

### Active

(None yet — next milestone pending)

### Out of Scope

- JIT compilation — reference interpreter must be complete first; separate crate/milestone
- Code generation (LLVM, WASM) — downstream of AST
- Macro system — no macros in current spec
- Optimization passes — premature at this stage
- Escaped brace de-escaping (`{{`/`}}`) — lexer gap in writ-parser, not lowering
- Closure capture classification (by-value vs by-reference) — TYPE-12 capture list fix is in scope for v12.0; full by-value/by-reference classification deferred
- async/await for tasks — spec uses cooperative yielding; Rust async futures cannot be inspected or serialized
- Script-defined components — spec says components are extern-only, data-only
- Exception tables — spec uses crash propagation with defer unwinding, not structured exceptions
- Standard library (writ-std) — shipped in v13.0
- Language Server Protocol (LSP) / Debug Adapter Protocol (DAP) — shipped in v5.0, fixes in v6.1

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
| check_path strips :: from first segment only | Matches lower/expr.rs encoding for root-qualified paths | ✓ Good |
| ChoiceOption atomic four-layer rename | Prevents partial rename from creating inconsistent state | ✓ Good |
| Sub-prelude injection for None/Some | User-defined symbols shadow without error; no DefKind change needed | ✓ Good |
| FileId(u32::MAX) sentinel for synthetic entries | Distinguishes compiler-injected log DefIds from user-declared ones | ✓ Good |
| log:: namespace via synthetic ExternFn DefIds | Reuses existing infrastructure; no new DefKind or resolver pass needed | ✓ Good |
| run_pipeline() synchronous shared helper | Thread spawning stays in callers; clean separation of concerns | ✓ Good |
| struct/class split (YES) for v4.0 | struct=value type, class=reference type; matches Rust/C# mental model | ✓ Good |
| Entity remains kind=2 (not kind=4) | VM treats kind=2 and kind=4 identically for heap alloc; entity-specific features key off kind=2 | ✓ Good |
| format_version=3 strict (no backward compat) | Pre-1.0 project; reader rejects anything else with UnsupportedVersion | ✓ Good |
| TypeDefKind enum at API boundaries | Compile-time safety; builder takes TypeDefKind not u8; invalid kinds caught at compile time | ✓ Good |
| Copy derive removed from Value | Explicit .clone() for multi-word copy semantics; prevents accidental shallow copies of InlineStruct | ✓ Good |
| Folder module splits for oversized files | Each subfile gets single responsibility; parent mod.rs re-exports public API | ✓ Good |
| Delegation pattern for lower_dlg_text | Converts DlgTextSegment→StringSegment then delegates to lower_fmt_string; eliminates fold duplication | ✓ Good |
| pub(crate) narrowing for internal modules | Exposes dead code to compiler analysis; caught 6 genuinely unused items | ✓ Good |
| Documented exceptions for 500-line target | parser/program.rs (Chumsky recursive()), module_builder.rs (single struct), dialogue.rs (tightly-coupled), resolver.rs (core algorithm) | ✓ Good |
| by_fqn prefix scan for log:: namespace completions | inject_log_namespace bypasses def_map.insert(); pub_members_of returns empty | ✓ Good |
| Hardcoded Option/Result namespace variants | Prelude types not in type_env.enum_variants; pragmatic workaround | ✓ Fixed v12.0 — prelude_enum_variants field on TypeEnv |
| Fix SWITCH offsets in serialize.rs Pass 4 | Encoding concerns belong in serializer, not emitter; instruction-index-based emitter stays clean | ✓ Good |
| source_paths Vec replacing source_path Option | Multi-file tracking for DAP; per-frame attribution deferred (FileId not in SourceSpan) | ✓ Good |
| Three-stage Dockerfile for benchmark container | Writ build, Rust bench build, Ubuntu runtime with 6 interpreters; clean layer separation | ✓ Good |
| Host-side chart generation (pygal) | Container produces raw.json only; charts outside container avoids Python dep in Docker image | ✓ Good |
| Dated result subdirectories | `benchmark/results/YYYY-MM-DD/` prevents overwrite, enables historical diff | ✓ Good |
| Writ compile/run split as first-class metrics | `compile_ms` and `run_ms` separate JSON fields; distinguishes interpreter startup from compilation | ✓ Good |
| CI numbers not authoritative | 15% regression threshold; publishable numbers from local Docker runs on stable machine | ✓ Good |
| IMPL-METHOD fix for contract dispatch | Intercept Field callee on Struct/Class receiver via methoddef_token_by_type_and_name | ✓ Good |
| Array growth methods removed, resize/copy added | T[] is fixed-size; dynamic behavior belongs on List<T> | ✓ Good |
| format_version=5 strict (no backward compat) | Pre-1.0 project; reader rejects anything else | ✓ Good |
| DefMap injection before collect_declarations | Library types must be visible during Pass 1 user code scanning | ✓ Good |
| Virtual module through same mechanism as libraries | No special-casing; CLI pushes build_writ_runtime_module() into library list | ✓ Good |
| Synthetic FileId(u32::MAX-1-lib_index) for library types | Avoids collision with existing FileId(u32::MAX) sentinel for log namespace | ✓ Good |

## Current State

v14.0 Array Semantics & Cross-Module Resolution complete. All 3 phases shipped: Phase 120 (Array Semantics Correction), Phase 121 (Stdlib Rewrite), Phase 122 (Cross-Module Type Resolution). T[] is allocation-explicit, stdlib uses resize+indexed assignment, compiler loads types from .writc library modules with compile-time validation.

## Context

**Shipped v7.1** with 8 phases, 12 plans in 1 day. VM hot-path optimization: fib(40) 83.297s→27.103s (-67.5%), PGO pipeline, 35 requirements satisfied.
**Shipped v7.0** with 79,005 LOC Rust (total), 5 phases, 12 plans in 1 day. Cross-language benchmark suite with Docker, SVG charts, CI workflow.
**Shipped v6.1** with 74,997 LOC Rust, 3 phases, 5 plans in 1 day. LSP completions, DAP runtime fixes, dialogue golden tests.
**Shipped v6.0** with 74,227 LOC Rust, 5 phases, 14 plans in 1 day. Structural cleanup across all crates.
**Shipped v5.0** with 72,591 LOC Rust, 10 phases, 20 plans in 4 days. Language server, debug adapter, VS Code extension.
**Shipped v4.0** with 61,853 LOC Rust, 5 phases, 12 plans in 1 day. Struct/class split across spec, IL format, VM, and compiler.
**Shipped v3.0-v3.2** with 24 phases, 61 plans. Full compiler pipeline, golden tests, spec corrections, tooling.
**Shipped v1.0-v2.0** with 21 phases, 44 plans. Lowering pipeline and IL runtime.
**Tech stack:** Rust 2024 edition, chumsky, logos, insta, thiserror, byteorder, clap, ena, id-arena, rustc-hash, ariadne, strsim, tower-lsp, dap. TypeScript for VS Code extension. Python (pygal) for benchmark charts.
**Workspace:** `writ-parser` (lexer+CST), `writ-compiler` (lowering + resolve + typecheck + codegen), `writ-module` (IL binary format), `writ-runtime` (VM+scheduler+entities+GC), `writ-assembler` (text IL assembler+disassembler), `writ-diagnostics` (shared error codes), `writ-cli` (`writ` binary with compile+run+build), `writ-lsp` (language server), `writ-dap` (debug adapter), `writ-vscode` (VS Code extension), `benchmark/` (Docker harness, Python charts, CI workflow).
**Language spec:** `language-spec/spec/` (splatted files, v0.4) — Section 28 is the lowering reference; Sections 30-66 are the IL spec; §8 Structs (value types), §9 Classes (reference types).

**Shipped v13.0** with 5 phases, 12 plans. Standard library: generic constraints, array primitives, string utilities, collections (List/Map/Set/HashMap), iterator protocol, diagnostics polish.
**Shipped v12.0** with 6 phases, 6 plans. Tech debt cleanup: closure capture fix, runtime bug fixes, assembler completeness, spec/test housekeeping, LSP completions refactor, DAP fixes.
**Shipped v11.0** with 9 phases, 17 plans. Runtime reflection: typeof, Reflectable auto-impl, 6 reflection types, dynamic invocation, generic reflection.
**Shipped v10.0** with 7 phases, 13 plans. Attribute system: blob encoding, user-defined attributes, [Deprecated]/[Conditional] semantic effects, speaker validation, runtime query API + pre-load callback.
**Shipped v9.0** with 6 phases, 13 plans. mdBook documentation site, language reference, getting started guide, cargo doc, GitHub Actions CI/deploy.
**Shipped v8.0** with 4 phases, 5 plans. Contract-as-type: TyKind::Contract, assignability, CALL_VIRT emission, LSP support.
**Shipped v14.0** with 3 phases, 8 plans in 1 day. Array semantics correction, stdlib rewrite, cross-module type resolution.
**20 milestones shipped** (v1.0-v14.0): 122 phases, 254 plans.

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-29 after v14.0 milestone*
