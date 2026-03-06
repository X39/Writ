# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — CST-to-AST Lowering Pipeline

**Shipped:** 2026-02-27
**Phases:** 7 | **Plans:** 13 | **Commits:** 67

### What Was Built
- Complete AST type hierarchy (AstExpr/AstStmt/AstDecl/AstType) with span preservation and error recovery
- Multi-pass lowering pipeline: optional sugar, formattable strings, compound assignments, operators, dialogue, entities
- Dialogue lowering with three-tier speaker resolution, FNV-1a localization keys, choice branch scoping
- Entity lowering with component field flattening, lifecycle hooks, [Singleton] propagation
- 69 insta snapshot tests locking down every lowering pass plus integration/determinism checks
- Clean pipeline infrastructure: LoweringContext with error accumulation, pass ordering, public API

### What Worked
- **Dependency-ordered phase sequencing** — building AST types first, then expression helpers, then structural passes meant zero rework and shared helpers flowed naturally
- **Snapshot tests from Phase 2 onward** — every new pass was tested immediately; Phase 6 integration tests required minimal new work because per-pass coverage was already comprehensive
- **Audit → gap closure cycle** — the milestone audit identified three real integration issues (speaker stack leak, dead code, stale docs) that Phase 7 closed surgically; without the audit these would have shipped as latent bugs
- **Manual fold pattern** — simple and direct; no visitor framework overhead for 7 passes; each lowering file is self-contained

### What Was Inefficient
- **SUMMARY.md frontmatter one-liners never populated** — all 13 summaries have empty `requirements_completed` and null `one_liner` fields; the milestone completion tooling couldn't auto-extract accomplishments
- **Namespace threading deferred** — FNV-1a key generation always uses empty namespace; this will need fixing before multi-file compilation
- **`lower_dlg_text` code duplication** — dialogue text lowering reimplements the same concatenation fold as `lower_fmt_string`; should have called the shared helper

### Patterns Established
- **Owned AST types** — no `'src` lifetime from CST; `String`, `Box<T>`, `Vec<T>` throughout
- **Private fields with method API on LoweringContext** — all access via methods; internal representation can change freely
- **`assert_debug_snapshot` over RON** — avoids needing chumsky serde feature for SimpleSpan
- **INSTA_UPDATE=always for test acceptance** — one-step snapshot workflow
- **Intermediate structs over tuples** — EntityProperty/EntityUseClause/EntityHook provide clear field access
- **Save/restore pattern for scoped state** — speaker stack depth saved before block entry, drained after

### Key Lessons
1. **Audit before completing** — the milestone audit caught real integration bugs that per-phase verification missed (speaker stack leak across sequential dlg items)
2. **Test infrastructure pays for itself immediately** — `lower_src` and `lower_src_with_errors` helpers made adding new tests trivial; 69 tests with minimal boilerplate
3. **Parser defects surface during lowering tests** — entity_property trailing-comma issue (Phase 5 gap closure) was a parser grammar bug caught by lowering snapshot tests
4. **Rust 2024 edition strictness is helpful but requires attention** — explicit direct deps, no inline `use` in match arms, for-loop over flat_map with `&mut` captures

### Cost Observations
- Model mix: predominantly opus (execution), sonnet (research/verification/integration), haiku (plan-check)
- Sessions: ~8 (research + planning + execution across 7 phases + audit + gap closure)
- Notable: 2-day wall-clock for 7 phases / 13 plans / 3,493 LOC — highly efficient for a complete lowering pipeline

---

## Milestone: v1.1 — Spec v0.4 Conformance

**Shipped:** 2026-03-01
**Phases:** 6 (8-13) | **Plans:** 12 | **Commits:** 27

### What Was Built
- Lexer validation for raw strings (delimiter enforcement, dedentation, escape rejection) and unicode escapes in formattable strings
- CST extensions: TypeExpr::Qualified multi-segment paths, Expr::Path rooted flag, DlgDecl attrs/vis fields, Stmt::DlgDecl dead variant removal
- Parser for all v0.4 syntax: `new` construction, hex/binary literals, struct lifecycle hooks, self/mut-self params, bit-shift/bitwise operators, impl generics, bodyless operator sigs, component errors, extern dotted names, spawn detached, defer block-only, attribute separator
- Dialogue lowering: namespace-prefixed localization keys, slot identity preservation, choice label emission, say/say_localized dispatch, speaker scope isolation
- Entity model rewrite: AstDecl::Entity with component slots, all 6 lifecycle hooks with implicit mut self, IndexSet contract name

### What Worked
- **Layer-ordered phases (lexer → CST → parser → lowering)** — natural dependency chain meant each phase built cleanly on the previous; zero circular dependencies
- **Batching related features into single phases** — Phase 10 combined 6 parser features; Phase 11 combined 9 requirements; fewer context switches, shared test infrastructure
- **Audit-driven gap closure** — v1.1 audit identified PARSE-02 lowering bug that snapshot tests accepted with wrong values; audit was the only thing that caught this
- **Parallel plan execution** — implementation + tests as separate plans within each phase allowed clear separation of concerns

### What Was Inefficient
- **SUMMARY.md one_liner fields still empty** — same issue from v1.0; accomplishment extraction during milestone completion falls back to manual
- **Traceability table not updated during execution** — 29 of 34 requirements remained "Pending" in REQUIREMENTS.md despite being complete; checkboxes also stale
- **Missing VERIFICATION.md for phases 10-13** — bookkeeping gap carried to v1.2 as tech debt
- **PARSE-02 snapshot accepted incorrect value** — `0xFF` → `0` was accepted as a snapshot; tests locked in wrong behavior

