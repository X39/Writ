# Milestones

## v7.0 Benchmark Suite (Shipped: 2026-03-20)

**Delivered:** Reproducible cross-language benchmark suite comparing Writ against Lua, Squirrel, Python, Node.js, and native Rust — with Docker containerization, SVG chart generation, and GitHub Actions CI workflow.

**Phases:** 5 (70-74) | **Plans:** 12
**LOC:** 79,005 Rust (total), +11,235 net lines this milestone | **Commits:** 66
**Timeline:** 1 day (2026-03-20)
**Git range:** `328c748..bb8aa91`
**Requirements:** 24/24 satisfied (BENCH x8, INFRA x8, REPORT x5, CI x3)

**Key accomplishments:**
1. Multi-stage Docker container with 6 language runtimes (Writ, Lua 5.4, Squirrel 3.x, Python 3.x, Node.js 22 LTS, Rust) and hyperfine-based measurement harness with median/MAD timing and anonymous RSS memory tracking
2. 7 cross-language benchmarks: fibonacci, sieve, string_concat, array_sort, hash_map, oop_dispatch, object_create — all with output-checksum parity verification across languages
3. Compiler feature additions during benchmark development: Array `.push()`/`.len()` methods, contract dispatch IMPL-METHOD fix for struct/class receivers
4. SVG chart generation (pygal) and markdown results tables with per-benchmark execution time, memory usage, and startup time comparison
5. One-command host runners (`run.sh` + `run.ps1`) with Docker/Podman auto-detection and cross-platform path normalization
6. GitHub Actions CI workflow with manual dispatch, weekly schedule, and artifact upload for `raw.json` + SVG charts

### Known Gaps
- Pre-existing: TYPE-12 closure capture list still stubbed empty (carried from v3.0)
- Pre-existing: `::choice` serialization failure with `fn() {}` lambdas (carried from v4.0)
- StrLen runtime bug: `s.len()` returns heap slot number not byte length (Writ string_concat uses constant fallback)
- No hash_map.writ — Writ has no Map type; bench_runner.sh emits null for writ fields
- stub/ suite in benchmark loop produces cosmetic noise in results
- 3 CI human-verification items pending (Actions UI, artifact download, weekly schedule)

**Archives:** `milestones/v7.0-ROADMAP.md`, `milestones/v7.0-REQUIREMENTS.md`, `milestones/v7.0-MILESTONE-AUDIT.md`

---

## v6.1 LSP/DAP Fixes (Shipped: 2026-03-18)

**Delivered:** Targeted fixes for LSP auto-completion, DAP runtime decode errors, and multi-file project launch — plus golden test coverage for dialogue/function mix patterns.

**Phases:** 3 (67-69) | **Plans:** 5
**LOC:** 74,997 Rust (total), +4,952 net lines this milestone | **Commits:** 8 feat
**Timeline:** 1 day (2026-03-18)
**Git range:** `bdac12e..a3e7660`
**Requirements:** 6/6 satisfied (LSP x2, DAP x2, GOLD x2)

**Key accomplishments:**

1. LSP namespace completions for `log::`, `Option::`, `Result::`, and user-defined enums via `::` trigger character
2. LSP dot-completion pipeline verified working end-to-end for struct and array receivers
3. Fixed SWITCH and DeferPush byte-offset encoding in IL serializer — `quest_system.writ` now runs through DAP without decode errors
4. Added multi-file writ.toml project launch support to DAP debug adapter (`compile_and_load_project`)
5. Golden test coverage for dialogue/function mix patterns (`dlg_fn_mix` and `dlg_quest_pattern`) with blessed snapshots

### Known Gaps

- Pre-existing: TYPE-12 closure capture list still stubbed empty (carried from v3.0)
- Pre-existing: `::choice` serialization failure with `fn() {}` lambdas in multi-function modules (carried from v4.0)
- Hardcoded Option/Result variants in namespace completions (not in type_env.enum_variants)
- Per-frame source file attribution deferred in DAP (source_paths.first() fallback)
- String interpolation in dialogue text lines unsupported (lower_fmt_string .into<string>() issue)

