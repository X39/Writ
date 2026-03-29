---
phase: 94-user-defined-attribute-declarations
verified: 2026-03-27T00:00:00Z
status: passed
score: 8/8 must-haves verified
re_verification: false
---

# Phase 94: User-Defined Attribute Declarations Verification Report

**Phase Goal:** Users can declare custom attributes with typed parameters, those declarations survive the full compiler pipeline into the binary module, and builtin attribute names are reserved so user attributes cannot shadow them.
**Verified:** 2026-03-27
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | `` `attribute Quest(name: string, level: int);` `` parses into an `Item::Attribute` CST node | VERIFIED | `Item::Attribute(Spanned<AttributeDecl>)` in `cst.rs:89`; `attribute_decl` combinator in `parser/program.rs:2807`; 3 parser tests pass |
| 2 | The lowered `AstDecl::Attribute` carries name, params, attrs, vis, and span | VERIFIED | `AstAttributeDecl` struct at `ast/decl.rs:445`; `lower_attribute()` wired at `lower/mod.rs:109,371` |
| 3 | `DefKind::AttributeDef` exists in the DefMap after collection | VERIFIED | `DefKind::AttributeDef` at `def_map.rs:255`; collector arm at `collector.rs:294` uses `try_insert` with `DefKind::AttributeDef`; `attribute_decl_collected` resolve test passes |
| 4 | `ResolvedDecl::AttributeDef` and `TypedDecl::AttributeDef` pass through resolver and checker | VERIFIED | `ResolvedDecl::AttributeDef { def_id }` at `resolve/ir.rs:89`; `TypedDecl::AttributeDef { def_id }` at `check/ir.rs:346`; `check_attribute_decl` at `check_decl.rs:267` dispatches param validation |
| 5 | User-defined attribute declarations produce `AttributeDef` rows with `owner_kind=3` and encoded param type signature | VERIFIED | `collect_attribute_decl_defs` at `encoding.rs:156` encodes u16 count + tag bytes; wired into `collect_post_finalize` at `mod.rs:150`; `attribute_decl_emits_def_row` emit test passes |
| 6 | Declaring an attribute named `Deprecated`, `Conditional`, `Singleton`, or `Locale` produces an E0008 name collision error | VERIFIED | `is_builtin_attribute_name` guard at `collector.rs:296` emits `ResolutionError::BuiltinAttributeShadow`; maps to E0008 in `error.rs:237`; `builtin_attribute_shadow` resolve test passes |
| 7 | The writ-runtime virtual module contains four builtin `AttributeDef` rows for Deprecated, Conditional, Singleton, and Locale | VERIFIED | Section 7 in `virtual_module.rs:376-403` calls `add_attribute_def` for all four names with `ATTR_OWNER_KIND_DECL` |
| 8 | Attribute parameter types are validated to be `string`, `int`, or `bool` only | VERIFIED | `check_attribute_decl` at `check_decl.rs:267-310` rejects any `AstType` not matching `"string" | "int" | "bool"` with E0006 |