### Patterns Established
- **`parse_int_literal` with radix dispatch** — prefix detection (0x/0b) + underscore stripping + `from_str_radix` (identified as needed by audit, fixed in v1.2)
- **Bracket-inner parser** — atom-level expressions for range operands prevent expr from consuming `..`
- **AstEntityDecl with typed sub-collections** — properties, component_slots, hooks, inherent_impl instead of flat field list
- **LoweringContext namespace API** — push/pop/set for hierarchical namespace threading

### Key Lessons
1. **Snapshot tests can lock in wrong values** — reviewing accepted snapshots matters; `value: 0` for hex was accepted without question
2. **Keep traceability tables updated during execution** — stale status tables create confusion at milestone completion
3. **Audit remains essential** — the v1.1 audit was the only process that caught the PARSE-02 cross-phase integration bug
4. **Batching features by compiler layer is efficient** — grouping by lexer/CST/parser/lowering minimizes context switches compared to feature-by-feature

### Cost Observations
- Model mix: predominantly opus (execution), sonnet (research/verification), haiku (plan-check)
- Sessions: ~6 (across 6 phases + audit)
- Notable: All 6 phases completed in a single day; ~3 hours execution time for 12 plans and ~96 new tests

---

## Milestone: v1.2 — Gap Closure

**Shipped:** 2026-03-01
**Phases:** 2 (14-15) | **Plans:** 3 | **Commits:** 13

### What Was Built
- Radix-aware `parse_int_literal` helper fixing hex/binary lowering (0xFF → 255, 0b1010 → 10)
- VERIFICATION.md for Phases 10-13; dead AstExpr::StructLit removed; stale comments fixed

### What Worked
- **Audit-driven gap closure** — v1.1 audit identified the exact bug and fix location; Phase 14 was surgical
- **Small focused milestone** — 2 phases, 3 plans, ~30 min; no overhead from scope creep

### What Was Inefficient
- Could have been caught during v1.1 if snapshot values were reviewed more carefully

### Key Lessons
1. **Gap closure milestones should be small and immediate** — fixing audit findings while context is fresh is highly efficient

### Cost Observations
- Sessions: 1 (combined with v1.1 completion)
- Notable: Entire milestone completed in ~30 minutes

---

## Milestone: v2.0 — Writ Runtime

**Shipped:** 2026-03-02
**Phases:** 6 (16-21) | **Plans:** 16 | **Commits:** 60

### What Was Built
- `writ-module` crate: spec-compliant binary module reader/writer (200-byte header, 21 metadata tables, 98-opcode instruction enum, ModuleBuilder API, round-trip identity)
- `writ-runtime` crate: register-based VM dispatch loop (91 instructions), cooperative task scheduler (5-state lifecycle), defer/crash engine, atomic sections, RuntimeHost trait
- Entity system with generation-indexed handles, SPAWN/INIT/DESTROY/IS_ALIVE/GET_OR_CREATE, MarkSweepHeap with precise mark-and-sweep GC, GcHeap trait abstraction
- writ-runtime virtual module: 9 types (Int/Float/Bool/String/Option/Result/Range/Array/Entity), 17 contracts, 36+ ImplDefs, cross-module Domain resolution, CALL_VIRT dispatch table
- `writ-assembler` crate: lexer (22 token types), recursive-descent parser, two-pass assembler with forward label resolution, binary-to-text disassembler with round-trip fidelity
- `writ-cli` crate: `writ` binary with run/assemble/disasm subcommands, CliHost with annotated output

### What Worked
- **Pure-data crate separation** (`writ-module`) — shared between assembler, runtime, and future compiler backend; clean dependency graph with no circular imports
- **TDD approach in later phases** — Phase 21 both plans passed first compile attempt using TDD (RED tests, GREEN implementation); 4-6 minute per plan
- **Phase dependency ordering** — Phases 16-19 built naturally on each other; Phase 20 (assembler) depended only on Phase 16 (module format) so could be planned/built independently
- **GcHeap trait boundary** — BumpHeap retained for tests (fast, no-op GC), MarkSweepHeap for production; swap required zero dispatch loop changes
- **Integration tests using ModuleBuilder** — programmatic module construction bypassed assembler limitations (no .export directive), enabling end-to-end runtime tests
- **Deviation tracking** — each phase carefully documented what was deferred and why (hook dispatch, generic specialization, register type blobs)

### What Was Inefficient
- **REQUIREMENTS.md checkboxes not updated during Phase 18** — 13 ENT/GC requirements stayed unchecked despite the implementation being complete; creates confusion at milestone completion
- **No milestone audit** — skipped audit for v2.0; while no critical bugs were found, the audit process has caught real issues in v1.0 and v1.1
- **SUMMARY.md one_liner fields still empty** — same issue across all 4 milestones; accomplishment extraction remains manual
- **Hook dispatch deferred across phases** — on_create/on_destroy/on_interact hook infrastructure was built in Phase 18 but method name lookup was deferred to Phase 19, which then also deferred it; the feature is ready but never wired

### Patterns Established
- **4-crate workspace architecture** — writ-module (data), writ-runtime (VM), writ-assembler (text format), writ-cli (binary) — each crate has single responsibility
- **MetadataToken newtype** — enforces 1-based indexing at the type level; 0 = null token
- **Generation-indexed handles** for entity identity — stale-handle detection without UB
- **Virtual module at startup** — no file on disk; programmatic construction guarantees availability
- **Contract dispatch table at load time** — O(1) CALL_VIRT; HashMap from (type, contract, method) to method index
- **peek_kind()/advance() parser pattern** — works cleanly with Rust 2024 edition's stricter borrow rules

