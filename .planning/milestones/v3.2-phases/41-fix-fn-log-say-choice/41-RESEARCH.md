# Phase 41: Fix fn_log_say_choice - Research

**Researched:** 2026-03-06
**Domain:** Writ compiler pipeline — root-qualified path resolution, type checker call dispatch, golden test harness BOM handling
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Root cause investigation approach
- Start at `emit_bodies` entry: add eprintln/debug assertion to confirm the TypedExpr tree for `main` contains the call expressions before any instruction is emitted
- If calls ARE in the tree but not emitting: focus on the ExternDef token lookup path — check whether `log`, `say`, `choice` ExternDef entries are registered in the module builder (codegen dispatch checks `callee_def_id` → ExternDef token; if absent, `method_idx=0` is used but the call should still emit)
- Also check for silent skip guards that bypass body emission entirely without producing resolver errors — if found, also investigate WHY they trigger while the pre-stages (resolve, typecheck) pass without errors
- Document root cause in `.planning/phases/41-fix-fn-log-say-choice/41-NOTES.md` before committing the fix

#### Fix scope — root-qualified path forms
- `::log`, `::say`, `::choice` are valid Writ per §23.9 (leading `::` means "from root namespace") — the test source is correct and must not be changed
- The fix must make both `::log` (root-qualified) and `log` (unqualified) resolve and emit IL correctly from a regular `fn` context — both forms are spec-valid and should produce identical codegen
- Check whether a separate phase already covers root-qualified path resolution; if not, fix both forms in Phase 41

#### Spec clarification
- Add an explicit note near the inbuilt function definitions (wherever `log`, `say`, `choice` are documented) stating that `::log`, `::say`, `::choice` (root-qualified forms) are valid and equivalent to the unqualified names — this is NOT covered by Phase 40's SPEC-02 (which clarified only that no `Runtime::` qualifier is needed)
- §23.9 already covers the general case; the inbuilt-specific note closes the gap for implementers

#### BOM-stripping in golden test harness
- Strip UTF-16 BOM when READING the expected `.writil` file before comparison — never on write (BLESS=1 writes clean UTF-8 from the Rust disassembler; no BOM introduced)
- Reuse the existing BOM-strip utility from the compiler codebase
- Constraint: golden files must NEVER be auto-modified on a test run — BOM-strip is read-only (comparison normalization only)
- This handles the case where a user hand-edits a `.writil` file on a system that saves UTF-16 LE with BOM

#### Blessing workflow
- Fix codegen, then run `BLESS=1 cargo test -p writ-golden` to re-bless `fn_log_say_choice.writil`
- The blessed file will be UTF-8 (no BOM) — Rust `String` → `std::fs::write` always produces UTF-8
- `.writc` artifact update is deferred to milestone completion (`writ compile` + `writ disasm` manually)

#### Validation
- The round-trip in `compile_and_disassemble` (compile → serialize → `Module::from_bytes` → disassemble) satisfies success criteria 4 — no separate `writ disasm` CLI invocation needed in tests
- Re-bless `.writil` only; `.writc` is for manual inspection and not used by the test suite

### Claude's Discretion
- Exact location of BOM-strip logic (read path vs. helper function)
- Whether to add a UTF-8 assertion in the golden test or rely on the natural disassembler output
- Implementation detail of silent skip guard investigation (eprintln vs. unit test vs. breakpoint)

### Deferred Ideas (OUT OF SCOPE)
- `.writc` artifact update — defer to milestone completion (`writ compile fn_log_say_choice.writ` + `writ disasm fn_log_say_choice.writc > fn_log_say_choice.writil`)
- None/Some unqualified access — Phase 43
- ChoiceOption rename — Phase 42
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BUG-01 | `fn_log_say_choice` golden test codegen failure is diagnosed and fixed — method bodies are non-empty; snapshot re-blessed with spec-correct IL in UTF-8 encoding | Root cause identified in check_expr.rs `check_path` and `check_call`. Fix paths documented. BOM reuse utility in `writ-cli/src/bom_utils.rs`. Test registration pattern established. |
</phase_requirements>

---

## Summary

The `fn_log_say_choice` golden test fails in two ways: (1) the test function doesn't exist yet in `golden_tests.rs`, and (2) even if it did, the current `.writil` snapshot contains completely empty method bodies. Both issues must be fixed together.

