---
phase: 43-unqualified-none-some
plan: 02
subsystem: resolver
tags: [rust, resolver, scope, parser, enum-glob, writ-compiler, writ-parser]

requires:
  - phase: 43-unqualified-none-some
    plan: 01
    provides: eight RED test stubs defining the LANG-02 contract

provides:
  - "LookupResult::BuiltinVariant(String) variant in scope.rs -- sub-prelude injection point for None/Some"
  - "SUB_PRELUDE_VARIANT_NAMES constant in prelude.rs -- [\"None\", \"Some\"]"
  - "resolve_value step 8: returns BuiltinVariant after all user-defined lookups fail"
  - "Parser extension: using_decl accepts optional ::* suffix (glob import)"
  - "process_usings glob branch: expands Enum::* to one UsingEntry per variant"
  - "UsingEntry enum-variant fallback: resolve_qualified_path tried when def_map.get fails"
  - "Glob conflict detection: ambiguity diagnostic at import-expansion time"

affects:
  - 43-03-PLAN

tech-stack:
  added: []
  patterns:
    - "Sub-prelude injection pattern: fallback after all user-defined lookups via matches!(result, LookupResult::NotFound)"
    - "AST-walk for variant extraction: enum variants not in DefMap, extracted directly from AstDecl::Enum.variants in process_usings"
    - "Import-time conflict detection: glob ambiguity detected when push loop finds duplicate alias in active_usings"
    - "resolve_qualified_path fallback in UsingEntry step 5: enables enum-variant FQNs (not in by_fqn) to resolve"

key-files:
  created: []
  modified:
    - writ-compiler/src/resolve/prelude.rs
    - writ-compiler/src/resolve/scope.rs
    - writ-compiler/src/resolve/resolver.rs
    - writ-parser/src/parser.rs

key-decisions:
  - "Parser extension included in 43-02 (not deferred to 43-03) because glob tests panicked at parse stage, making resolver tests unrunnable without it"
  - "Enum variants NOT stored in DefMap -- extracted from AST items list in find_enum_variants helper; this avoids a Pass 1 schema change (would be Rule 4 architectural)"
  - "Glob conflict detected at import-expansion time rather than use-site because resolver does not walk function bodies; ambiguity must be detectable without use-site lookup"
  - "UsingEntry step 5 extended with resolve_qualified_path fallback to handle enum variants (Status::Active) that resolve to the enum's DefId but are not in by_fqn"
  - "Three typecheck stubs (none_unqualified_with_annotation, some_unqualified_infers_type, none_some_in_pattern_position) remain RED -- deferred to plan 43-03 per plan design"

patterns-established:
  - "find_enum_variants: recursive AST walk for block-namespace enums -- same pattern usable for other variant-level lookups"

requirements-completed: [LANG-02]

duration: 10min
completed: 2026-03-06
---

# Phase 43 Plan 02: Resolver Sub-prelude Injection and Using-Glob Expansion Summary

**Sub-prelude None/Some injection via LookupResult::BuiltinVariant plus using Enum::* glob expansion with conflict detection and enum-variant UsingEntry resolution.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-03-06T16:35:00Z
- **Completed:** 2026-03-06T16:45:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- `LookupResult::BuiltinVariant(String)` added to scope.rs; `resolve_value` returns it for "None"/"Some" after all user-defined lookups fail (sub-prelude injection, step 8)
- Parser extended to accept `using Status::*;` syntax -- `Token::Star` appended to path as `"*"` segment
- `process_usings` glob branch: finds enum in DefMap, walks AST to extract variant names (variants not in DefMap), pushes one UsingEntry per variant; detects conflicts between two overlapping glob imports at expansion time
- `scope.rs` step 5 extended with `resolve_qualified_path` fallback so `Status::Active` FQNs (not in `by_fqn`) resolve through glob UsingEntry records
- All 9 golden tests still pass; `using_enum_glob` and `using_glob_conflict_ambiguous` now GREEN; `using_option_glob_redundant_no_error` also GREEN

## Task Commits

Each task was committed atomically:

1. **Task 1: Add BuiltinVariant to scope.rs and SUB_PRELUDE_VARIANT_NAMES to prelude.rs** - `b0602d7` (feat)
2. **Task 2: Implement using-glob expansion in process_usings (resolver.rs) + parser extension** - `5096ea7` (feat)

## Files Created/Modified

- `writ-compiler/src/resolve/prelude.rs` - Added `SUB_PRELUDE_VARIANT_NAMES = &["None", "Some"]`
- `writ-compiler/src/resolve/scope.rs` - Added `LookupResult::BuiltinVariant(String)`; updated `resolve_value` step 8; extended UsingEntry step 5 with `resolve_qualified_path` fallback
- `writ-compiler/src/resolve/resolver.rs` - Added `prelude` import; glob branch in `process_usings`; `find_enum_variants` AST-walk helper; `BuiltinVariant` arm in `resolve_ast_type`
- `writ-parser/src/parser.rs` - Extended `using_decl` parser with `qualified_name_glob` that accepts optional `::*` suffix