### Key Lessons
1. **Keep traceability tables updated during execution** — stale checkboxes in REQUIREMENTS.md created 13 false-unchecked requirements at milestone completion; third milestone with this issue
2. **Pure-data crates pay dividends immediately** — `writ-module` shared by assembler and runtime without any coupling; future compiler backend gets it for free
3. **Defer judiciously but track deferrals explicitly** — hook dispatch was deferred from Phase 18 to 19 to "future phase"; needs a clear home in the next milestone
4. **TDD works especially well for tooling** — disassembler and CLI both passed first compile; tests drove the implementation cleanly

### Cost Observations
- Model mix: predominantly opus (execution), sonnet (research), haiku (plan-check)
- Sessions: ~6 (across 6 phases)
- Notable: Entire 6-phase milestone completed in ~2 days; 13,937 LOC (src) across 4 new crates; average ~7 min per plan

---

## Milestone: v3.0 — Writ Compiler

**Shipped:** 2026-03-03
**Phases:** 8 (22-29) | **Plans:** 24 | **Commits:** 41

### What Was Built
- Name resolution with two-pass symbol collection, qualified paths, visibility enforcement, generic scoping, fuzzy "did you mean?" suggestions (12 requirements)
- Type checker with `ena`-based unification for generic inference, strict mutability enforcement, ?/!/try desugaring to typed match nodes, enum exhaustiveness checking (19 requirements)
- IL metadata skeleton populating all 21 spec-defined tables with correct token assignment, CALL_VIRT slot ordering from contract declaration (8 requirements)
- All 90 IL instructions emitted — register allocator, entity construction sequences, closure/delegate emission, concurrency primitives, constant folding, TAIL_CALL, StrBuild, debug info (20 requirements)
- `writ compile` CLI with 5-stage pipeline (parse -> lower -> resolve -> typecheck -> codegen), ariadne error diagnostics, end-to-end validation (6 requirements)
- Gap closure — LocaleDef emission for locale overrides, 4 codegen bug fixes, retroactive verification of all 66 requirements

### What Worked
- **Linear pipeline IR** (AST -> NameResolved -> Typed -> Module) — clean phase boundaries with hard stops on errors; each phase tests independently
- **Two-pass name resolution** — collecting all declarations before resolving any reference eliminated forward-reference failures entirely
- **Audit-driven gap closure** — v3.0 audit after Phase 26 identified 40 orphaned requirements and 4 integration bugs; Phases 27-29 closed all gaps systematically
- **CALL_VIRT slot ordering from declaration order** — fixed before any body emission; prevented O(n^2) impl-order sensitivity bugs
- **Deferred string interning** (`pending_strings` pattern) — cleanly separated immutable BodyEmitter from mutable ModuleBuilder without lifetime gymnastics

### What Was Inefficient
- **SUMMARY.md frontmatter still not populated during execution** — 34/66 requirements missing from SUMMARY frontmatter; audit relies on VERIFICATION.md as ground truth; 5th milestone with this issue
- **Three audit-gap-closure iterations** — Phase 27 (retroactive verification), Phase 28 (codegen bug fixes), Phase 29 (LocaleDef) could have been one consolidated gap phase if the initial audit ran earlier
- **extract_callee_def_id_opt dead code retained** — kept "for debugging" but never used; pattern of keeping dead code "just in case" adds noise
- **Phase 25 needed 6 plans** (2 gap closures) — initial 4-plan scope underestimated SWITCH fixup, closure body emission, and string interning complexity

### Patterns Established
- **writ-diagnostics shared crate** — error codes and DiagnosticBuilder shared between writ-compiler and writ-cli; single source of truth for error formatting
- **TyInterner with structural deduplication** — Ty(u32) interning prevents duplicate type allocations; FxHashMap<TyKind, Ty> for O(1) lookup
- **`ena` union-find for generic inference** — InferValue wrapper cleanly integrates with Rust's `ena` crate for unification-based type variable resolution
- **`$` suffix for locale-override names** — invalid in user identifiers, prevents resolver collisions; base name recovered via `split('$').next()`
- **collect_post_finalize() for token-dependent collection** — LocaleDef, ExportDef, and other token-dependent metadata collected after finalize() assigns stable tokens

### Key Lessons
1. **Run the audit earlier** — running audit after all code phases would have created one gap-closure phase instead of three (27, 28, 29)
2. **Hard phase boundaries prevent cascading errors** — resolve errors block typecheck; typecheck errors block codegen; no "compile anyway" mode means bugs are caught at the right layer
3. **Type checker complexity concentrates in desugaring** — ?/!/try desugaring, generic inference, and exhaustiveness checking were 60% of type checker effort despite being "sugar"
4. **Register allocation is simpler than expected** — LIFO high-watermark allocator sufficed; no need for graph coloring or live range analysis in a stack-oriented IR
5. **Virtual module contracts drive dispatch table correctness** — getting the 17 base + 5 specialized contracts right in writ-runtime was the key to CALL_VIRT working end-to-end

### Cost Observations
- Model mix: predominantly opus (execution), sonnet (research/verification/integration), haiku (plan-check)
- Sessions: ~10 (across 8 phases + audit + gap closure)
- Notable: 8 phases / 24 plans / 37,582 new LOC in 2 days; average ~15 min per plan (up from ~7 min in v2.0 due to increased complexity)

---

## Milestone: v3.1 — Compiler Bug Fixes