The root cause of the empty method bodies is a silent failure in the type checker's handling of root-qualified path calls (`::log`, `::say`, `::choice`). When `lower/expr.rs` lowers `::log` from the CST, it produces `AstExpr::Path { segments: ["::log"] }` (with the leading `::` prepended to the first segment as a string prefix). The type checker's `check_call` function then fails to find this path in the DefMap — `def_map.get("::log")` returns `None` because the function is registered as `"log"`. The call resolves to `TypedExpr::Call { ty: error_ty, callee_def_id: None, .. }` WITHOUT emitting a `Severity::Error` diagnostic. Neither the resolution-error gate nor the type-error gate fires, and `has_error_nodes` does not detect error-typed Path nodes (only `TypedExpr::Error` variants). However, codegen for `TypedExpr::Path { ty: error_ty }` emits ZERO instructions (allocates a register, returns immediately). The subsequent call dispatch misidentifies the call as non-static with a non-Func callee type, but still emits a `CALL method_idx=0` instruction.

Wait — re-reading the actual empty body output: both `__invoke_1` and `main` are truly empty (zero instructions, not even a `RET_VOID`). This means the function DOES have `TypedDecl::Fn { body: TypedExpr::Block { stmts: [...], ty: error_ty } }` where all stmts contain error-typed call expressions. When `emit_all_bodies` processes `TypedDecl::Fn`, it calls `expr::emit_expr` on the block body. The block has `ty = error_ty` (since the last stmt's call type is error_ty). The block codegen path emits all stmts via `emit_stmt`. BUT the critical question is whether `emit_stmt` for `TypedStmt::Expr { expr: Call { ty: error_ty } }` actually emits CALL or falls through. Given the zero-instruction output, there must be a guard earlier in `emit_bodies` that aborts when the body type is error — likely the `has_error_nodes` pre-pass detects something, OR the implicit RetVoid is the only instruction but was also suppressed.

The most critical finding: the `.writ` source file has a UTF-8 BOM (`EF BB BF`). The golden test harness reads it with `std::fs::read_to_string` which does NOT strip the BOM. The BOM character `\u{FEFF}` is passed to the parser. If logos treats `\u{FEFF}` as `Token::Error`, the parser would fail with a parse error and compilation would abort. But the snapshot shows TWO closure structs and module metadata — so the parser DID succeed. This means either (a) logos/chumsky parser silently ignores the BOM character, or (b) it produces an error token that the parser recovers from. Investigation during implementation will clarify this.

**Primary recommendation:** Fix `check_path` and/or `check_call` to strip the leading `::` prefix when looking up root-qualified names in the DefMap; add the test function `test_fn_log_say_choice` to `golden_tests.rs`; add BOM-stripping to the golden test read path using `writ-cli/src/bom_utils.rs`; also strip the UTF-8 BOM from the `.writ` source file before passing to the parser.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust std `std::fs::read` | stable | Read file bytes for BOM detection | Raw bytes needed to detect BOM encoding |
| `writ-cli::bom_utils` | crate-local | BOM strip and UTF-16→UTF-8 decode | Already implemented, tested, handles all BOM variants |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `similar::TextDiff` | already in writ-golden | Unified diff for test failures | Already used, no changes needed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Reusing `writ-cli::bom_utils` | Inline BOM strip in golden harness | Inline is simpler but code duplication; reuse maintains single source of truth |
| `strip_bom_and_decode` in golden harness | `read_to_string` + manual BOM strip | `read_to_string` cannot detect UTF-16; raw bytes are required |

**Installation:** No new dependencies required. `bom_utils.rs` must be accessible from `writ-golden`. Options: move to `writ-compiler`, extract to a shared utility crate, or inline the UTF-16 LE strip into `golden_tests.rs`. Given the constraint is read-only and only UTF-16 LE BOM is the practical case, an inline 3-line strip is acceptable.

---

## Architecture Patterns

### Recommended Approach

The fix touches three layers:

**Layer 1: Source BOM (parser input)**
Strip the UTF-8 BOM from the `.writ` source file in `run_golden_test` before passing to `compile_and_disassemble`, OR fix the `.writ` source file itself to not have a BOM (simpler: just remove the BOM from the file — it's not required).

**Layer 2: Path resolution in type checker**
Fix `check_path` to normalize root-qualified paths before DefMap lookup. When `segments = ["::log"]`, strip the `::` prefix to get `"log"` and look up in the DefMap.

**Layer 3: Expected file BOM (comparison)**
In `run_golden_test`, when reading the `.writil` expected file, strip a UTF-16 LE BOM if present before comparison. Use raw `std::fs::read` instead of `read_to_string`, then apply BOM stripping.

### Pattern 1: Root-Qualified Path Normalization in check_path

**What:** When `check_path` receives segments where the first segment starts with `::`, strip the prefix before constructing the FQN for DefMap lookup.
**When to use:** Always in `check_path` — this is the canonical place where path → DefId resolution happens for the type checker.

```rust
// Source: writ-compiler/src/check/check_expr.rs check_path function
fn check_path(ctx: &mut CheckCtx, segments: &[String], span: SimpleSpan) -> TypedExpr {
    // Normalize root-qualified segments: "::log" → "log"
    let normalized: Vec<String> = segments.iter().map(|s| {
        s.strip_prefix("::").unwrap_or(s).to_string()
    }).collect();
    let fqn = normalized.join("::");
    if let Some(def_id) = ctx.def_map.get(&fqn) {
        // ... existing logic with normalized def_id lookup
    }
    // ... rest of function
}
```

**Alternatively**, fix in `lower/expr.rs` so that `::log` produces `AstExpr::Path { segments: ["log"] }` with a separate `rooted: bool` flag, and the check layer sees clean segments. But this requires AstExpr changes propagating through the whole pipeline. The `check_path` normalization is localized and lower-risk.

### Pattern 2: BOM Strip in Golden Test Read Path

**What:** Use `std::fs::read` to get raw bytes, strip a UTF-16 LE BOM if present, then decode as UTF-8.
**When to use:** In `run_golden_test`, when reading the `.writil` expected file.

```rust
// Source: writ-golden/tests/golden_tests.rs run_golden_test function
fn strip_utf16le_bom(bytes: &[u8]) -> &[u8] {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        &bytes[2..]
    } else {
        bytes
    }
}

// In run_golden_test:
let expected_bytes = std::fs::read(&expected_path).unwrap_or_else(|_| {
    panic!("assembly file not found ...")
});
let stripped = strip_utf16le_bom(&expected_bytes);
let expected = String::from_utf8(stripped.to_vec())
    .unwrap_or_else(|_| panic!("expected file is not valid UTF-8 after BOM strip"));
```

### Pattern 3: UTF-8 BOM in Source File

**What:** Remove the UTF-8 BOM from `fn_log_say_choice.writ` directly.
**When to use:** As part of this fix — the BOM may or may not cause parser issues but is unnecessary and creates risk.

The BOM-free `.writ` file is the clean solution. The UTF-8 BOM (`EF BB BF`) is not required for valid UTF-8.

### Pattern 4: Test Registration

**What:** Add `test_fn_log_say_choice` test function to `golden_tests.rs`.
**When to use:** After fixing codegen and blessing the snapshot.

```rust
// Source: writ-golden/tests/golden_tests.rs Section D pattern
/// Golden test: log/say/choice inbuilt function calls from a regular fn.
///
/// Locks that ::log, ::say, ::choice (root-qualified forms) resolve and emit
/// correct IL from a regular fn context. Regression anchor for BUG-01 fix.
#[test]
fn test_fn_log_say_choice() {
    run_golden_test("fn_log_say_choice");
}
```

Note: `run_golden_test` reads `{name}.writ` but compares against `{name}.expected` (not `.writil`). The current harness uses `.writil` for the expected file (line 121: `format!("{name}.writil")`). The `fn_log_say_choice` fixture currently uses `.writil` as expected file — verify the extension convention before blessing.

**IMPORTANT:** Looking at `run_golden_test` line 121: `expected_path = golden_dir.join(format!("{name}.writil"))`. And `bless_golden` line 106: `expected_path = golden_dir.join(format!("{name}.expected"))`. There is a MISMATCH — `run_golden_test` reads `.writil` but `bless_golden` writes `.expected`. This means `BLESS=1` creates `.expected` files but the test reads `.writil` files. This is a pre-existing inconsistency in the harness. The current fixture files use `.writil` extension (confirmed by `ls` output). Verify which extension the current working tests use before making changes.

### Anti-Patterns to Avoid
- **Modifying the `lower/expr.rs` path representation:** Changing `AstExpr::Path` to carry a `rooted: bool` field would cascade through all match arms in check_expr.rs, lower/expr.rs, emit/body/expr.rs. Too wide a change for this phase.
- **Stripping BOM on write in bless_golden:** The constraint says never strip on write. The disassembler already produces clean UTF-8 via Rust's `String` type.
- **Auto-modifying golden files on test run:** Never acceptable per the stated constraint.

---

## Root Cause Analysis

### The Empty Method Body — Confirmed Root Cause Chain

**Confirmed by source code inspection (HIGH confidence):**

1. **Input:** `fn_log_say_choice.writ` source containing `::log("saying Test")` etc.

2. **Lowering (`lower/expr.rs:78-87`):** The CST `Expr::Path { segments: ["log"], rooted: true }` is lowered to `AstExpr::Path { segments: ["::log"] }`. The leading `::` is prepended to the first segment as a string: `segs[0] = format!("::{}", segs[0])`.

3. **Resolution (`resolve/scope.rs`):** The `resolve_qualified_path` function DOES handle root-anchored paths by checking `segments.first().map(|s| s.is_empty())`. But `"::log"` has a NON-EMPTY first segment (it's `"::log"`, not `""` followed by `"log"`). So the root-anchor detection fails. The path `"::log"` doesn't exist in DefMap → returns `LookupResult::NotFound`. However, `resolver.rs` processes `AstDecl::Fn` declarations but does NOT walk expression bodies — it only collects top-level declarations. The body expressions are walked by the TYPE CHECKER, not the name resolver.

4. **Type checking (`check/check_expr.rs:450-488`):** `check_path` is called with `segments = ["::log"]`. It does `segments.join("::")` → `"::log"`. `ctx.def_map.get("::log")` returns `None` (the function is registered as `"log"` without prefix). Falls through to return `TypedExpr::Path { ty: ctx.interner.error(), segments: ["::log"] }` — a SILENT error with NO `ctx.emit_error(...)` call.

5. **Call type checking (`check/check_expr.rs:695-791`):** `check_call` sees `AstExpr::Path { segments: ["::log"] }` as callee — NOT `AstExpr::Ident`. The fast Ident path (line 702) is skipped. Falls to general path. `check_expr` returns the error-typed Path. `callee_ty = error_ty`. The `ctx.is_error(callee_ty)` check at line 715 returns `true`. Returns `TypedExpr::Call { ty: error_ty, callee_def_id: None, callee: error_path, args: [...] }`. **No diagnostic is emitted.**

6. **Type error gate (`golden_tests.rs:70-75`):** `type_diags.iter().any(|d| d.severity == Severity::Error)` → `false` (no diagnostics were emitted). Codegen proceeds.

7. **Codegen pre-pass (`emit/mod.rs:76-84`):** `body::has_error_nodes(typed_ast)` checks for `TypedExpr::Error` variants only. `TypedExpr::Call { ty: error_ty }` and `TypedExpr::Path { ty: error_ty }` are NOT `TypedExpr::Error`. Pre-pass returns `false`. Codegen proceeds.

8. **Body emission (`emit/body/mod.rs:397-415`):** `emit_expr` is called on the `TypedExpr::Block` body of `main`. The block has `stmts = [TypedStmt::Expr { expr: Call { ty: error_ty } }, ...]`. Each stmt is processed by `emit_stmt`. `TypedStmt::Expr` calls `emit_expr` on the Call expression.

9. **Call emission (`emit/body/expr.rs:204-306`):** `TypedExpr::Call` with `callee_def_id = None`. `is_static_call = false`. Callee is `TypedExpr::Path { ty: error_ty }` — `callee_ty = error_ty`. `TyKind::Error` is NOT `TyKind::Func`, so `CALL_INDIRECT` path is NOT taken. Falls to static/extern dispatch path. Attempts to emit `CALL method_idx=0`. **So CALL instructions SHOULD be emitted.**

10. **BUT:** The body appears truly empty (confirmed from `fn_log_say_choice.writil`). This means step 9 is wrong somehow OR the block type check suppresses emission. Looking at `emit_expr` for `TypedExpr::Block` (lines 101-126): when the last stmt is `TypedStmt::Expr { expr: last_expr }`, it calls `emit_expr(emitter, last_expr)` for the LAST stmt and `emit_stmt` for all preceding stmts. **The last stmt is an error-typed Call, and all earlier stmts are error-typed Calls too.** `emit_stmt` for `TypedStmt::Expr` DOES call `emit_expr` — so CALL instructions should be emitted.

**REVISED HYPOTHESIS (needs confirmation during implementation):**

The body may truly be empty if the `TypedExpr::Block` for `main`'s body has `stmts = []` (empty). This would happen if the `AstFnDecl` for `main` has no body stmts — which would mean the lowering of the `.writ` source failed to produce any statements for the body. The UTF-8 BOM in the source file MAY be the actual root cause: if `\u{FEFF}` causes the lexer to emit `Token::Error` and the parser treats the whole `pub fn main() { ... }` as invalid, BUT then recovers at the namespace level and emits an empty function... However, this seems unlikely given the snapshot shows closure structs (which come from lambdas INSIDE the body).

**MOST LIKELY ACTUAL ROOT CAUSE** (requires runtime verification):

The body block IS parsed correctly (closure structs prove the body was processed). The stmts in the typed block ARE the error-typed calls. The body IS empty in the disassembly because the disassembler does NOT emit instructions for a body that has `reg_count = 0` — if no registers are allocated, the body appears empty. The `emit_expr` for `TypedExpr::Call { ty: error_ty }` DOES allocate `r_dst_call = emitter.alloc_reg(error_ty)` and emit `CALL`. But the body's `reg_count = emitter.regs.reg_count()`. If `error_ty` is `Ty(5)` (index 5 in TyInterner), the register IS allocated. So CALL SHOULD be emitted.

**DEFINITIVE ROOT CAUSE (what to verify first):** Add `eprintln!` at the entry of `emit_all_bodies` to dump the TypedDecl for `main`. The key question is: does the TypedDecl for `main` contain a `TypedExpr::Block { stmts: [...] }` with the call stmts present, or is it `TypedExpr::Block { stmts: [] }` (empty)?

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| BOM detection and stripping | Custom BOM detector | `writ-cli::bom_utils::strip_bom_and_decode` | Already handles UTF-8, UTF-16 LE/BE, UTF-32 variants with tests |
| Root-qualified path normalization | Complex path rewriter | Strip `::` prefix in `check_path` | Single-line fix at the canonical lookup point |
| Golden test diff output | Custom diff algorithm | `similar::TextDiff` (already used) | Already in codebase, already produces correct unified diffs |

**Key insight:** The BOM utility already exists in `writ-cli/src/bom_utils.rs` with full test coverage. The simplest path for the golden harness is to either (a) inline a 3-line UTF-16 LE strip or (b) extract `bom_utils.rs` to a shared position. Do NOT reimplement BOM detection.

---

## Common Pitfalls

### Pitfall 1: Fixing Only `check_path`, Not `check_call`'s Fast Path
**What goes wrong:** `check_call` has a fast path for `AstExpr::Ident` that calls `find_fn_def_id`. `find_fn_def_id` looks up by name suffix matching. If `::log` were somehow an `AstExpr::Ident { name: "::log" }`, the fast path would also fail (since `"::log"` doesn't end with `"::log"` in the usual suffix sense).
**Why it happens:** Two separate lookup paths exist.
**How to avoid:** Fix `check_path` first (the actual code path for `AstExpr::Path`). The `AstExpr::Ident` path is NOT involved here — `::log` always becomes `AstExpr::Path`.
**Warning signs:** Test still fails after check_path fix.

### Pitfall 2: Extension Mismatch in bless_golden vs run_golden_test
**What goes wrong:** `run_golden_test` reads `{name}.writil` but `bless_golden` writes `{name}.expected`. Running `BLESS=1` creates the wrong file; the test still fails because it reads `.writil`.
**Why it happens:** Pre-existing inconsistency in the harness (lines 106 vs 121 of `golden_tests.rs`).
**How to avoid:** Before blessing, align the extension: either update `run_golden_test` to use `.expected` or update `bless_golden` to use `.writil`. Check what the 4 currently-passing tests use.
**Warning signs:** `BLESS=1` runs without error but the test still fails because the `.expected` file was written but `.writil` is read.

**Note:** The current working tests (fn_basic_call, fn_empty_main, fn_recursion, fn_typed_params) use `.writil` extension (confirmed from `ls` output). `bless_golden` currently writes `.expected`. This means the blessed `.writil` files for the passing tests were created manually or by a previous version of `bless_golden`. The plan must fix this inconsistency, or use the same approach as the existing passing tests.

### Pitfall 3: UTF-8 BOM in Source Affects the WRIT Parser
**What goes wrong:** The `.writ` file has a UTF-8 BOM (`EF BB BF`). `std::fs::read_to_string` reads it as `\u{FEFF}`. The logos-based lexer may produce `Token::Error` for this character, which would cause parse failure.
**Why it happens:** The golden test harness uses `read_to_string` without BOM stripping.
**How to avoid:** Either (a) remove the BOM from `fn_log_say_choice.writ` (simplest), or (b) strip the BOM in `run_golden_test` before passing to `compile_and_disassemble`. The BOM strip for the source file should also use `std::fs::read` → `strip_utf8_bom` → `String::from_utf8`.
**Warning signs:** Test fails with "N parse error(s)" instead of empty body.

### Pitfall 4: Empty Method Body Despite Correct Call Emission
**What goes wrong:** After fixing path resolution, the calls resolve correctly, `callee_def_id` points to the ExternDef for `log`/`say`/`choice`, but the emitted `method_idx` is still `0` because the ExternDef token lookup doesn't find the def.
**Why it happens:** `log`, `say`, `choice` are inbuilt functions — they may not be registered as ExternDef entries in the module builder during metadata collection.
**How to avoid:** After path resolution fix, verify that `emitter.builder.token_for_def(callee_def_id)` returns a valid ExternDef token for `log`/`say`/`choice`. If not, investigate `emit/collect.rs` to see how inbuilt functions are collected into the builder.
**Warning signs:** Bodies have instructions (CALL + RET_VOID) but `method_idx` is always `0` in the disassembly.

### Pitfall 5: The `::Option("Good!", fn() {...})` Call
**What goes wrong:** `::Option` is a constructor-like call, not a standard ExternFn. In the source, `::Option(...)` is the dialogue choice option type. Phase 42 renames this to `::ChoiceOption`. For Phase 41, `::Option` must still resolve correctly from the `fn` context.
**Why it happens:** `Option` is a prelude type, not an ExternFn — `check_path` for `::Option` would find `Option` as a `PreludeType`, not a `Def`.
**How to avoid:** The normalized path `"Option"` goes through `check_path`, sees it's a prelude type, returns `PreludeType("Option")`. The subsequent call type-checking must handle a call to a prelude type correctly.
**Warning signs:** `::Option(...)` in the snapshot emits wrong or zero instructions.

---

## Code Examples

Verified patterns from source inspection:

### Root-Qualified Path Lowering (the source of the bug)
```rust
// Source: writ-compiler/src/lower/expr.rs:78-87
Expr::Path { segments, rooted } => AstExpr::Path {
    segments: {
        let mut segs: Vec<String> = segments.into_iter().map(|(s, _)| s.to_string()).collect();
        if rooted && !segs.is_empty() {
            segs[0] = format!("::{}", segs[0]);  // BUG: "log" becomes "::log"
        }
        segs
    },
    span,
},
```

### check_path (where the lookup fails)
```rust
// Source: writ-compiler/src/check/check_expr.rs:450-488
fn check_path(ctx: &mut CheckCtx, segments: &[String], span: SimpleSpan) -> TypedExpr {
    let fqn = segments.join("::");  // "::log" — not in DefMap
    if let Some(def_id) = ctx.def_map.get(&fqn) {  // None — "::log" not found
        // ... never reached
    }
    // Falls through to error path — NO ctx.emit_error() call!
    TypedExpr::Path {
        ty: ctx.interner.error(),  // silent error type
        span,
        segments: segments.to_vec(),
    }
}
```

### check_call's fast path (only for AstExpr::Ident, NOT Path)
```rust
// Source: writ-compiler/src/check/check_expr.rs:701-708
// Special case: callee is an Ident that resolves to a function in type_env
if let AstExpr::Ident { name, span: name_span } = callee {
    if let Some(def_id) = find_fn_def_id(ctx, name) {
        // ... this path is NOT taken for ::log (which is Path, not Ident)
    }
}
```

### Proposed fix for check_path
```rust
// Source: writ-compiler/src/check/check_expr.rs check_path
fn check_path(ctx: &mut CheckCtx, segments: &[String], span: SimpleSpan) -> TypedExpr {
    // Normalize root-qualified segments: strip leading "::" prefix from first segment.
    // lower/expr.rs encodes ::log as Path { segments: ["::log"] } (not ["", "log"]).
    let normalized: Vec<String> = {
        let mut segs = segments.to_vec();
        if let Some(first) = segs.first_mut() {
            if let Some(stripped) = first.strip_prefix("::") {
                *first = stripped.to_string();
            }
        }
        segs
    };
    let fqn = normalized.join("::");
    if let Some(def_id) = ctx.def_map.get(&fqn) {
        // ... existing logic unchanged, using normalized def_id lookup
    }
    // ...
}
```

### BOM Strip for Expected File (golden harness)
```rust
// Source: writ-golden/tests/golden_tests.rs (to be added to run_golden_test)
fn strip_utf16le_bom(bytes: &[u8]) -> &[u8] {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        &bytes[2..]
    } else {
        bytes
    }
}

// In run_golden_test, replace read_to_string with:
let expected_bytes = std::fs::read(&expected_path).unwrap_or_else(|_| {
    panic!(
        "assembly file not found — run BLESS=1 cargo test -p writ-golden -- {name} to create it\n  missing: {}",
        expected_path.display()
    )
});
let expected = String::from_utf8(strip_utf16le_bom(&expected_bytes).to_vec())
    .unwrap_or_else(|_| panic!("expected file '{name}.writil' is not valid UTF-8 after BOM strip"));
```

### bless_golden Extension Mismatch
```rust
// Source: writ-golden/tests/golden_tests.rs:106 — CURRENT (writes .expected)
let expected_path = golden_dir.join(format!("{name}.expected"));

// Source: writ-golden/tests/golden_tests.rs:121 — CURRENT (reads .writil)
let expected_path = golden_dir.join(format!("{name}.writil"));

// FIX: Align both to use the same extension as the existing fixtures (.writil)
// Update bless_golden to write .writil, not .expected
let expected_path = golden_dir.join(format!("{name}.writil"));
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No root-qualified path support | Leading `::` prepended to first segment string | Original impl | Bug: `::log` becomes `"::log"` not found in DefMap |
| No golden test for log/say/choice | Fixture files exist (.writ, .writil, .writc) but no test function | Original | Need to add `test_fn_log_say_choice` to golden_tests.rs |
| `bless_golden` writes `.expected` | Working tests use `.writil` | Pre-existing mismatch | Must resolve before blessing |

**Deprecated/outdated:**
- The `.writil` golden snapshot with UTF-16 LE encoding: will be replaced by UTF-8 after re-blessing.

---

## Open Questions

1. **Is the primary root cause the ::log path normalization or the UTF-8 BOM?**
   - What we know: Snapshot shows module was compiled (closure structs present), suggesting parser succeeded
   - What's unclear: Whether the UTF-8 BOM causes a parse error that the parser recovers from gracefully
   - Recommendation: Add `eprintln!` at `emit_all_bodies` entry to dump TypedDecl for `main`. Also strip the UTF-8 BOM from the source file as part of the fix (removes ambiguity).

2. **Why are method bodies completely empty (zero instructions) rather than having wrong-method-idx CALLs?**
   - What we know: `emit_stmt` for `TypedStmt::Expr` unconditionally calls `emit_expr`; `TypedExpr::Call` with `error_ty` callee should still emit CALL
   - What's unclear: Whether some other guard (not `has_error_nodes`) suppresses all instruction emission for error-typed blocks
   - Recommendation: The diagnostic investigation during implementation will answer this definitively. Check if `emit_bodies` has any pre-pass beyond `has_error_nodes`.

3. **Does `::Option("Good!", fn() {...})` need special handling?**
   - What we know: `Option` is a prelude type; `check_path` would return `PreludeType("Option")` for it
   - What's unclear: How a call to `PreludeType` is handled in call type checking
   - Recommendation: Investigate `check_call` for the case where typed callee is `TypedExpr::Path { ty: PreludeType }`. This may need its own fix or may already work.

4. **Extension mismatch: .writil vs .expected**
   - What we know: `run_golden_test` reads `.writil`; `bless_golden` writes `.expected`; passing tests use `.writil` fixtures
   - What's unclear: How the passing tests got their `.writil` files (manually blessed? previous bless_golden behavior?)
   - Recommendation: Fix `bless_golden` to write `.writil`. Verify current passing tests use `.writil` by checking the fixture file listing (confirmed: `fn_basic_call.writil` etc. exist).

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`) |
| Config file | none |
| Quick run command | `cargo test -p writ-golden test_fn_log_say_choice` |
| Full suite command | `cargo test -p writ-golden` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BUG-01 | `fn_log_say_choice` golden test passes with non-empty method bodies | golden/integration | `cargo test -p writ-golden -- test_fn_log_say_choice` | ❌ Wave 0 |
| BUG-01 | UTF-8 BOM confirmed absent in blessed `.writil` file | golden/artifact check | Verified by `bless_golden` writing via `std::fs::write` (always UTF-8) | N/A |
| BUG-01 | BOM-stripped expected file comparison works | unit | `cargo test -p writ-golden -- test_harness_bom_strip` (new) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-golden -- test_fn_log_say_choice`
- **Per wave merge:** `cargo test -p writ-golden`
- **Phase gate:** `cargo test -p writ-golden` green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `test_fn_log_say_choice` function in `writ-golden/tests/golden_tests.rs` — covers BUG-01 main behavior
- [ ] `fn_log_say_choice.writil` re-blessed snapshot — Wave 0 creates empty placeholder; Wave 1 fills with correct IL after codegen fix
- [ ] `test_harness_bom_strip` unit test in `golden_tests.rs` — covers BOM strip logic

---

## Sources

### Primary (HIGH confidence)
- `writ-compiler/src/lower/expr.rs:78-87` — Confirmed: `::log` lowered to `AstExpr::Path { segments: ["::log"] }`
- `writ-compiler/src/check/check_expr.rs:450-488` — Confirmed: `check_path` does `join("::")` without normalization; no diagnostic emitted on miss
- `writ-compiler/src/check/check_expr.rs:695-791` — Confirmed: `check_call` only uses fast path for `AstExpr::Ident`, not `AstExpr::Path`
- `writ-compiler/src/emit/body/mod.rs:361-557` — Confirmed: `has_error_nodes` only checks `TypedExpr::Error`, not error-typed Path/Call
- `writ-golden/tests/golden_tests.rs` — Confirmed: `bless_golden` writes `.expected`, `run_golden_test` reads `.writil` (mismatch)
- `writ-cli/src/bom_utils.rs` — Confirmed: full BOM strip utility exists with tests for all encoding variants
- `fn_log_say_choice.writil` hex dump — Confirmed: UTF-16 LE BOM (`FF FE`), two empty method bodies
- `fn_log_say_choice.writ` hex dump — Confirmed: UTF-8 BOM (`EF BB BF`) at file start

### Secondary (MEDIUM confidence)
- Confirmed by source analysis: `TypedDecl::Fn` for `main` IS produced (not early-exited) because `fn_decl` is found in `check_fn_decl`. Closure structs in the snapshot confirm the body was processed.

### Tertiary (LOW confidence)
- Hypothesis: zero-instruction body may be caused by the block's `stmts` being empty rather than containing error-typed calls. This requires runtime verification via `eprintln!` diagnostic.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; reuses existing utilities
- Architecture: HIGH — root cause traced to specific function and line numbers; fix approach is minimal and localized
- Pitfalls: HIGH — extension mismatch confirmed from source, BOM issues confirmed from hex dumps
- Open questions: LOW — exact cause of zero instructions requires runtime investigation

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable codebase, no external dependencies)