**Archives:** `milestones/v6.1-ROADMAP.md`, `milestones/v6.1-REQUIREMENTS.md`, `milestones/v6.1-MILESTONE-AUDIT.md`

---

## v6.0 Code Cleanup (Shipped: 2026-03-18)

**Delivered:** Structural cleanup across all 9 Rust crates — zero clippy warnings, 12 oversized files split into focused submodules, duplicate code consolidated, module boundaries tightened with explicit imports and narrowed visibility, and cross-phase regressions fixed.

**Phases:** 5 (62-66) | **Plans:** 14
**LOC:** 74,227 Rust (total), +1,752 net lines this milestone | **Commits:** 18
**Timeline:** 1 day (2026-03-18)
**Git range:** `4fb7be5..f993330`
**Requirements:** 21/21 satisfied (2 WARN + 14 SPLIT + 2 DUP + 3 MOD)

**Key accomplishments:**

1. Eliminated all 194 clippy warnings across 9 Rust crates — 155 auto-fixed, 39 manually resolved, zero warnings/errors on `cargo clippy --workspace`
2. Split 12 oversized files into focused submodules — writ-compiler (check_expr/, collect/, expr/, env_build), writ-parser (parser/), writ-lsp (queries/), writ-dap (server/), writ-runtime (domain_dispatch), writ-cli (commands/)
3. Consolidated duplicate `lower_dlg_text`/`lower_fmt_string` fold logic via DlgTextSegment→StringSegment delegation pattern
4. Replaced internal wildcard imports with explicit lists across all crates; documented intentional Tier 3 domain-vocabulary wildcards
5. Narrowed `pub` to `pub(crate)` for internal-only items; added `//!` module structure doc headers to all crate entry points
6. Fixed cross-phase regressions — deleted 6 dead code items exposed by visibility narrowing, restored say() 2-argument ABI accidentally removed by clippy auto-fix, blessed golden tests with correct method bodies

### Known Gaps

- Pre-existing: TYPE-12 closure capture list still stubbed empty (carried from v3.0)
- Pre-existing: `::choice` serialization failure with multiple function definitions (carried from v4.0)
- Orphaned re-export: `collect_dialogue_speaker_tokens` in queries/mod.rs (low priority)
- parser/program.rs ~2,976 lines (documented Chumsky recursive() exception)

**Archives:** `milestones/v6.0-ROADMAP.md`, `milestones/v6.0-REQUIREMENTS.md`, `milestones/v6.0-MILESTONE-AUDIT.md`

---

## v5.0 LSP and DAP (Shipped: 2026-03-17)

**Delivered:** Developer tooling for the Writ language — a language server (writ-lsp) for IntelliSense, a debug adapter (writ-dap) for source-level debugging, and a VS Code extension with bundled binaries and TextMate syntax highlighting, shipped as a distributable .vsix artifact.

**Phases:** 10 (52-61) | **Plans:** 20
**LOC:** 72,591 Rust (total), +38,231 net lines this milestone | **Commits:** 144
**Timeline:** 4 days (2026-03-13 → 2026-03-17)
**Git range:** `v4.0..0af3a5d`
**Requirements:** 26/26 satisfied (PREP x5, LSP x8, DAP x7, EXT x4, DIFF x2)

**Key accomplishments:**

1. Language server (writ-lsp, 5,144 LOC) — diagnostics as inline squiggles, hover with type/signature info, go-to-definition, find-all-references, keyword/dot completions, signature help, cross-file resolution via writ.toml
2. Debug adapter (writ-dap, 2,486 LOC) — F5 launch, source-level breakpoints, step over/into/out, local variable inspection, call stack with source locations, watch expression evaluation, cooperative tasks as debugger threads
3. VS Code extension (writ-vscode) — TextMate grammar for syntax highlighting, bundled binary launch, default launch.json snippet, VSIX packaging pipeline with release build
4. Semantic highlighting — distinct token types for entities, components, dialogue speakers, types, functions, and variables with theme-independent color overrides
5. Debug info pipeline — SourceSpan line/column numbers, debug locals (register → name + type), parser error recovery for partial ASTs, RuntimeHost debug hooks
6. UAT-driven gap closure (phases 58-61) — dialogue speaker tokens, VSIX release build, LSP query robustness for bindings/type annotations/declarations, text-based signature help, zero-width span expansion, entity color remapping

