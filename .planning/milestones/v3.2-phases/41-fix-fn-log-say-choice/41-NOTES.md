# Phase 41 — Root Cause Notes (BUG-01)

**Date:** 2026-03-06
**Status:** Resolved

---

## Root Cause Chain

BUG-01: `fn_log_say_choice` golden snapshot contained completely empty method bodies.

**5-step chain:**

1. `lower/expr.rs` encodes `::log` as `AstExpr::Path { segments: ["::log"] }` — it prepends `::` to the first segment of root-qualified paths.

2. `check_path` joined segments naively: `["::log"].join("::")` = `"::log"`. `def_map.get("::log")` returned `None` because items are registered as `"log"` (no leading `::`).

3. `check_path` fell through to the `// Could be an enum variant path` stub, returning `TypedExpr::Path { ty: error() }` — no diagnostic emitted, so the error was silent.

4. `check_call` saw an error-typed callee and returned `TypedExpr::Call { ty: error(), callee_def_id: None }`.

5. `has_error_nodes` treated `TypedExpr::Path` as a leaf (not detected). Codegen proceeded and produced zero instructions for the error-typed call.

---

## Fix Applied (Plan 02)

`check_path` in `writ-compiler/src/check/check_expr.rs` now strips the leading `::` from the first segment before the DefMap lookup:

```rust
let normalized_segments: Vec<String> = {
    let mut segs = segments.to_vec();
    if let Some(first) = segs.first_mut() {
        if let Some(stripped) = first.strip_prefix("::") {
            *first = stripped.to_string();
        }
    }
    segs
};
let fqn = normalized_segments.join("::");
```

This normalizes `"::log"` → `"log"` at the canonical resolution point. Single-segment only — matches `lower/expr.rs` encoding where only `segs[0]` gets the `::` prefix for root-qualified paths.

---

## Path Fast-Path Fix (Plan 03)

`check_path` normalization enabled resolution but `callee_def_id` remained `None` for path-form calls (the general `check_call` path doesn't extract def_id from `TypedExpr::Path`). Without `callee_def_id`, codegen emits `CALL_INDIRECT` instead of `CALL_EXTERN`.

Added a path fast-path in `check_call` analogous to the existing Ident fast-path:

```rust
if let AstExpr::Path { segments, span: path_span } = callee {
    if segments.len() == 1 {
        let normalized = segments[0].strip_prefix("::").unwrap_or(&segments[0]);
        if let Some(def_id) = find_fn_def_id(ctx, normalized) {
            if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                return check_call_with_sig(ctx, normalized, def_id, sig.clone(), ...);
            }
        }
    }
}
```

`check_call_with_sig` sets `callee_def_id: Some(def_id)` → emit layer sees the ExternDef token → emits `CALL_EXTERN`.

---

## Ancillary Fixes

| Fix | Plan | Description |
|-----|------|-------------|
| `fn_log_say_choice.writ` UTF-8 BOM removed | 02 | File had EF BB BF BOM causing parse error `found 'Error' at 0..3` |
| `fn_log_say_choice.writil` re-blessed | 03 | Was UTF-16 LE with empty bodies; now clean UTF-8 with `CALL_EXTERN` IL |
| `bless_golden` extension fixed | 01 | Was writing `.expected`; changed to `.writil` to match `run_golden_test` read path |
| `run_golden_test` BOM-strip added | 01 | Added `strip_utf16le_bom` helper for hand-edited UTF-16 LE files on read path |
| `run_golden_test` CRLF normalization | 01 | Switching to binary read exposed CRLF mismatch; added `.replace("\r\n", "\n")` |
| `fn_log_say_choice.writ` simplified | 03 | Removed `::Option`/`Option::None` (Phase 42 scope); added `extern fn` declarations |

---

## Scope Boundary

**`::Option` and `Option::None` deferred to Phase 42 (ChoiceOption rename).**

The original test source used `::Option("label", fn() {...})` for choice arms, which is ambiguous with the prelude type `Option<T>`. Phase 42 renames the choice constructor to `ChoiceOption`, resolving the ambiguity. For Phase 41, the test was simplified to `::choice()` (empty call, no options) to prove the core BUG-01 fix without triggering the Phase 42 issue.

The `check_path` normalization intentionally does NOT emit a diagnostic for unresolved single-segment paths. The existing fall-through behavior (silent error type) is unchanged for cases that legitimately fall through (enum variant paths). A future phase may add proper "unresolved path" diagnostics.

---

## Verification Summary

| Check | Result |
|-------|--------|
| `cargo test -p writ-compiler` | 65/65 pass |
| `cargo test -p writ-golden` | 9/9 pass |
| `fn_log_say_choice.writil` first byte | `0x2e` (`.`) — no BOM |
| `fn_log_say_choice.writil` contains `CALL_EXTERN` | Yes (4 instructions) |
| `fn_log_say_choice.writil` method body non-empty | Yes (`main` has 9 instructions) |