## Decisions Made

- **Parser extension in 43-02**: The three glob tests panicked at the parse stage before they could test resolver behavior. Since adding `::*` parser support was trivial and did not require touching check_expr.rs, it was included here rather than deferred to 43-03.
- **Enum variants not in DefMap**: Enum variants are never stored in `by_fqn` or `namespace_members`. Rather than adding them (architectural change, Rule 4), the glob expansion walks the AST `items` slice via `find_enum_variants` to extract variant names directly.
- **Import-time conflict detection**: Because the resolver doesn't walk function bodies, use-site ambiguity detection won't fire for `Green == Green`. Instead, the push loop checks for duplicate aliases among existing `active_usings` entries and emits `E0004` at import time.
- **UsingEntry step 5 fallback**: Standard specific imports use `def_map.get(target_fqn)`. For glob-expanded enum variants, the target FQN (`Status::Active`) is not in `by_fqn`. Extended the check to try `resolve_qualified_path` when direct lookup fails -- this returns the enum's DefId for `Enum::Variant` paths.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Parser extension added in Task 2 (planned for 43-03)**
- **Found during:** Task 2 (using-glob expansion)
- **Issue:** All three glob tests panicked at the parse stage ("found 'Star' at ...") before any resolver behavior could be tested; the glob resolver logic could not be verified at all without the parser accepting `::*`
- **Fix:** Extended `using_decl` parser to use `qualified_name_glob` which appends `Token::Star` as `"*"` to the path if present after `::`. No changes to check_expr.rs required.
- **Files modified:** `writ-parser/src/parser.rs`
- **Verification:** `using_option_glob_redundant_no_error` passes (previously a parse panic); all existing parser tests still pass
- **Committed in:** `5096ea7` (Task 2 commit)

**2. [Rule 1 - Bug] UsingEntry step 5 extended with resolve_qualified_path fallback**
- **Found during:** Task 2 (testing using_enum_glob)
- **Issue:** Glob-expanded UsingEntry records had `target_fqn = Some("Status::Active")`. Step 5 in `resolve_type` tried `def_map.get("Status::Active")` which returned None (variants not in by_fqn), so `Active` was never found
- **Fix:** Added fallback in step 5: when `def_map.get` returns None, try `resolve_qualified_path(&segments)` which handles `Enum::Variant` paths by returning the enum's DefId
- **Files modified:** `writ-compiler/src/resolve/scope.rs`
- **Verification:** `using_enum_glob` passes
- **Committed in:** `5096ea7` (Task 2 commit)

**3. [Rule 1 - Bug] Import-time conflict detection for overlapping globs**
- **Found during:** Task 2 (testing using_glob_conflict_ambiguous)
- **Issue:** The resolver does not walk function bodies, so the use-site path for detecting `Green` ambiguity from two overlapping globs was never exercised; `using_glob_conflict_ambiguous` expected an error but got none
- **Fix:** Added conflict check in the variant push loop: if any existing UsingEntry already has the same alias with a `target_fqn` containing `::`, emit `E0004` AmbiguousName immediately
- **Files modified:** `writ-compiler/src/resolve/resolver.rs`
- **Verification:** `using_glob_conflict_ambiguous` passes
- **Committed in:** `5096ea7` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 blocking, 2 bugs)
**Impact on plan:** All auto-fixes necessary for the tests to work correctly. No scope creep -- check_expr.rs was not touched.

## Issues Encountered

- Three typecheck stubs (`none_unqualified_with_annotation`, `some_unqualified_infers_type`, `none_some_in_pattern_position`) remain RED -- these require the check layer to understand `BuiltinVariant`, which is deferred to plan 43-03 per the plan design. They were already RED before plan 43-02 started.

## Next Phase Readiness

- Resolver infrastructure for sub-prelude None/Some is complete
- Using-glob mechanism is working at the resolver level
- Plan 43-03 needs to: (1) teach check_expr.rs to handle `BuiltinVariant` identifiers, (2) add pattern-position support for None/Some in match arms, (3) add the assertion to `none_some_in_pattern_position`

## Self-Check: PASSED

- `writ-compiler/src/resolve/prelude.rs`: FOUND (SUB_PRELUDE_VARIANT_NAMES present)
- `writ-compiler/src/resolve/scope.rs`: FOUND (BuiltinVariant variant present)
- `writ-compiler/src/resolve/resolver.rs`: FOUND (glob branch + find_enum_variants present)
- `writ-parser/src/parser.rs`: FOUND (qualified_name_glob present)
- Commit `b0602d7`: FOUND (Task 1)
- Commit `5096ea7`: FOUND (Task 2)

---
*Phase: 43-unqualified-none-some*
*Completed: 2026-03-06*