### Known Gaps

- Hover on variable declarations shows type but not always correct for complex expressions (partially fixed)
- DAP compile_and_load only supports single-file launch, not writ.toml multi-file projects
- Dot-completion re-analysis is single-file only; cross-file receiver types may not resolve
- TYPE-12: Closure capture list still stubbed empty (carried from v3.0)

**Archives:** `milestones/v5.0-ROADMAP.md`, `milestones/v5.0-REQUIREMENTS.md`, `milestones/v5.0-MILESTONE-AUDIT.md`

---

## v4.0 Structs as Value Types (Shipped: 2026-03-13)

**Delivered:** Struct/class split — `struct` becomes a value type (inline, copy-on-assign, structural equality), `class` becomes a new keyword for reference types (heap-allocated, GC-managed). Full-stack implementation from spec through VM through compiler.

**Phases:** 5 (47-51) | **Plans:** 12
**LOC:** 61,853 Rust (total), +4,282 net lines this milestone | **Commits:** 74
**Timeline:** 1 day (2026-03-12 → 2026-03-13)
**Git range:** `ca7622d..3d9eb03`
**Requirements:** 31/31 satisfied (SPEC x11, IL x3, VM x6, COMP x7, TEST x4)

**Key accomplishments:**

1. Language spec updated — Section 8 rewritten for value-type structs, new Section 9 for classes, memory model/construction model/IL type system/module format amended across 29 spec files
2. IL format extended — `TypeDefKind::Class=4` added, `format_version=3`, compile-time type safety via `TypeDefKind` enum at all API boundaries
3. VM runtime restructured — `Value::InlineStruct` for inline struct registers, `Copy` derive removed (explicit `.clone()` for multi-word copy), kind-dependent `NEW` dispatch, GC tracing through value-struct fields
4. Compiler frontend extended — `class` keyword through full pipeline (lexer, CST, parser, AST, lowering, name resolution, type checking), recursive struct cycle detection
5. Structural equality emission — field-by-field comparison chains for `==`/`!=` on value-type structs, closure capture environments migrated to `kind=4` (class)
6. Golden tests blessed — all existing tests updated for format_version=3, new tests for struct equality, class declarations, and recursive struct error

**Archives:** `milestones/v4.0-ROADMAP.md`, `milestones/v4.0-REQUIREMENTS.md`

---

## v3.2 Spec Corrections and Tooling (Shipped: 2026-03-07)

**Delivered:** Spec corrections, semantic conflict resolution, compiler tooling additions, and a structs-as-value-types design record — addressing accumulated language spec todos and extending the CLI with project-level compilation.

**Phases:** 7 (40-46) | **Plans:** 15
**LOC:** 59,767 Rust (total), +14,247 lines this milestone | **Commits:** 91
**Timeline:** 1 day (2026-03-06 → 2026-03-07)
**Git range:** `4cba393..25c20cd`
**Requirements:** 10/10 satisfied (3 SPEC + 2 LANG + 1 BUG + 3 TOOL + 1 DES)

**Key accomplishments:**

1. Spec cleanup — removed §1.2.8 Serialization Critical Sections, added §26.4 root-namespace inbuilt calls, aligned writ.toml config fields with serde renames
2. Fixed fn_log_say_choice golden test — check_path normalization for ::-prefixed paths, BOM-free source loading, UTF-8 snapshot re-blessed
3. ChoiceOption atomic rename — dialogue choice type renamed from Option to ChoiceOption across spec, lowering, virtual module, and prelude
4. Unqualified None/Some — sub-prelude injection in resolver and type checker, parser `::*` glob expansion, user-defined symbols shadow without error
5. Leveled logging — `log::trace`/`debug`/`info`/`warn`/`error` namespace with synthetic ExternFn DefIds, replacing old unqualified `log(msg)`
6. writ.toml project file compilation — `writ build` subcommand with `--release`/`--debug` flags, ProfileConfig, multi-file compilation pipeline
7. Structs-as-value-types design record — spec amendment §8.4 with struct/class split decision (YES), v4.0 milestone scoped