**Score:** 8/8 truths verified

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-parser/src/lexer.rs` | `KwAttribute` token | VERIFIED | Line 245: `KwAttribute,` |
| `writ-parser/src/cst.rs` | `AttributeDecl` struct + `Item::Attribute` variant | VERIFIED | `AttributeDecl<'src>` at line 219; `Item::Attribute(Spanned<AttributeDecl<'src>>)` at line 89 |
| `writ-compiler/src/ast/decl.rs` | `AstAttributeDecl` struct + `AstDecl::Attribute` variant | VERIFIED | Struct at line 445; variant at line 33 |
| `writ-compiler/src/resolve/def_map.rs` | `DefKind::AttributeDef` variant | VERIFIED | Line 255 |
| `writ-compiler/src/resolve/ir.rs` | `ResolvedDecl::AttributeDef { def_id }` variant | VERIFIED | Line 89 |
| `writ-compiler/src/check/ir.rs` | `TypedDecl::AttributeDef { def_id }` variant | VERIFIED | Line 346 |
| `writ-compiler/src/resolve/prelude.rs` | `BUILTIN_ATTRIBUTE_NAMES` constant + `is_builtin_attribute_name` predicate | VERIFIED | Lines 31-35 |
| `writ-compiler/src/resolve/error.rs` | `BuiltinAttributeShadow` variant | VERIFIED | Line 90; From impl at line 237 |
| `writ-diagnostics/src/code.rs` | `E0008` constant | VERIFIED | Line 14: `pub const E0008: &str = "E0008";` |
| `writ-module/src/tables.rs` | `ATTR_OWNER_KIND_DECL: u8 = 3` constant | VERIFIED | Line 275 |
| `writ-runtime/src/virtual_module.rs` | Four builtin `AttributeDef` rows | VERIFIED | Lines 376-403: Deprecated, Conditional, Singleton, Locale all call `add_attribute_def` with `ATTR_OWNER_KIND_DECL` |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `parser/program.rs` | `cst.rs` | `attribute_decl` combinator produces `Item::Attribute` | WIRED | `cst::Item::Attribute` at program.rs:2807,2855 |
| `lower/mod.rs` | `ast/decl.rs` | `Item::Attribute` arm produces `AstDecl::Attribute` | WIRED | `AstDecl::Attribute(lower_attribute(...))` at lower/mod.rs:109,371 |
| `resolve/collector.rs` | `resolve/prelude.rs` | `is_builtin_attribute_name` check in `AstDecl::Attribute` arm | WIRED | Import at collector.rs:11; guard at line 296 |
| `emit/collect/mod.rs` | `emit/collect/encoding.rs` | `collect_attribute_decl_defs` called in `collect_post_finalize` | WIRED | Import at mod.rs:28; call at line 150 |
| `emit/collect/encoding.rs` | `writ-module/src/tables.rs` | writes `AttributeDef` rows with `ATTR_OWNER_KIND_DECL` | WIRED | `use writ_module::tables::ATTR_OWNER_KIND_DECL` at encoding.rs:7; used at line 209 |
| `virtual_module.rs` | `writ-module/src/builder.rs` | `add_attribute_def` calls for four builtins | WIRED | Four explicit calls at lines 381,389,395,403 |

---

## Data-Flow Trace (Level 4)

Not applicable — this phase adds a compiler pipeline feature, not a UI component rendering dynamic data. The data flow is compile-time (source -> CST -> AST -> DefMap -> TypedDecl -> binary rows), verified by the test suite.

---

## Behavioral Spot-Checks

| Behavior | Evidence | Status |
|----------|----------|--------|
| `attribute Quest(name: string, level: int);` parses to `Item::Attribute` | `attribute_decl_with_params` parser test passes | PASS |
| Quest attribute collects as `DefKind::AttributeDef` | `attribute_decl_collected` resolve test passes | PASS |
| Declaring `attribute Deprecated(...)` produces E0008 | `builtin_attribute_shadow` resolve test passes | PASS |
| Quest attribute emits `AttributeDef` row with `owner_kind=3` | `attribute_decl_emits_def_row` emit test passes | PASS |
| Full workspace test suite | `cargo test --workspace`: 0 failures across all test result lines | PASS |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| UATTR-01 | 94-01 | User can declare attributes with typed parameters using `attribute Name(args);` syntax | SATISFIED | KwAttribute token, AttributeDecl CST, AstDecl::Attribute, lowering, DefKind::AttributeDef, resolver + checker passthrough all present; param type validation in `check_attribute_decl` |
| UATTR-02 | 94-02 | User-defined attributes pass through the pipeline and appear in the module's `AttributeDef` table with serialized arguments | SATISFIED | `collect_attribute_decl_defs` produces rows with `owner_kind=3` and u16+tags blob; `attribute_decl_emits_def_row` test confirms |
| UATTR-03 | 94-02 | Builtin attributes (`[Deprecated]`, `[Conditional]`, `[Singleton]`, `[Locale]`) are registered in the writ-runtime virtual module namespace | SATISFIED | Section 7 of `build_writ_runtime_module()` adds all four with `ATTR_OWNER_KIND_DECL` |
| UATTR-04 | 94-02 | Builtin attribute names are reserved; user-defined attributes with the same name produce a name collision error | SATISFIED | `is_builtin_attribute_name` guard in collector emits `BuiltinAttributeShadow` → E0008; `builtin_attribute_shadow` test confirms DefMap exclusion |

All four requirements mapped in `REQUIREMENTS.md` are checked `[x]` and marked Complete.

---

## Anti-Patterns Found

No blockers or stubs found. Scanned modified files for `TODO`, `FIXME`, `HACK`, `PLACEHOLDER`, `unimplemented!`, `todo!`. The one `panic!` at `virtual_module.rs:603` is a pre-existing test assertion helper unrelated to attribute declarations.

---

## Human Verification Required

None. All behaviors are fully verifiable via automated test suite and static code inspection.

---

## Gaps Summary

No gaps. All 8 must-have truths verified. All artifacts exist and are substantive. All key links wired. All four UATTR requirements satisfied. Test suite passes with zero failures across the workspace.

---

_Verified: 2026-03-27_
_Verifier: Claude (gsd-verifier)_