**Shipped:** 2026-03-04
**Phases:** 10 (30-39) | **Plans:** 22 | **Commits:** ~80

### What Was Built
- Critical bug fixes for codegen emission ordering and token assignment
- Golden test harness with BOM stripping, CRLF normalization, and bless workflow
- 15+ golden test fixtures covering all major language features end-to-end
- Fix for spurious void register in empty function bodies
- Fix for CALL method token resolution in runtime
- Extended method_defs metadata with param_count and format_version bump to 2

### What Worked
- **Golden test approach** — snapshot testing for entire compilation pipeline catches regressions across all layers simultaneously
- **Inserted decimal phases (31.1, 31.2)** — urgent bug fixes inserted without renumbering; clean precedent for future insertions
- **Bug-fix batching** — Phases 32-36 combined golden test creation with bug discovery; tests drove fixes naturally

### What Was Inefficient
- **Many small phases** — 10 phases for what was essentially iterative bug fixing; could have been fewer larger phases
- **Golden test creation overlapped with bug discovery** — some tests were written, found bugs, which needed new phases to fix

### Key Lessons
1. **Golden tests are the highest-value test investment** — they exercise the entire pipeline and caught more bugs per test than unit tests
2. **Decimal phases work well for urgent insertions** — 31.1 and 31.2 inserted cleanly without disrupting phase numbering

### Cost Observations
- Sessions: ~5 (iterative fix-test-fix cycles)
- Notable: High iteration count but each phase was fast (most plans < 10 minutes)

---

## Milestone: v3.2 — Spec Corrections and Tooling

**Shipped:** 2026-03-07
**Phases:** 7 (40-46) | **Plans:** 15 | **Commits:** 91

### What Was Built
- Spec cleanup: §1.2.8 removed, §26.4 root-namespace inbuilt calls added, writ.toml field alignment
- fn_log_say_choice golden test fix: check_path normalization, BOM handling, UTF-8 snapshot
- ChoiceOption atomic four-layer rename (spec, lowering, virtual module, prelude)
- Unqualified None/Some with sub-prelude injection in resolver and type checker, parser `::*` glob expansion
- Leveled logging namespace (log::trace/debug/info/warn/error) with synthetic ExternFn DefIds
- `writ build` subcommand with --release/--debug profiles, multi-file compilation
- Structs-as-value-types design record (§8.4) with struct/class split decision (YES)

### What Worked
- **Synthetic DefId injection pattern** — reused from None/Some (Phase 43) to log namespace (Phase 44); the sub-prelude pattern proved composable
- **Atomic rename approach** — ChoiceOption rename across 4 layers in a single phase prevented partial-rename inconsistency
- **Design record as phase** — Phase 46 produced a thorough decision record without implementation pressure; clean separation of design from code
- **FileId(u32::MAX) sentinel** — cleanly distinguishes compiler-injected entries from user-declared; no new DefKind needed
- **run_pipeline() extraction** — shared helper between cmd_compile and cmd_build avoided code duplication

### What Was Inefficient
- **SUMMARY.md one_liner fields still empty** — 7th milestone with this issue; milestone completion tooling couldn't auto-extract accomplishments
- **§26.4 not added to TOC** — spec section added in Phase 40 but TOC update missed; caught only by audit
- **test_fn_optional not registered** — golden test fixture created but registration in golden_tests.rs missed; audit caught this

### Patterns Established
- **Sub-prelude injection** — inject symbols at lower priority than user definitions; used for None/Some and log:: levels
- **Synthetic DefId with FileId sentinel** — compiler-known builtins injected without user AST
- **ProfileConfig with serde default functions** — enables partial TOML overrides
- **run_pipeline() shared helper** — 5-stage pipeline extracted for reuse across compile modes

### Key Lessons
1. **Sub-prelude patterns compose** — the None/Some injection pattern was directly reused for log:: levels in the next phase with minimal adaptation
2. **Audit catches registration gaps** — test_fn_optional and §26.4 TOC were both caught only by audit
3. **Design-only phases are valuable** — Phase 46 produced a clear decision record that defines v4.0 scope without premature implementation

### Cost Observations
- Model mix: opus (execution), sonnet (research/verification), haiku (plan-check)
- Sessions: ~4 (across 7 phases + audit)
- Notable: 7 phases / 15 plans in 1 day; smallest code milestone but highest spec/tooling impact

---

## Milestone: v4.0 — Structs as Value Types

**Shipped:** 2026-03-13
**Phases:** 5 (47-51) | **Plans:** 12 | **Commits:** 74

### What Was Built
- Language spec rewrite: §8 Structs as value types, new §9 Classes, memory model/construction model/IL type system/module format amendments across 29 spec files
- IL format extension: TypeDefKind::Class=4, format_version=3, compile-time type safety via TypeDefKind enum at all public API boundaries
- VM runtime restructure: Value::InlineStruct for inline struct registers, Copy derive removed, kind-dependent NEW dispatch, GC tracing through value-struct fields
- Compiler frontend: `class` keyword through full pipeline (lexer, CST, parser, AST, lowering, name resolution, type checking), recursive struct cycle detection
- Structural equality: field-by-field comparison emission for ==/!= on value-type structs, closure capture environments migrated to kind=4 (class)
- Golden tests: all existing tests updated for format_version=3, new tests for struct equality, class declarations, recursive struct error

