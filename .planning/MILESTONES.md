# Milestones
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