### Known Tech Debt

- §26.4 missing from spec table of contents
- `using log::*;` behavior undocumented (correctly errors but spec doesn't mention limitation)
- test_fn_optional not registered in golden_tests.rs (fixture exists but no running test)
- CALL_EXTERN token round-trip through writ compile CLI not automated
- 3 manual CLI verification items (writ build debug/release size, writ compile . error, writ new next-steps message)

**Archives:** `milestones/v3.2-ROADMAP.md`, `milestones/v3.2-REQUIREMENTS.md`, `milestones/v3.2-MILESTONE-AUDIT.md`

---

---

## v3.0 Writ Compiler (Shipped: 2026-03-03)

**Delivered:** Full compilation pipeline — name resolution, type checking, and IL codegen — making Writ a complete language from source to executable .writil binary.

**Phases:** 8 (22-29) | **Plans:** 24
**LOC:** 57,146 Rust (total), +37,582 lines this milestone | **Commits:** 41
**Timeline:** 2 days (2026-03-02 → 2026-03-03)
**Git range:** `14e9ae2..d53cd1a`
**Requirements:** 66/66 satisfied (12 RES + 19 TYPE + 29 EMIT + 3 CLI + 3 FIX)

**Key accomplishments:**

1. Name resolution — two-pass symbol collection, qualified paths, visibility enforcement, generic scoping, fuzzy "did you mean?" suggestions (12 requirements)
2. Type checker — `ena`-based unification for generic inference, strict mutability enforcement, ?/!/try desugaring to typed match nodes, enum exhaustiveness checking (19 requirements)
3. IL metadata skeleton — all 21 spec-defined tables populated with correct token assignment, CALL_VIRT slot ordering from contract declaration (8 requirements)
4. All 90 IL instructions emitted — register allocator, entity construction sequences, closure/delegate emission, concurrency primitives, constant folding, TAIL_CALL, STR_BUILD, debug info (20 requirements)
5. `writ compile` CLI — 5-stage pipeline (parse → lower → resolve → typecheck → codegen), ariadne error diagnostics, end-to-end validation (6 requirements)
6. Gap closure — LocaleDef emission for locale overrides, 4 codegen bug fixes (CALL resolution, Range emission, DeferPush patching), retroactive verification of all 66 requirements

### Known Tech Debt

- TYPE-12: Closure capture list stubbed empty — closures referencing outer variables compile but execute with empty captures
- RES-09: Speaker validation infrastructure in place but full @Speaker resolution deferred
- method_idx fallback to 0 for unresolved DefIds in emit_tail_call and SpawnDetached
- Human verification pending: VM load/execute .writil end-to-end, CLI release-build on Windows

### v2.0 Gaps Resolved

- Lifecycle hook dispatch (on_create/on_destroy/on_interact/on_finalize) — FIX-01 ✓
- Generic contract specialization collisions — FIX-02 ✓ (36 unique dispatch entries)
- CliHost string dereferencing — FIX-03 ✓

**Archives:** `milestones/v3.0-ROADMAP.md`, `milestones/v3.0-REQUIREMENTS.md`, `milestones/v3.0-MILESTONE-AUDIT.md`

---

## Completed

- **v6.1** — LSP/DAP Fixes (shipped 2026-03-18)
- **v6.0** — Code Cleanup (shipped 2026-03-18)
- **v5.0** — LSP and DAP (shipped 2026-03-17)
- **v4.0** — Structs as Value Types (shipped 2026-03-13)
- **v3.2** — Spec Corrections and Tooling (shipped 2026-03-07)
- **v3.1** — Compiler Bug Fixes (shipped 2026-03-04)
- **v3.0** — Writ Compiler (shipped 2026-03-03)
- **v2.0** — Writ Runtime (shipped 2026-03-02)
- **v1.2** — Gap Closure (shipped 2026-03-01)
- **v1.1** — Spec v0.4 Conformance (shipped 2026-03-01)
- **v1.0** — CST-to-AST Lowering Pipeline (shipped 2026-02-27)

---

## v2.0 Writ Runtime (Shipped: 2026-03-02)

**Delivered:** Spec-compliant IL runtime — register-based VM, cooperative task scheduler, entity system with GC, contract dispatch, text assembler/disassembler, and `writ` CLI — as a reference implementation to validate future AST-to-IL codegen.

**Phases:** 6 (16-21) | **Plans:** 16
**LOC:** 13,937 Rust (src), 20,284 with tests | **Commits:** 60
**Timeline:** 2 days (2026-03-01 → 2026-03-02)
**Git range:** `cab7809..23171e7`

**Key accomplishments:**

1. `writ-module` crate — spec-compliant binary module reader/writer (200-byte header, 21 metadata tables), 98-opcode Instruction enum with round-trip identity, ModuleBuilder API
2. Register-based VM — dispatch loop for all 91 instructions, cooperative task scheduler (5-state lifecycle), defer/crash engine with LIFO unwinding, atomic sections, RuntimeHost trait with NullHost
3. Entity system + GC — generation-indexed entity registry (SPAWN/INIT/DESTROY/IS_ALIVE/GET_OR_CREATE), MarkSweepHeap with precise mark-and-sweep, GcHeap trait abstraction, root collection from all task registers
4. Contract dispatch — writ-runtime virtual module (9 types, 17 contracts, 36+ ImplDefs, primitive intrinsics), Domain cross-module resolution, CALL_VIRT HashMap dispatch table
5. Text assembler — `writ-assembler` crate with lexer (22 token types), recursive-descent parser, two-pass assembler with forward label resolution
6. Developer tooling — binary-to-text disassembler with round-trip fidelity, `writ` CLI binary with run/assemble/disasm subcommands, CliHost with annotated output

### Known Gaps

- ENT-03/04/05: Lifecycle hook dispatch (on_create, on_destroy, on_interact) — infrastructure ready (two-phase destroy protocol, scheduler helper) but method name lookup deferred
- ENT-08: Component field GET/SET host confirmation — entity instruction handlers exist but component-level suspend not wired
- GC-03: on_finalize hook task scheduling — finalization queue populated during sweep but task creation stubbed (needs method lookup)
- Generic contract specialization collisions in dispatch table (e.g., Int:Into<Float> vs Int:Into<String> share same key)
- CliHost cannot dereference GC heap Ref values (string args to say() print as `<string>` placeholder)
- Register type blob offsets stored as 0 placeholders (assembler limitation — ModuleBuilder doesn't expose blob interning)

**Tech debt carried forward:**

- Lifecycle hook dispatch requires method resolution by name (on_create/on_destroy/on_interact/on_finalize)
- Full generic contract dispatch (type-specialized ImplDef lookup)
- Assembler directives for .export, .extern_fn, .component, .locale, .attribute (disassembler emits as comments)
- Entry method arg passing (Array<String> of CLI args)

**Archives:** `milestones/v2.0-ROADMAP.md`, `milestones/v2.0-REQUIREMENTS.md`

---

## v1.2 Gap Closure (Shipped: 2026-03-01)

**Delivered:** Closed all gaps from v1.1 audit — hex/binary literal lowering corrected, missing verification docs added, dead code removed, stale comments fixed.

**Phases:** 2 (14-15) | **Plans:** 3
**LOC:** 8,826 Rust (src) | **Commits:** 13
**Timeline:** ~30 min (2026-03-01)
**Git range:** `0c81099..7facc14`

**Key accomplishments:**

1. Fixed hex/binary literal lowering with radix-aware `parse_int_literal` helper — `0xFF` → 255, `0b1010` → 10 (closes PARSE-02 gap from v1.1)
2. Created VERIFICATION.md for Phases 10-13, removed dead AstExpr::StructLit variant, fixed stale doc comments and test references

**Tech debt resolved from v1.1:**

- PARSE-02 lowering bug (Phase 14)
- Missing VERIFICATION.md for 4 phases (Phase 15)
- AstExpr::StructLit dead variant (Phase 15)
- Stale comments referencing IndexMut and pre-Phase-13 model (Phase 15)

**Archives:** `milestones/v1.2-ROADMAP.md`

---

## v1.1 Spec v0.4 Conformance (Shipped: 2026-03-01)

**Delivered:** Parser and lowering updated to fully conform to the v0.4 language specification — all parsing gaps, CST/AST representation issues, and lowering correctness bugs fixed.

**Phases:** 6 (8-13) | **Plans:** 12 | **Tests:** ~96 new tests
**LOC:** 8,826 Rust (src) | **Commits:** 27
**Timeline:** 1 day (2026-03-01)
**Git range:** `5295e8f..fa0bbad`

**Key accomplishments:**

1. Lexer validation and escape handling — raw string delimiter enforcement, unicode escape handling in formattable strings, invalid escape rejection with 13 tests
2. CST type system — TypeExpr::Qualified multi-segment paths, Expr::Path rooted flag, DlgDecl attrs/vis fields
3. Parser core syntax — `new` keyword construction, hex/binary literals, struct lifecycle hooks, self/mut-self params, bit-shift and bitwise operators with 33 tests
4. Parser declarations and expressions — impl generics, bodyless operator sigs, component error, extern dotted names/visibility, contextual caret, spawn-detached, defer-block-only, attribute separator with 27 tests
5. Dialogue lowering — namespace-prefixed localization keys, slot identity preservation, choice label emission, say vs say_localized dispatch, speaker scope isolation with 11 tests
6. Entity model — AstDecl::Entity variant, component slots (not fields), all 6 lifecycle hooks with implicit mut self, IndexSet contract name with 12 tests

### Known Gaps

- PARSE-02: Hex/binary literal parsing worked at CST level but lowering silently mapped `0xFF` → 0 (fixed in v1.2 Phase 14)

**Tech debt carried forward:**

- AstExpr::StructLit dead variant (removed in v1.2 Phase 15)
- Stale test comments referencing pre-Phase-13 model (fixed in v1.2 Phase 15)
- Missing VERIFICATION.md for Phases 10-13 (added in v1.2 Phase 15)

**Archives:** `milestones/v1.1-ROADMAP.md`, `milestones/v1.1-REQUIREMENTS.md`, `milestones/v1.1-MILESTONE-AUDIT.md`

---

## v1.0 CST-to-AST Lowering Pipeline (Shipped: 2026-02-27)

**Delivered:** Complete multi-pass CST-to-AST lowering pipeline for the Writ compiler, desugaring all higher-level constructs into spec-defined primitives.

**Phases:** 7 | **Plans:** 13 | **Tests:** 69 snapshot tests
**LOC:** 3,493 Rust (src) | **Commits:** 67
**Timeline:** 2 days (2026-02-26 → 2026-02-27)
**Git range:** `99d6ad6..26837c2`

**Key accomplishments:**

1. AST type hierarchy and pipeline infrastructure — AstExpr/AstStmt/AstDecl/AstType enums with span preservation, LoweringContext with error accumulation, public `lower()` API
2. Expression-level desugaring — Optional sugar (`T?` → `Option<T>`), formattable strings (interpolation → concatenation), compound assignments (`+=`/`-=`/etc.)
3. Operator and concurrency lowering — Operator overloads desugar to contract impls with auto-generated derived operators; concurrency primitives pass through as first-class AST nodes
4. Dialogue lowering with localization — `dlg` → `fn` transformation with three-tier speaker resolution, FNV-1a localization keys, `$ choice` branch scoping, `->` transition validation
5. Entity lowering — `entity` → struct + `ComponentAccess<T>` impls + lifecycle hook registrations with `[Singleton]` propagation and component field flattening
6. Pipeline integration and quality gate — End-to-end pipeline with 69 insta snapshot tests, localization key determinism, speaker stack isolation fix, dead code cleanup

**Tech debt carried forward:**

- R3: `?` propagation and `!` unwrap deferred (spec §18 features)
- R4: Escaped braces (`{{`/`}}`) — lexer gap in writ-parser, not lowering
- R9: Namespace not threaded to FNV-1a key generation
- R4/R8: `lower_dlg_text` duplicates `lower_fmt_string` fold logic

**Archives:** `milestones/v1.0-ROADMAP.md`, `milestones/v1.0-REQUIREMENTS.md`, `milestones/v1.0-MILESTONE-AUDIT.md`

---