### What Worked
- **Spec-first approach** — Phase 47 (spec amendments) completed before any code; all subsequent phases had a single source of truth for behavior; zero spec-code disagreements
- **Strict dependency ordering (spec → IL → VM → compiler → tests)** — each phase built on the previous with no circular dependencies; natural bottom-up build
- **TypeDefKind enum at API boundaries** — catching kind errors at compile time (builder API takes TypeDefKind not u8) eliminated an entire class of runtime bugs; zero invalid-kind issues in later phases
- **format_version=3 strict rejection** — reader rejects anything != 3; clean break from v2 modules with no backward compat complexity
- **Separate class AST types** — ClassDecl/AstClassDecl distinct from StructDecl; prevented struct-specific logic from accidentally applying to classes
- **DFS cycle detection with globally_visited + in_path sets** — correctly handles diamond dependencies without false positives; excludes class/entity/enum fields from cycles

### What Was Inefficient
- **No milestone audit** — skipped audit; while all 31 requirements were checked off, an audit would have verified cross-phase integration (e.g., BOX/UNBOX actually exercised end-to-end)
- **SUMMARY.md one_liner fields still empty** — 8th milestone with this issue; remains an unsolved process gap
- **COMP-06 resolved by decision rather than implementation** — entity→class was originally scoped but became unnecessary; requirement should have been questioned during planning
- **Phase 47 Plan 04 (renumbering) took 45min** — mechanical file renaming and heading fixes across 33 files; could potentially be automated

### Patterns Established
- **Value::InlineStruct with Vec\<Value\> fields** — uniform representation for inline struct registers; PartialEq derived from type_idx + fields
- **Explicit .clone() for register reads** — removing Copy from Value forces every register read to explicitly clone; prevents accidental shallow copies of multi-word InlineStruct values
- **Kind-dependent dispatch in VM** — NEW, GET_FIELD, SET_FIELD, GC tracing all switch on TypeDefKind; extensible to future type kinds
- **Structural equality emission** — compiler emits field-by-field GetField + CmpEq chains; no STRUCT_EQ opcode needed; nested structs handled recursively
- **parse-and-report pattern for lifecycle hook rejection** — `class` supports on create/finalize; `struct` parses the hook then emits a diagnostic, giving better error messages than a parse failure

### Key Lessons
1. **Spec-first pays off** — having normative spec prose before code meant zero ambiguity in VM and compiler phases; the spec was the specification, not a retroactive document
2. **TypeDefKind at API boundaries > u8** — compile-time type safety eliminated runtime invalid-kind errors; pattern should apply to all metadata enum types
3. **Copy removal forces correct cloning** — removing Copy from Value was a large mechanical change (72 .clone() insertions) but eliminated an entire class of subtle bugs where InlineStruct fields were shared instead of copied
4. **Decision-resolved requirements should be flagged earlier** — COMP-06 (entity→class) was unnecessary per user decision; catching this during planning would have saved scope

### Cost Observations
- Model mix: opus (execution), sonnet (research/verification), haiku (plan-check)
- Sessions: ~5 (across 5 phases)
- Notable: 5 phases / 12 plans / +4,282 net LOC in 1 day; average ~15 min per plan; smallest v-major milestone by phase count but touched 238 files

---

## Milestone: v5.0 — LSP and DAP

**Shipped:** 2026-03-17
**Phases:** 10 (52-61) | **Plans:** 20 | **Commits:** 144

### What Was Built
- Language server (writ-lsp, 5,144 LOC) — diagnostics, completions, hover, go-to-definition, find references, signature help, semantic tokens, cross-file via writ.toml
- Debug adapter (writ-dap, 2,486 LOC) — F5 launch, source-level breakpoints, step over/into/out, local variable inspection, call stack, watch expressions, cooperative tasks as debugger threads
- VS Code extension (writ-vscode) — TextMate grammar, bundled binary launch, default launch.json, VSIX packaging pipeline
- Semantic highlighting with distinct token types for entities, components, dialogue speakers, types, functions
- Debug info pipeline: SourceSpan line/column, debug locals, parser error recovery, RuntimeHost debug hooks
- UAT-driven gap closure (phases 58-61): speaker tokens, VSIX build, query robustness, signature help

### What Worked
- **UAT-driven phases (58-61)** — user acceptance testing identified real usability gaps that phase-level verification missed; phases 58-61 were all UAT-originated
- **Compiler preparation phase (52)** — adding debug info, source spans, and parser error recovery before LSP/DAP work prevented blocking dependencies later
- **tower-lsp framework** — async handler pattern worked well; language server features could be added incrementally

### What Was Inefficient
- **4-day timeline** — longest milestone; LSP and DAP required learning tower-lsp and DAP protocol details
- **Single-file DAP limitation** — compile_and_load only supports single-file, not writ.toml projects; limits real debugging scenarios

### Patterns Established
- **AnalysisHost as compilation cache** — single entry point for all LSP queries; re-compiles on change
- **writ-vscode packaging** — esbuild + vsce, bundled binaries, VSIX artifact

### Key Lessons
1. **UAT phases are high-value** — phases 58-61 each fixed real usability issues that automated testing missed
2. **Compiler prep phase is essential** — adding debug info to the compiler before starting LSP/DAP saved integration headaches
3. **Protocol learning has a fixed cost** — DAP and LSP both required upfront protocol study; batching related protocol work is efficient

### Cost Observations
- Sessions: ~8 (across 10 phases + UAT iterations)
- Notable: Largest milestone by LOC addition (+38,231 net lines); 4 days wall-clock for 20 plans

---

## Milestone: v6.0 — Code Cleanup

**Shipped:** 2026-03-18
**Phases:** 5 (62-66) | **Plans:** 14 | **Commits:** 18

