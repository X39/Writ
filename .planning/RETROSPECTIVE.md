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

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 67 | 7 | Established audit → gap closure cycle; snapshot-first testing |
| v1.1 | 27 | 6 | Layer-ordered phases; batched features per phase; audit caught cross-phase bug |
| v1.2 | 13 | 2 | Small focused gap closure; audit-driven fixes |
| v2.0 | 60 | 6 | 4-crate workspace; TDD for tooling; pure-data crate sharing; no audit (risk accepted) |
| v3.0 | 41 | 8 | Linear pipeline IR; `ena`-based type inference; audit-driven 3-phase gap closure; 66 requirements verified |

### Cumulative Quality

| Milestone | Tests | LOC (src) | Tech Debt Items |
|-----------|-------|-----------|-----------------|
| v1.0 | 69 | 3,493 | 7 (all minor) |
| v1.1 | ~165 | 8,826 | 10 (PARSE-02 critical, rest minor → all fixed in v1.2) |
| v1.2 | ~165 | 8,826 | 0 (all v1.1 debt resolved) |
| v2.0 | ~300+ | 13,937 | 6 (hook dispatch, generic dispatch, assembler directives, CLI arg passing, Ref deref, blob offsets) |
| v3.0 | 1,100+ | 57,146 | 6 (TYPE-12 closure captures, RES-09 speaker validation, method_idx fallbacks, dead code, assembler directives, blob offsets) |

### Top Lessons (Verified Across Milestones)

1. **Audit before milestone completion catches integration bugs** that per-phase verification misses — verified v1.0 (speaker stack leak), v1.1 (PARSE-02 lowering bug), v3.0 (4 codegen integration bugs); v2.0 skipped without incident but risk is real
2. **Snapshot tests compound into comprehensive coverage** — 69 in v1.0, ~165 in v1.1, ~300+ in v2.0, 1,100+ in v3.0; minimal late-stage effort each time
3. **Snapshot tests can lock in wrong behavior** — review accepted snapshots carefully; wrong values accepted as correct in both v1.0 (minor) and v1.1 (critical PARSE-02)
4. **Keep traceability tables updated during execution** — stale REQUIREMENTS.md and SUMMARY frontmatter caused confusion in v1.1, v2.0, and v3.0; remains an unsolved process gap across all 5 milestones
5. **Pure-data crate separation pays dividends** — v2.0's writ-module shared cleanly between assembler, runtime, and v3.0 compiler; no coupling across 3 consumers
6. **Run audits early, not after all code phases** — v3.0's audit after Phase 26 spawned 3 gap-closure phases (27-29); running after Phase 25 would have consolidated into 1-2 phases
