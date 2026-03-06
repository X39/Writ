---
phase: 50-compiler-frontend
plan: 01
subsystem: compiler
tags: [rust, class, keyword, lexer, parser, cst, ast, lowering, name-resolution, type-checker, codegen]

# Dependency graph
requires:
  - phase: 49-vm-runtime
    provides: Value::InlineStruct added, kind-dispatch NEW instruction
  - phase: 47-spec-amendments
    provides: struct/class split spec, lifecycle hook restrictions
provides:
  - Token::KwClass in writ-parser lexer
  - ClassDecl/ClassMember CST nodes (cst.rs)
  - Item::Class, ExternDecl::Class in parser (parser.rs)
  - struct on-create/on-finalize parse-time rejection with diagnostic
  - AstDecl::Class(AstClassDecl), AstExternDecl::Class in AST (decl.rs)
  - lower_class() lowering pipeline (lower/mod.rs)
  - DefKind::Class, DefKind::ExternClass in name resolution (def_map.rs)
  - ResolvedDecl::Class, ResolvedDecl::ExternClass (resolve/ir.rs)
  - TyKind::Class(DefId) in type checker (ty.rs)
  - TypedDecl::Class, TypedDecl::ExternClass in typed IR (check/ir.rs)
  - Class field access, method dispatch, new construction in type checker
  - collect_class, collect_extern_class emitting TypeDefKind::Class in codegen
affects: [50-02-PLAN, codegen, emit, writ-cli]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ClassDecl reuses StructField for fields, ClassMember mirrors StructMember"
    - "AstClassDecl reuses AstStructMember (identical field/hook shape)"
    - "Class fields stored in struct_fields map (same lookup path as Struct)"
    - "Parse-and-report pattern for hook rejection (emit diagnostic, continue parsing)"

key-files:
  created: []
  modified:
    - writ-parser/src/lexer.rs
    - writ-parser/src/cst.rs
    - writ-parser/src/parser.rs
    - writ-compiler/src/ast/decl.rs
    - writ-compiler/src/lower/mod.rs
    - writ-compiler/src/resolve/def_map.rs
    - writ-compiler/src/resolve/collector.rs
    - writ-compiler/src/resolve/resolver.rs
    - writ-compiler/src/resolve/ir.rs
    - writ-compiler/src/check/ty.rs
    - writ-compiler/src/check/ir.rs
    - writ-compiler/src/check/check_decl.rs
    - writ-compiler/src/check/env.rs
    - writ-compiler/src/check/check_expr.rs
    - writ-compiler/src/check/infer.rs
    - writ-compiler/src/check/unify.rs
    - writ-compiler/src/emit/collect.rs
    - writ-compiler/src/emit/type_sig.rs
    - writ-compiler/src/emit/body/expr.rs

key-decisions:
  - "ClassDecl/AstClassDecl are separate types from StructDecl/AstStructDecl for clarity, but members reuse StructField/AstStructMember since class members have identical shape"
  - "Class fields stored in type_env.struct_fields map (DefKind::Class/ExternClass both use struct_fields) — intentional sharing since field lookup logic is identical"
  - "parse-and-report pattern for on create/finalize rejection: diagnostic emitted but parse continues to avoid cascade errors"
  - "08_dialogue.writ player.class renamed to player.job since class is now a reserved keyword"

patterns-established:
  - "All enum dispatch sites (DefKind, TyKind, ResolvedDecl, TypedDecl) updated with Class/ExternClass arms matching Struct pattern"
  - "Class codegen emits TypeDefKind::Class (kind=4) distinguishing it from Struct (kind=0) for Phase 02 IL emission"

requirements-completed: [COMP-01]

# Metrics
duration: 31min
completed: 2026-03-13
---

# Phase 50 Plan 01: Class Keyword Frontend Pipeline Summary

**`class` keyword threaded through all 8 compiler layers: lexer, CST, parser, AST, lowering, name resolution, type checker, and codegen — with parse-time rejection of on create/finalize on struct types**

## Performance

- **Duration:** 31 min
- **Started:** 2026-03-13T01:33:44Z
- **Completed:** 2026-03-13T02:04:08Z
- **Tasks:** 2
- **Files modified:** 27

## Accomplishments
- Token::KwClass added to lexer; ClassDecl/ClassMember/ExternDecl::Class added to CST
- Full class_decl and extern_class parsers with all lifecycle hooks supported
- struct_on_hook validates and rejects on create/finalize with diagnostic message
- Complete compiler pipeline: AstClassDecl, lower_class(), DefKind::Class/ExternClass, TyKind::Class(DefId), TypedDecl::Class/ExternClass
- All type-checking dispatch sites (field access, method dispatch, new construction, unification) handle TyKind::Class identically to TyKind::Struct
- Codegen emits TypeDefKind::Class (kind=4) for class declarations

## Task Commits

Each task was committed atomically:

1. **Task 1: Add class keyword to lexer, CST, parser, and struct hook rejection** - `ca4b32b` (feat)
2. **Task 2: Add class to AST, lowering, name resolution, type checking, and all match dispatch sites** - `d094550` (feat)