### What Was Built
- Zero clippy warnings across 9 Rust crates (194 eliminated: 155 auto-fixed, 39 manual)
- 12 oversized files split into focused submodules (check_expr/, collect/, expr/, parser/, queries/, server/, commands/, etc.)
- Duplicate code consolidated (lower_dlg_text delegates to lower_fmt_string)
- Module boundaries tightened (explicit imports, pub(crate) narrowing, module doc headers)
- Cross-phase regression fixes (6 dead code items removed, say() 2-arg ABI restored, golden tests blessed)

### What Worked
- **cargo clippy --fix for bulk auto-apply** — 155 of 194 warnings fixed in one pass; massive time savings
- **Folder module pattern** — consistent `file.rs` → `file/mod.rs` + subfiles pattern; parent re-exports maintain public API
- **Delegation over deduplication** — lower_dlg_text → lower_fmt_string delegation was cleaner than extracting a shared helper
- **Milestone audit caught cross-phase regression** — say() 1-arg regression from Phase 62 clippy auto-fix was invisible until audit checked E2E flows

### What Was Inefficient
- **Clippy --fix introduced semantic regression** — auto-renaming `speaker_ref` to `_speaker_ref` silently removed the argument from say() calls; required Phase 66 to fix
- **Phase 66 needed as gap closure** — audit found issues that should have been caught by Phase 62 verification
- **Golden tests not included in Phase 66 scope** — 2 golden test regressions (fn_log_say_choice, quest_system) discovered during milestone completion, not during phase execution

### Patterns Established
- **Tier 1/2/3 wildcard classification** — Tier 1 (same-crate imports): always explicit; Tier 2 (cross-crate internal): always explicit; Tier 3 (domain vocabulary like AST types): intentional wildcard with comment
- **pub(crate) narrowing reveals dead code** — narrowing module visibility from pub to pub(crate) allows rustc dead_code analysis to detect genuinely unused items
- **Documented 500-line exceptions** — files exceeding the target get comments explaining why (Chumsky recursive(), tightly-coupled state, etc.)

### Key Lessons
1. **cargo clippy --fix can introduce semantic regressions** — auto-fixes that rename variables (e.g., `x` → `_x`) can remove them from use sites; always review auto-fix diffs for behavioral changes
2. **Visibility narrowing is a discovery tool** — pub(crate) narrowing found 6 genuinely dead items that were invisible as pub; the refactoring paid for itself in dead code discovery
3. **Audit catches cross-phase regressions that per-phase verification misses** — Phase 62 verification said "zero warnings" but Phase 65 pub narrowing broke that invariant; only the milestone audit detected the gap
4. **Golden tests need re-verification after pipeline changes** — any change to compiler output (like say() arg count) can silently invalidate golden test expectations

### Cost Observations
- Model mix: opus (execution), sonnet (research/verification/integration), haiku (plan-check)
- Sessions: ~4 (across 5 phases + audit + gap closure)
- Notable: 5 phases / 14 plans in 1 day; +1,752 net LOC; pure refactoring milestone — all behavioral changes were bug fixes

---

## Milestone: v6.1 — LSP/DAP Fixes

**Shipped:** 2026-03-18
**Phases:** 3 (67-69) | **Plans:** 5 | **Commits:** 8 feat

### What Was Built
- LSP namespace completions for `log::`, `Option::`, `Result::`, and user-defined enums via `::` trigger
- LSP dot-completion pipeline verified end-to-end for struct and array receivers
- SWITCH and DeferPush byte-offset encoding fixed in IL serializer (quest_system.writ runnable through DAP)
- Multi-file writ.toml project launch through DAP debug adapter
- Golden test coverage for dialogue/function mix patterns (dlg_fn_mix, dlg_quest_pattern)

### What Worked
- **Targeted milestone scope** — 6 requirements, 3 phases, 5 plans; no scope creep; all requirements satisfied on first pass
- **Audit-first approach** — milestone audit ran before completion; all gaps flagged early; no gap-closure phases needed
- **Verify-before-fix approach (Phase 67 Plan 02)** — dot-completion diagnosis confirmed pipeline worked correctly, avoiding unnecessary code changes; diagnosis was the fix
- **Pass 4 byte-offset pattern** — consolidating all instruction-index-to-byte-offset conversions in a single serialize.rs pass kept concerns separated from the emitter

### What Was Inefficient
- **SUMMARY.md one_liner fields still empty** — 11th milestone with this issue; summary-extract tool returned null for all 5 plans; manual accomplishment extraction at milestone completion
- **summary-extract key name mismatch** — tool looks for `requirements_completed` but YAML frontmatter uses `requirements-completed`; longstanding tool bug
- **Nyquist validation partial** — all 3 phases have VALIDATION.md but none marked compliant; Wave 0 tests not formally stubbed before execution

### Patterns Established
- **compile_and_load_project** — DAP now supports both single-file and project-mode launch with mode detection (`is_dir` || `ends_with(writ.toml)`)
- **source_paths Vec<(FileId, String)>** — multi-file source tracking in DAP replaces single-source Option
- **Explicit extern say() override in dialogue golden tests** — workaround for auto-injected 1-param vs lowered 2-param say() mismatch

### Key Lessons
1. **Verify before fixing** — Phase 67 Plan 02 confirmed the dot-completion pipeline was already correct; saved implementing unnecessary changes
2. **Serialization concerns belong in the serializer** — SWITCH/DeferPush byte-offset conversion in Pass 4 of serialize.rs was cleaner than fixing in the emitter (patterns.rs)
3. **Known bugs constrain test patterns** — `::choice` serialization bug forced `$ if` workaround in dlg_quest_pattern golden test; test design must account for existing limitations

### Cost Observations
- Model mix: opus (execution), sonnet (research/verification/integration), haiku (plan-check)
- Sessions: ~3 (across 3 phases + audit + completion)
- Notable: Entire 3-phase milestone completed in <1 day; fastest complete milestone cycle (from roadmap creation through audit to completion)

---

## Milestone: v7.0 — Benchmark Suite

**Shipped:** 2026-03-20
**Phases:** 5 (70-74) | **Plans:** 12 | **Commits:** 66

### What Was Built
- Multi-stage Docker container with 6 language runtimes (Writ, Lua 5.4, Squirrel 3.x, Python 3.x, Node.js 22 LTS, Rust) and hyperfine-based measurement harness
- 7 cross-language benchmarks: fibonacci, sieve, string_concat, array_sort, hash_map, oop_dispatch, object_create — all with output-checksum parity verification
- SVG chart generation (pygal) and markdown results tables with execution time, memory usage, and startup time comparison
- One-command host runners (run.sh + run.ps1) with Docker/Podman auto-detection and cross-platform path normalization
- Compiler features added to support benchmarks: Array `.push()`/`.len()` methods, IMPL-METHOD fix for contract/impl dispatch
- GitHub Actions CI workflow with manual dispatch, weekly schedule, and artifact upload

### What Worked
- **Algorithm-spec-first approach** — locking canonical algorithm parameters and expected output checksums before writing any language implementations prevented correctness drift across 6 languages
- **Benchmark-driven compiler bug discovery** — array_sort and oop_dispatch benchmarks uncovered real compiler bugs (TyKind::Array member access, IMPL-METHOD dispatch) that were immediately fixed as part of benchmark development
- **Host-side chart generation** — keeping pygal chart generation outside the Docker container simplified the Docker image and allowed rapid iteration on chart formatting without container rebuilds
- **Single-day completion** — all 5 phases and 12 plans completed in one day; tight phase dependencies (70→71→72→73→74) executed cleanly in sequence
- **Null guard pattern for missing Writ benchmarks** — hash_map has no Writ implementation (no Map type); generate.py's null guard pattern handled this gracefully across charts and tables

### What Was Inefficient
- **SUMMARY.md one_liner fields still empty** — 12th milestone with this issue; summary-extract tool returned null for all plans; manual accomplishment extraction at milestone completion
- **stub/ suite pollution** — bench_runner.sh's glob pattern includes the stub directory, producing cosmetic noise in raw.json, RESULTS.md, and SVG charts; should have been excluded once real benchmarks were added
- **smoke_contract.writ orphan** — development artifact left in benchmark/cases/oop_dispatch/; should have been cleaned up during Phase 73 execution
- **Nyquist compliance partial** — only Phase 70 formally validated; Phases 71-74 have VALIDATION.md but marked non-compliant
- **Squirrel build complexity** — cmake + shared lib copy + ldconfig required multiple Docker build iterations to get right

### Patterns Established
- **Cross-language benchmark template** — each benchmark directory contains 6 source files (`.writ`, `.lua`, `.nut`, `.py`, `.js`, `.rs`) with identical algorithm and verified matching output
- **bench_runner.sh auto-discovery** — `/bench/cases/*/` glob pattern automatically picks up new benchmark suites without runner changes
- **measure_anon_rss() function** — background subprocess + /proc/<pid>/status RssAnon polling; reusable for any Linux memory measurement
- **Writ compile/run separation** — `compile_ms` and `run_ms` as separate JSON fields; distinguishes interpreter startup from compilation overhead
- **generate.py null guard** — `writ_compile_ms`/`writ_run_ms` return None for missing writ entries; chart/table callers use if-guards to skip

### Key Lessons
1. **Benchmarks are excellent compiler integration tests** — the array_sort benchmark discovered the TyKind::Array member access gap, and oop_dispatch discovered the IMPL-METHOD dispatch bug; benchmarks stress the compiler in ways unit tests don't
2. **Cross-platform Docker volume mounts are fragile** — MINGW path conversion, Podman WSL mount paths, and .gitattributes eol=lf were all required to make run.sh work across Windows/Linux/macOS
3. **Algorithm-spec-first prevents 6x rework** — fixing an algorithm bug in one language after writing all six is expensive; specifying the exact algorithm upfront saved significant rework
4. **No Map type limits benchmark coverage** — Writ's lack of a built-in Map type meant hash_map benchmark required null handling; this is the first real feature gap exposed by benchmarking

### Cost Observations
- Model mix: opus (execution), sonnet (research/planning), haiku (plan-check)
- Sessions: ~5 (across 5 phases + audit)
- Notable: Entire 5-phase milestone completed in ~6 hours; 12 plans producing 152 files / +11,235 net LOC; fastest multi-phase milestone by wall-clock time

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 67 | 7 | Established audit → gap closure cycle; snapshot-first testing |
| v1.1 | 27 | 6 | Layer-ordered phases; batched features per phase; audit caught cross-phase bug |
| v1.2 | 13 | 2 | Small focused gap closure; audit-driven fixes |
| v2.0 | 60 | 6 | 4-crate workspace; TDD for tooling; pure-data crate sharing; no audit (risk accepted) |
| v3.0 | 41 | 8 | Linear pipeline IR; `ena`-based type inference; audit-driven 3-phase gap closure; 66 requirements verified |
| v3.1 | ~80 | 10 | Golden test harness; decimal phase insertion; iterative bug-fix cycles |
| v3.2 | 91 | 7 | Sub-prelude injection pattern; synthetic DefId injection; design-only phases; spec/tooling focus |
| v4.0 | 74 | 5 | Spec-first approach; TypeDefKind enum safety; Value Copy removal; kind-dependent VM dispatch |
| v5.0 | 144 | 10 | UAT-driven gap closure; compiler prep phase; tower-lsp/DAP protocol; VSIX packaging |
| v6.0 | 18 | 5 | cargo clippy --fix bulk auto-apply; folder module pattern; pub(crate) dead code discovery; audit caught clippy regression |
| v6.1 | 8 | 3 | Targeted bug-fix milestone; verify-before-fix approach; audit-first completion; fastest milestone cycle |
| v7.0 | 66 | 5 | Algorithm-spec-first; benchmark-driven compiler bug discovery; single-day 12-plan completion; cross-platform Docker |