## Files Created/Modified
- `writ-parser/src/lexer.rs` - Token::KwClass added after KwStruct
- `writ-parser/src/cst.rs` - ClassDecl, ClassMember, ExternDecl::Class added; Item::Class added
- `writ-parser/src/parser.rs` - class_decl, extern_class parsers; struct_on_hook validation added
- `writ-compiler/src/ast/decl.rs` - AstClassDecl, AstDecl::Class, AstExternDecl::Class
- `writ-compiler/src/lower/mod.rs` - lower_class(), lower_class_member(), Item::Class dispatch
- `writ-compiler/src/resolve/def_map.rs` - DefKind::Class, DefKind::ExternClass
- `writ-compiler/src/resolve/collector.rs` - AstDecl::Class and AstExternDecl::Class collection
- `writ-compiler/src/resolve/resolver.rs` - AstDecl::Class and AstExternDecl::Class resolution
- `writ-compiler/src/resolve/ir.rs` - ResolvedDecl::Class, ResolvedDecl::ExternClass
- `writ-compiler/src/check/ty.rs` - TyKind::Class(DefId) with display arm
- `writ-compiler/src/check/ir.rs` - TypedDecl::Class, TypedDecl::ExternClass
- `writ-compiler/src/check/check_decl.rs` - Class/ExternClass arms; impl self_type for Class
- `writ-compiler/src/check/env.rs` - Class/ExternClass field building; find_class_decl helpers
- `writ-compiler/src/check/check_expr.rs` - Field access, new construction for TyKind::Class
- `writ-compiler/src/check/infer.rs` - DefKind::Class/ExternClass produces TyKind::Class
- `writ-compiler/src/check/unify.rs` - TyKind::Class unification rule
- `writ-compiler/src/emit/collect.rs` - collect_class, collect_extern_class; export/attribute dispatch
- `writ-compiler/src/emit/type_sig.rs` - TyKind::Class in type encoding
- `writ-compiler/src/emit/body/expr.rs` - extract_type_def_id and call dispatch for Class

## Decisions Made
- ClassDecl is a separate CST/AST type from StructDecl (not a shared enum with discriminant) for clarity, even though fields share StructField/AstStructMember shape
- Class fields are stored in `type_env.struct_fields` (same map as Struct) since field lookup logic is identical — the distinction only matters at codegen time (kind=4)
- Parse-and-report pattern for on create/finalize: diagnostic is emitted but the hook is still parsed to avoid cascading parser errors downstream

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed missing Value::InlineStruct match arm in cli_integration.rs**
- **Found during:** Task 2 (test run)
- **Issue:** cli_integration.rs test had a match on Value that was missing InlineStruct (added in Phase 49), preventing test compilation
- **Fix:** Added `Value::InlineStruct { .. } => "<struct>".to_string()` arm
- **Files modified:** writ-cli/tests/cli_integration.rs
- **Verification:** cargo test --workspace passes
- **Committed in:** d094550 (Task 2 commit)

**2. [Rule 1 - Bug] Fixed existing parser tests testing on create/finalize on structs**
- **Found during:** Task 1 test run
- **Issue:** Four parser tests (struct_with_on_create_hook, struct_interleaved_fields_and_hooks, struct_all_four_lifecycle_hooks, struct_hook_with_body) used struct + on create/finalize — now correctly rejected by parser
- **Fix:** Changed tests to use class keyword; updated assertions to Item::Class/ClassMember
- **Files modified:** writ-parser/tests/parser_tests.rs
- **Verification:** All 239+ parser tests pass
- **Committed in:** ca4b32b (Task 1 commit)

**3. [Rule 1 - Bug] Fixed existing lowering tests testing on create/finalize on structs**
- **Found during:** Task 2 test run
- **Issue:** lower_struct_lifecycle_hook and lower_struct_multiple_hooks tests used struct + on create/finalize
- **Fix:** Changed tests to use class keyword; regenerated snapshots (AstDecl::Class/AstClassDecl output)
- **Files modified:** writ-compiler/tests/lowering_tests.rs + 2 snapshot files
- **Verification:** All 112 lowering tests pass
- **Committed in:** d094550 (Task 2 commit)

**4. [Rule 1 - Bug] Fixed 08_dialogue.writ using class as field name**
- **Found during:** Task 1 test run
- **Issue:** player.class field access in test file — class is now a reserved keyword
- **Fix:** Renamed player.class to player.job in the test fixture
- **Files modified:** writ-parser/tests/cases/08_dialogue.writ
- **Verification:** parse_08_dialogue and recovery tests pass
- **Committed in:** ca4b32b (Task 1 commit)

---

**Total deviations:** 4 auto-fixed (all Rule 1 - existing tests using now-restricted patterns)
**Impact on plan:** All auto-fixes are direct consequences of making class a reserved keyword and restricting on create/finalize to classes only. No scope creep.

## Issues Encountered
None beyond the test updates documented above.

## Next Phase Readiness
- All 8 compiler layers support class: lexer, CST, parser, AST, lowering, name resolution, type checker, codegen
- TypeDefKind::Class (kind=4) is already emitted by collect_class — Plan 02 (IL body emission) can use this immediately
- `on create`/`on finalize` diagnostic message is in place for struct bodies
- Blockers: none

## Self-Check: PASSED

- FOUND: .planning/phases/50-compiler-frontend/50-01-SUMMARY.md
- FOUND: ca4b32b (Task 1 commit)
- FOUND: d094550 (Task 2 commit)
- FOUND: Token::KwClass in lexer.rs
- FOUND: ClassDecl in cst.rs
- FOUND: AstClassDecl in ast/decl.rs
- FOUND: DefKind::Class in def_map.rs
- FOUND: TyKind::Class in ty.rs
- FOUND: TypedDecl::Class in check/ir.rs

---
*Phase: 50-compiler-frontend*
*Completed: 2026-03-13*