### Cumulative Quality

| Milestone | Tests | LOC (src) | Tech Debt Items |
|-----------|-------|-----------|-----------------|
| v1.0 | 69 | 3,493 | 7 (all minor) |
| v1.1 | ~165 | 8,826 | 10 (PARSE-02 critical, rest minor → all fixed in v1.2) |
| v1.2 | ~165 | 8,826 | 0 (all v1.1 debt resolved) |
| v2.0 | ~300+ | 13,937 | 6 (hook dispatch, generic dispatch, assembler directives, CLI arg passing, Ref deref, blob offsets) |
| v3.0 | 1,100+ | 57,146 | 6 (TYPE-12 closure captures, RES-09 speaker validation, method_idx fallbacks, dead code, assembler directives, blob offsets) |
| v3.1 | 1,100+ | ~58,000 | 0 (all v3.0 code debt resolved; method_defs extended) |
| v3.2 | 1,100+ | 59,767 | 8 (all low severity: TOC gap, undocumented using log::*, test registration, CLI manual verification) |
| v4.0 | 1,100+ | 61,853 | 8 (carried from v3.2; no new debt added) |
| v5.0 | 1,100+ | 72,591 | 4 (hover edge cases, single-file DAP, dot-completion, TYPE-12) |
| v6.0 | 1,100+ | 74,227 | 7 (TYPE-12 carried, choice serialization, assembler directives, TOC gap, log::* spec, test registration, orphaned re-export) |
| v6.1 | 1,100+ | 74,997 | 6 (TYPE-12, choice serialization, hardcoded Option/Result completions, DAP source attribution, dlg string interpolation, Nyquist partial) |
| v7.0 | 1,100+ | 79,005 | 6 (TYPE-12, choice serialization, StrLen heap bug, no Map type, stub suite noise, CI human-verification) |

### Top Lessons (Verified Across Milestones)

1. **Audit before milestone completion catches integration bugs** that per-phase verification misses — verified v1.0 (speaker stack leak), v1.1 (PARSE-02 lowering bug), v3.0 (4 codegen integration bugs), v3.2 (test registration gap, TOC gap), v6.0 (clippy say() regression, dead code from pub narrowing); v2.0 skipped without incident but risk is real
2. **Snapshot tests compound into comprehensive coverage** — 69 in v1.0, ~165 in v1.1, ~300+ in v2.0, 1,100+ in v3.0+; minimal late-stage effort each time
3. **Snapshot tests can lock in wrong behavior** — review accepted snapshots carefully; wrong values accepted as correct in both v1.0 (minor) and v1.1 (critical PARSE-02)
4. **Keep traceability tables updated during execution** — stale REQUIREMENTS.md and SUMMARY frontmatter caused confusion in v1.1, v2.0, and v3.0; remains an unsolved process gap across all milestones
5. **Pure-data crate separation pays dividends** — v2.0's writ-module shared cleanly between assembler, runtime, and v3.0 compiler; no coupling across 3 consumers
6. **Run audits early, not after all code phases** — v3.0's audit after Phase 26 spawned 3 gap-closure phases (27-29); running after Phase 25 would have consolidated into 1-2 phases
7. **Composable injection patterns scale** — v3.2's sub-prelude injection for None/Some was directly reused for log:: levels; synthetic DefId injection established as a general-purpose compiler extension pattern
8. **Design-only phases clarify scope boundaries** — v3.2 Phase 46 produced a thorough v4.0 decision record without implementation pressure; prevents scope creep
9. **Spec-first development eliminates ambiguity** — v4.0 wrote normative spec prose before any code; zero spec-code disagreements across 5 phases
10. **TypeDefKind at API boundaries > raw u8** — compile-time safety eliminated an entire class of runtime invalid-kind errors; pattern generalizes to all metadata enum types
11. **cargo clippy --fix requires manual review** — v6.0's auto-fix silently removed say() speaker argument; automated tooling can introduce semantic regressions
12. **Visibility narrowing is a code health tool** — v6.0's pub(crate) narrowing discovered 6 genuinely dead items invisible at pub visibility; the refactoring pays for itself in dead code discovery
13. **Verify-before-fix saves effort** — v6.1 Phase 67 diagnosed the dot-completion pipeline as already correct; "fix what's broken" beats "fix what looks broken"
14. **Benchmarks are compiler integration tests** — v7.0 benchmark development discovered TyKind::Array member access gap and IMPL-METHOD dispatch bug; real programs stress compilers in ways unit tests don't
15. **Algorithm-spec-first prevents N-language rework** — v7.0 locked algorithm parameters and expected output before writing implementations across 6 languages; prevented 6x rework on correctness bugs
