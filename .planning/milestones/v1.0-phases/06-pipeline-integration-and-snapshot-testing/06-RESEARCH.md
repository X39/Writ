# Phase 6: Pipeline Integration and Snapshot Testing - Research

**Researched:** 2026-02-27
**Domain:** Rust snapshot testing with `insta`; pipeline integration verification; determinism testing for localization keys
**Confidence:** HIGH — all findings sourced directly from the live codebase

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| R15 | Snapshot Testing — all lowering passes tested via `insta` snapshot tests; integration test lowers a full mixed-construct program; localization key determinism verified by comparing two runs; every pass has at least one snapshot test | The existing test infrastructure in `writ-compiler/tests/lowering_tests.rs` uses `lower_src` / `lower_src_with_errors` helpers and `insta::assert_debug_snapshot!` already; 62 tests already exist covering R3–R13; Phase 6 adds: (1) a lower_fn-focused snapshot test for the `fn`-only pass, (2) a full integration snapshot covering all construct types together, (3) a determinism assertion test for localization keys |
</phase_requirements>

---

## Summary

Phase 6 is a **quality and verification phase**, not an implementation phase. All five passes (`lower_fn`, `lower_dialogue`, `lower_entity`, `lower_operator_impls`, `lower_stmt`/`lower_expr` expression helpers) are fully wired and operational. The `lower/mod.rs` dispatch loop already sequences all passes correctly with documented rationale comments. The 62 existing snapshot tests already cover R3 through R13.

What remains is closing the four open R15 acceptance criteria: (1) verifying the pass ordering documentation in `lower/mod.rs` is present and correct (it is — see the `lower()` doc comment), (2) adding an integration snapshot test that exercises all construct types in a single source program, (3) adding a determinism test that lowers the same source twice and asserts identical localization key output, and (4) confirming every individual pass has at least one snapshot test (most do; `lower_fn` is only exercised indirectly through R3–R7 wrapper tests, not as its own named section).

**Primary recommendation:** This phase needs one plan covering three tasks: a `lower_fn` direct snapshot test (closes R15 criterion 4 gap), an integration snapshot test covering all construct types (criterion 2), and a determinism assertion test (criterion 3). Pass ordering documentation is already in place (criterion 1 is already met).

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `insta` | `1` (with `ron` feature) | Snapshot testing — `assert_debug_snapshot!` for AST output | Already in `[dev-dependencies]`; established project pattern across all 62 existing tests |
| `cargo test` | Rust toolchain | Test runner | Already used; command is `cargo test -p writ-compiler` |
| Rust 2024 edition | workspace | Implementation language | Already established |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `writ_compiler::lower` | workspace | Public `lower()` API + `LoweringError` | All tests go through this public API |
| `writ_parser::parse` | workspace | Parse source strings into CST items | Used by `lower_src` helper already |
| `INSTA_UPDATE=always` env var | insta feature | Accept new snapshots in a single pass | First run of new tests before committing; no separate `cargo insta review` step needed |

### No New Dependencies Needed

Phase 6 requires **zero new `[dependencies]` or `[dev-dependencies]` entries**. The entire test suite runs on the existing stack.

**Verify baseline before starting:**
```bash
cargo test -p writ-compiler
# Expected: 62 passed; 0 failed
```

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `assert_debug_snapshot!` | `assert_ron_snapshot!` | RON requires `Serialize` on all types; `SimpleSpan` lacks it without chumsky serde feature; debug snapshots are the project standard (documented in STATE.md) |
| Single integration test file | Separate integration test binary | No benefit; all tests live in `writ-compiler/tests/lowering_tests.rs` by convention |
| Runtime comparison for determinism | Two separate lowering calls in one test | Two calls in one `#[test]` function compares results directly and avoids flakiness from external state |

---

## Architecture Patterns

### Recommended Project Structure After Phase 6

```
writ-compiler/tests/
└── lowering_tests.rs         # Add R15 integration tests to the existing file (sections at end)

writ-compiler/tests/snapshots/
└── lowering_tests__<new_test_names>.snap  # Created automatically by insta on first run
```

No new source files needed. No changes to `writ-compiler/src/`. This is a test-only phase.

### Pattern 1: Existing Test Helper (Established — Do Not Change)

**What:** `lower_src` and `lower_src_with_errors` parse source then lower it, asserting no parse errors along the way.

**When to use:** All new snapshot tests use these helpers exactly as written.

```rust
// Source: writ-compiler/tests/lowering_tests.rs lines 7–23
fn lower_src(src: &'static str) -> Ast {
    let (items, parse_errors) = writ_parser::parse(src);
    let items = items.expect("parse returned None");
    let error_msgs: Vec<String> = parse_errors.iter().map(|e| format!("{e:?}")).collect();
    assert!(error_msgs.is_empty(), "parse errors: {:?}", error_msgs);
    let (ast, lower_errors) = lower(items);
    assert!(lower_errors.is_empty(), "lowering errors: {:?}", lower_errors);
    ast
}

fn lower_src_with_errors(src: &'static str) -> (Ast, Vec<LoweringError>) {
    let (items, parse_errors) = writ_parser::parse(src);
    let items = items.expect("parse returned None");
    let error_msgs: Vec<String> = parse_errors.iter().map(|e| format!("{e:?}")).collect();
    assert!(error_msgs.is_empty(), "parse errors: {:?}", error_msgs);
    lower(items)
}
```

**CRITICAL — `&'static str` constraint:** The `lower_src` helper takes `&'static str` because the CST borrows from the source string and `Rich<'static, Token<'src>, Span>` errors in the parser force the source to be `'static`. All test source strings must be string literals (not heap-allocated `String`s). Use `r#"..."#` raw string literals for complex multi-line sources.

### Pattern 2: Integration Test — Single Mixed-Construct Source

**What:** One large snapshot test that lowers a Writ program containing all construct types: an `fn` with optional params, a `dlg` with three-tier speaker resolution and choices, an `entity` with components and lifecycle hooks, and an `impl` block with operator overloads.

**When to use:** The R15 integration snapshot (criterion 2). One test, one snapshot, all passes exercised together.

**Design constraints for the source string:**
- Must be `&'static str` (string literal)
- Must parse without errors (all constructs must be syntactically valid)
- Must lower without errors (speakers must be resolvable, keys must not collide)
- Should be minimal but representative — enough to exercise each pass's distinct output

**Sketch of the integration source:**
```rust
#[test]
fn integration_all_constructs() {
    // Exercises: lower_fn (fn with optional param + return),
    //            lower_operator_impls (operator == → Eq impl + derived Ne),
    //            lower_dialogue (dlg with param speaker, singleton speaker, choice, transition),
    //            lower_entity (entity with property, use clause, lifecycle hook, [Singleton])
    let ast = lower_src(r#"
fn describe(name: string?) -> string { return "unknown"; }

impl Score {
    operator ==(other: Score) -> bool { true }
}

dlg greet(player: Entity) {
    @player Hey there.
    @Narrator
    $ choice {
        "Yes" { @player Sure. }
        "No" { @Narrator Okay. }
    }
    -> greet(player)
}

[Singleton]
entity Narrator {
    name: string = "Narrator",
    use Speaker {},
    on create { let ready: bool = true; }
}
"#);
    insta::assert_debug_snapshot!(ast);
}
```

**Note:** The exact source must be validated against the parser before finalizing. The planner should run a quick `cargo test` after writing the test to see if it parses and lowers cleanly, accepting the snapshot with `INSTA_UPDATE=always`.

### Pattern 3: Determinism Test — Two Lowering Runs Compared

**What:** Lowers the same source string twice using two separate calls to `lower()` and asserts that the resulting ASTs are identical and that the localization keys in both outputs are exactly equal.

**When to use:** The R15 determinism requirement (criterion 3). This verifies that FNV-1a keys are not dependent on pointer addresses, allocation order, or any non-deterministic state.

**Implementation approach:**
```rust
#[test]
fn localization_keys_are_deterministic() {
    let src = r#"dlg intro() { @Narrator Hello. @Narrator Goodbye. }"#;

    let (items1, _) = writ_parser::parse(src);
    let (ast1, errors1) = lower(items1.expect("parse 1"));

    let (items2, _) = writ_parser::parse(src);
    let (ast2, errors2) = lower(items2.expect("parse 2"));

    // Both runs must be error-free
    assert!(errors1.is_empty(), "run 1 errors: {:?}", errors1);
    assert!(errors2.is_empty(), "run 2 errors: {:?}", errors2);

    // ASTs must be identical (this implicitly checks keys)
    assert_eq!(format!("{ast1:?}"), format!("{ast2:?}"));
}
```

**Why `format!("{:?}")` comparison:** `Ast`, `AstDecl`, etc. derive `Debug` but not `PartialEq`. The project pattern uses `assert_debug_snapshot!` throughout, so debug output is the authoritative representation. Comparing `Debug` strings is correct for this use case. An alternative is to snapshot both runs separately and compare snapshot files, but the inline approach is simpler and more CI-friendly.

**Alternative approach (also valid):** Use `assert_debug_snapshot!` on the output of both runs with the same snapshot name — insta will compare them to the same stored file. But this adds two snapshots for one test; the inline comparison is cleaner.

### Pattern 4: Lower_fn Direct Coverage Snapshot

**What:** A focused test that exercises `lower_fn` for a plain function with no special sugar — confirming the basic fn lowering path (params, return type, body) is covered by its own named snapshot.

**Why this matters:** All 62 existing tests use `fn` wrappers, but the section headers in `lowering_tests.rs` cover R3–R13. There is no explicit "R2 / lower_fn" section. R15 criterion 4 requires every lowering pass to have snapshot test coverage. While `lower_fn` IS exercised by every other test, adding one explicitly-named test closes the gap unambiguously.

```rust
// =========================================================
// R2 — Pipeline Infrastructure / lower_fn
//
// Basic fn with params and return type → AstFnDecl with correct fields
// =========================================================

/// Basic function with params and return type lowered correctly
#[test]
fn fn_basic_with_params_and_return() {
    let ast = lower_src("fn add(a: int, b: int) -> int { return a; }");
    insta::assert_debug_snapshot!(ast);
}
```

### Pattern 5: Pass Ordering Documentation (Already Present — Verify, Don't Rewrite)

**What:** The `lower()` function in `lower/mod.rs` already has a doc comment explaining pass ordering with rationale. R15 criterion 1 is already satisfied.

**Current state (lines 35–63 of `lower/mod.rs`):** The doc comment on `lower()` explicitly documents:
1. Expression helpers (invoked per-node from inside structural passes, not top-level): `lower_optional`, `lower_fmt_string`, `lower_compound_assign`, `lower_operator`, `lower_concurrency`
2. Structural passes (top-level, process Item variants): `lower_fn`, `lower_dialogue`, `lower_entity`
3. The rationale: "Expression helpers run BEFORE structural passes because structural passes invoke them when they encounter expression/type positions."

**Action for Phase 6:** Read the doc comment and verify it accurately reflects the current implementation. If accurate, no change is needed. If any pass names or ordering has drifted during Phases 4–5, update the comment.

### Anti-Patterns to Avoid

- **Adding new source files:** Phase 6 is test-only. All new code goes in `writ-compiler/tests/lowering_tests.rs`.
- **Breaking the `&'static str` contract:** Never use `String::from(...)` or `format!(...)` as the source argument to `lower_src`. Use raw string literals (`r#"..."#`).
- **Designing an integration source that exercises errors:** The integration snapshot test should be an error-free program — use `lower_src` not `lower_src_with_errors`. Error paths are already covered by individual pass tests.
- **Comparing `Ast` with `==`:** `Ast` does not derive `PartialEq`. Use `assert_debug_snapshot!` or `format!("{:?}")` comparison.
- **Using pointer/address-dependent values in integration source:** Do not use any construct that would make the snapshot non-deterministic (e.g., global mutable state). The FNV-1a keys are deterministic by design; the test simply confirms this.
- **Skipping `INSTA_UPDATE=always` for new snapshots:** First-run snapshots must be accepted. The project pattern (documented in STATE.md) is `INSTA_UPDATE=always cargo test -p writ-compiler` — this accepts all new snapshots in one step.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| AST equality comparison | Custom `PartialEq` impl on `Ast` | `format!("{:?}")` string comparison or `assert_debug_snapshot!` | `SimpleSpan` derives PartialEq but full AST does — the project uses debug representation as the canonical form throughout |
| Multi-pass pipeline sequencing | New orchestrator module | Already implemented in `lower/mod.rs` `lower()` fn | The item dispatch loop already calls all passes in order; no new wiring needed |
| Snapshot acceptance | Manual snapshot files | `INSTA_UPDATE=always cargo test -p writ-compiler` | The project pattern auto-accepts on first run without a separate review step |

**Key insight:** Phase 6 has no novel implementation work. The entire value is in the test coverage that validates what already exists. Resist the urge to refactor or restructure code.

---

## Common Pitfalls

### Pitfall 1: Integration Source Fails to Parse

**What goes wrong:** The integration test source string uses constructs that are syntactically valid in the spec but not yet implemented in the parser. For example, `$ match expr { ... }` in a dlg block, or `?` propagation (`expr?`), may not be parsed correctly.

**Why it happens:** The parser implements a subset of the spec. Some constructs listed in REQUIREMENTS.md are marked `[ ]` (not yet done): `?` propagation, `!` unwrap, `$"{{literal}}"` braces, dialogue text as implicit formattable string.

**How to avoid:** Test each part of the integration source string individually before combining. Run `cargo test -p writ-parser` on constituent pieces. Avoid using unchecked (`[ ]`) requirement items in the integration source. The integration source should use only constructs that are already snapshot-tested individually.

**Constructs to AVOID in integration source (not yet fully verified):**
- `expr?` (R3 `?` propagation — unchecked)
- `expr!` (R3 `!` unwrap — unchecked)
- `$"{{escaped braces}}"` (R4 escaped braces — unchecked by current snapshots, though `fmt_string_escaped_braces` test exists)
- Dialogue text with implicit formattable string (R4 — unchecked; `{expr}` in dialogue text IS tested via `dlg_text_interpolation`)

**Constructs safe to use (all have confirmed snapshot tests):**
- `fn name(param: type?) -> type { ... }` (R3)
- `$"text {expr}"` (R4)
- `x += n` (R5)
- `impl T { operator +(other: T) -> T { ... } }` (R6, including derived ops)
- `spawn/join/cancel/defer` (R7)
- `dlg name(param: Entity) { @param Text. @Narrator\n$ choice { "A" { @Narrator ok. } } -> target }` (R8–R11)
- `[Singleton] entity Name { prop: type, use Component {}, on create { } }` (R12–R13)

### Pitfall 2: Determinism Test Uses `&'static str` Constraint Wrong

**What goes wrong:** The determinism test calls `writ_parser::parse(src)` twice with the same `&'static str`. This is correct. However, if the test uses a runtime `String` variable for `src`, the second parse will fail to compile because `lower_src` (and by extension `parse`) requires the source to be borrowed with a `'static` lifetime via the parser's internal handling.

**Why it happens:** The determinism test bypasses `lower_src` and calls `lower()` directly (to avoid the `assert!(lower_errors.is_empty())` check that hides the second run). It must pass the same `&'static str` literal to both calls.

**How to avoid:**
```rust
// CORRECT: static string literal
let src: &'static str = r#"dlg intro() { @Narrator Hello. }"#;
let (items1, _) = writ_parser::parse(src);
let (items2, _) = writ_parser::parse(src);

// WRONG: string variable
let src = String::from("dlg ...");  // will cause lifetime issues
```

### Pitfall 3: `Ast` Debug Output Contains Span Offsets That Differ Between Runs

**What goes wrong:** If any AST node's span is derived from pointer arithmetic or allocation address (not source byte offsets), two runs would produce different debug output even for identical source.

**Why it happens:** Normally, `SimpleSpan` stores byte offset ranges in the source string. But if a synthetic span uses `SimpleSpan::new(0, 0)` (a tombstone) or some computed value, it might be stable or not. The project invariant says NO tombstones.

**How to avoid:** The determinism test will immediately reveal any non-deterministic span generation by failing when spans differ between runs. This is intentional — the test is designed to catch this. If the test fails, the issue is in span synthesis logic (likely in dialogue.rs FNV-1a key generation or entity.rs synthetic node spans).

**Expected behavior:** All spans are byte offsets into the source string. Two parses of the same source string produce identical byte offsets. All synthetic spans reuse their CST origin span. The test should pass without issue.

### Pitfall 4: Dialogue Test Source — Newline Handling

**What goes wrong:** The dialogue lowering pass distinguishes `SpeakerLine` (speaker + text on the same line) from `SpeakerTag` (speaker alone, followed by text on subsequent lines). The parser is token-based (whitespace filtered as trivia), so "same line" vs "next line" is parsed differently depending on what follows the `@Speaker` token.

**Why it happens:** From STATE.md: "Whitespace is trivia-filtered before parsing — @Speaker followed by newline+text = SpeakerLine not SpeakerTag; SpeakerTag only when @Speaker immediately before a sigil ($, @, ->, })."

**How to avoid:** In the integration source string, use the same patterns already confirmed in existing snapshot tests:
- `@player Hey there.` — SpeakerLine (speaker + text, no newline issue)
- `@Narrator\n$ choice { ... }` — SpeakerTag (speaker before `$` sigil)
- `@Narrator\nHello.\nHow are you?` — SpeakerTag followed by text lines (confirmed by `dlg_speaker_tag_sets_active`)

Use raw string literals (`r#"..."#`) to include actual newlines. Confirm behavior against existing snapshots before writing the integration test.

### Pitfall 5: Integration Source — Choice Arm Speaker Scope

**What goes wrong:** A choice arm's `@Player` speaker leaks into sibling arms, causing unexpected speaker resolution.

**Why it happens:** Speaker scope isolation in choices is only enforced when `@Speaker` appears as a `SpeakerTag` (before a `$` sigil), which pushes the speaker onto the stack with save/restore at choice boundaries. This is subtle.

**How to avoid:** Use the exact pattern from `dlg_choice_speaker_scope_isolation` (which is verified working): outer `@Narrator` as a SpeakerTag (before `$ choice`), then choice arms with inline speaker lines. Copy from that test's source as the dialogue section of the integration test.

---

## Code Examples

### Accepted Snapshot Pattern (from existing tests)

```rust
// Source: writ-compiler/tests/lowering_tests.rs
// Use this exact pattern — assert_debug_snapshot! is the project standard
#[test]
fn fn_basic_with_params_and_return() {
    let ast = lower_src("fn add(a: int, b: int) -> int { return a; }");
    insta::assert_debug_snapshot!(ast);
}
```

### Snapshot Acceptance Command (from STATE.md)

```bash
# Accept new snapshots in one step (project standard)
INSTA_UPDATE=always cargo test -p writ-compiler

# Then verify all snapshots are stable (second run, no INSTA_UPDATE)
cargo test -p writ-compiler
```

### Determinism Test Pattern

```rust
use writ_compiler::{lower, Ast, LoweringError};

#[test]
fn localization_keys_are_deterministic() {
    let src: &'static str = r#"dlg intro() {
        @Narrator Hello.
        @Narrator Goodbye.
        @Narrator Welcome. #welcome
    }"#;

    let (items1, parse_errs1) = writ_parser::parse(src);
    assert!(parse_errs1.is_empty(), "parse 1 errors: {:?}", parse_errs1);
    let (ast1, errors1) = lower(items1.expect("parse 1 returned None"));
    assert!(errors1.is_empty(), "run 1 errors: {:?}", errors1);

    let (items2, parse_errs2) = writ_parser::parse(src);
    assert!(parse_errs2.is_empty(), "parse 2 errors: {:?}", parse_errs2);
    let (ast2, errors2) = lower(items2.expect("parse 2 returned None"));
    assert!(errors2.is_empty(), "run 2 errors: {:?}", errors2);

    // FNV-1a keys must be identical across runs
    assert_eq!(
        format!("{ast1:?}"),
        format!("{ast2:?}"),
        "Two lowering runs of identical source produced different output"
    );
}
```

### Integration Snapshot — Validated Safe Constructs

```rust
// =========================================================
// R15 — Integration Snapshot
//
// Full program: fn + operator impl + dlg + entity
// All passes exercised in one test
// =========================================================

#[test]
fn integration_all_constructs() {
    let ast = lower_src(r#"fn describe(name: string?) -> string { return "unknown"; }

impl Score {
    operator ==(other: Score) -> bool { true }
}

dlg greet(player: Entity) {
    @player Hey there.
    @Narrator
    $ choice {
        "Yes" { @player Sure. }
        "No" { @Narrator Okay. }
    }
    -> greet(player)
}

[Singleton]
entity Narrator {
    name: string = "Narrator",
    use Speaker {},
    on create { let ready: bool = true; }
}"#);
    insta::assert_debug_snapshot!(ast);
}
```

**Note:** This source string must be confirmed parseable by running it through the parser before committing. The planner should include a task to verify it parses, then accept the snapshot.

### Pass Ordering Doc Comment (current state — already correct)

```rust
// Source: writ-compiler/src/lower/mod.rs lines 35–63
// This comment is already present and correctly documents pass ordering.
// Criterion 1 of R15 is already met.

/// # Pass Ordering
///
/// Passes execute in this order (rationale: each pass's output is required
/// by subsequent passes):
///
/// 1. **Expression helpers** (invoked from inside structural passes, not top-level):
///    - `lower_optional` — `T?` → `Option<T>`, `null` → `Option::None`
///    - `lower_fmt_string` — `$"..."` → string concatenation
///    - `lower_compound_assign` — `a += b` → `a = a + b`
///    - `lower_operator` — operator decls → contract impl methods
///    - `lower_concurrency` — spawn/join/cancel/defer/detached pass-through
///
/// 2. **Structural passes** (top-level, process Item variants):
///    - `lower_fn` — Fn items
///    - `lower_dialogue` — Dlg items → Fn decls
///    - `lower_entity` — Entity items → Struct + Impl + lifecycle registrations
```

---

## Current Coverage Gap Analysis

This is the key planning input: which R15 criteria are already met and which are open.

### Criterion 1: Pass Ordering in `lower/mod.rs`
**Status: ALREADY MET**
The doc comment on `lower()` already documents all passes with ordering rationale. No action needed beyond verification.

### Criterion 2: Integration Snapshot Test
**Status: MISSING — needs new test**
No test currently lowers a program with all construct types together. The `entity_full_declaration` test is the largest single-construct test. An integration test combining `fn` + `impl` with operators + `dlg` + `entity` does not exist.

### Criterion 3: Determinism Test
**Status: MISSING — needs new test**
No test currently calls `lower()` twice on the same source. The FNV-1a key algorithm is deterministic by design (uses byte-offset spans and string content, not pointers), but this is not proven by any test.

### Criterion 4: Every Pass Has Snapshot Coverage
**Status: PARTIALLY MET**
Coverage by pass:
| Pass | Tests | Named Section | Gap |
|------|-------|--------------|-----|
| `lower_fn` | Exercised by ALL 62 tests (every test wraps source in `fn` or uses `dlg`/`entity`/`impl`) | No dedicated "R2" section | LOW RISK: technically zero named `lower_fn` tests, but it IS exercised. Add one named test for clarity. |
| `lower_type` (optional.rs) | R3 section: 4 tests | Yes (R3) | NONE |
| `lower_fmt_string` | R4 section: 4 tests | Yes (R4) | NONE |
| `lower_compound_assign` (in expr.rs) | R5 section: 6 tests | Yes (R5) | NONE |
| `lower_operator_impls` | R6 section: 10 tests | Yes (R6) | NONE |
| Concurrency pass-through | R7 section: 5 tests | Yes (R7) | NONE |
| `lower_dialogue` | R8–R11 sections: 19 tests | Yes (R8–R11) | NONE |
| `lower_entity` | R12–R13 + error sections: 14 tests | Yes (R12–R13) | NONE |
| `lower_stmt` / `lower_expr` | Exercised by all statement/expression tests | No dedicated section | LOW RISK: heavily exercised indirectly |
| Pass-through lowers (struct/enum/contract/component/extern/const/global/namespace/using) | None | None | MEDIUM: these lowers in `lower/mod.rs` have zero snapshot coverage |

**Pass-through lowers gap:** Functions `lower_struct`, `lower_enum`, `lower_contract`, `lower_component`, `lower_extern`, `lower_const`, `lower_global`, `lower_namespace`, `lower_using` in `lower/mod.rs` currently have no snapshot tests. These are mechanical pass-throughs with minimal logic, but R15 criterion 4 says "no lowering pass has zero test coverage." The planner must decide whether to add minimal snapshot tests for each pass-through, or interpret "pass" as the five major passes (fn, dialogue, entity, operator, expression helpers). The success criteria text says "every individual lowering pass" — the pass-through functions are not separate named passes in the documentation, they are helpers within the `lower()` dispatch loop.

**Recommendation:** The integration snapshot test (criterion 2) should include at least one struct and one enum in the source to exercise `lower_struct` and `lower_enum` indirectly. This closes the gap pragmatically. Alternatively, add a dedicated `struct_passthrough`, `enum_passthrough`, etc. section — but this is lower priority given the purely mechanical nature of these functions.

---

## Open Questions

1. **Do pass-through item lowers (struct, enum, contract, component, extern, const, global, namespace, using) each need their own snapshot test?**
   - What we know: R15 says "no lowering pass has zero test coverage." These lowers in `lower/mod.rs` are mechanical pass-throughs with almost no logic. The existing 62 tests do not exercise them (all tests use `fn`, `impl`, `dlg`, or `entity` as top-level items).
   - What's unclear: Whether the R15 criterion means every function in the lower module, or only the five major desugaring passes.
   - Recommendation: The planner should add a `struct_passthrough` and `enum_passthrough` test at minimum (two lines each in `lowering_tests.rs`). For `contract`, `component`, `extern`, `const`, `global`, `namespace`, and `using`, decide based on time budget. A single combined "pass-through items" snapshot test could cover all of them in one source string.

2. **Should the integration snapshot test include unchecked R3 items (`?` propagation, `!` unwrap)?**
   - What we know: R3 has two unchecked items (`[ ]`): `?` propagation and `!` unwrap. These are not yet implemented in the lowering pipeline. Including them in the integration source would cause a test failure.
   - What's unclear: Whether Phase 6 is expected to implement these R3 gaps or just test what already works.
   - Recommendation: Do NOT include `?` propagation or `!` unwrap in the integration source. Phase 6's scope is R15 (snapshot testing), not R3 implementation. The unchecked R3 items are out of scope unless specifically added to Phase 6's mandate.

3. **What is the expected scope of the integration source — minimal or comprehensive?**
   - What we know: R15 says "fn with optional params, dlg with three-tier speaker resolution and choices, entity with components and lifecycle hooks, operator overloads."
   - What's unclear: Whether "three-tier speaker resolution" means the integration test must exercise all three speaker resolution tiers (Tier 1: param, Tier 2: singleton, and the error Tier 3: unknown speaker). Tier 3 produces errors, which conflicts with using `lower_src` (which asserts no errors).
   - Recommendation: The integration test uses `lower_src` (no errors). Include Tier 1 (param speaker) and Tier 2 (singleton, e.g., `@Narrator` without a param). Tier 3 is already covered by `dlg_text_without_speaker_error` test. The integration test does not need to replicate error cases.

---

## What Exists vs What Needs Building

| Item | Status | Action |
|------|--------|--------|
| `lower()` pass ordering doc comment | EXISTS in `lower/mod.rs` lines 35–63 | Verify accuracy — no rewrite needed |
| `lower_src` / `lower_src_with_errors` helpers | EXISTS | Use as-is |
| R3–R13 snapshot tests (62 total) | EXISTS | No changes needed |
| Integration snapshot test (all constructs) | MISSING | Add to `lowering_tests.rs` |
| Determinism test (two runs, same output) | MISSING | Add to `lowering_tests.rs` |
| `lower_fn` direct snapshot test | MISSING (soft gap) | Add one `fn_basic_with_params_and_return` test |
| Pass-through item snapshot tests | MISSING | Add at minimum `struct_passthrough` + `enum_passthrough`; consider adding all 8 pass-throughs in one combined test |
| New snapshots in `tests/snapshots/` | MISSING | Created automatically by insta on first run with `INSTA_UPDATE=always` |

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual `assert_eq!` with hand-written expected values | `insta::assert_debug_snapshot!` auto-captures AST output | Phase 2 (2026-02-26) | Snapshots are ground truth; changes to AST structure are caught automatically on next test run |
| `cargo insta review` for snapshot acceptance | `INSTA_UPDATE=always cargo test -p writ-compiler` | STATE.md decision | Single-step acceptance; no interactive review needed |

**Not deprecated:** Nothing in the test infrastructure is deprecated. The `ron` feature in insta is present but unused (all tests use `assert_debug_snapshot!`, not `assert_ron_snapshot!`). It does no harm.

---

## Sources

### Primary (HIGH confidence)

- `D:/dev/git/Writ/writ-compiler/tests/lowering_tests.rs` — Full file read; all 62 existing test functions and their source strings confirmed; `lower_src` and `lower_src_with_errors` helper signatures confirmed
- `D:/dev/git/Writ/writ-compiler/src/lower/mod.rs` — Full file read; pass ordering doc comment confirmed at lines 35–63; all pass-through lower functions confirmed (lower_struct, lower_enum, lower_contract, lower_component, lower_extern, lower_const, lower_global, lower_namespace, lower_using); all five major passes wired
- `D:/dev/git/Writ/writ-compiler/Cargo.toml` — `insta = { version = "1", features = ["ron"] }` confirmed in dev-dependencies; no new deps needed
- `D:/dev/git/Writ/.planning/REQUIREMENTS.md` — R15 acceptance criteria confirmed; four unchecked `[ ]` items confirmed (two in R3, two in R4, four in R15)
- `D:/dev/git/Writ/.planning/STATE.md` — Key decisions confirmed: `INSTA_UPDATE=always` for snapshot acceptance; `assert_debug_snapshot` over RON; `lower_src` takes `&'static str`; FNV-1a key algorithm is deterministic
- `D:/dev/git/Writ/.planning/ROADMAP.md` — Phase 6 success criteria confirmed verbatim
- `D:/dev/git/Writ/.planning/phases/05-entity-lowering/05-VERIFICATION.md` — Baseline confirmed: 62 compiler tests pass, 177 parser tests pass, zero regressions

### Secondary (MEDIUM confidence)

- `D:/dev/git/Writ/.planning/phases/03-operator-and-concurrency-lowering/03-RESEARCH.md` — Established patterns for `insta` usage, `assert_debug_snapshot!`, and `lower_src` helper — all confirmed still accurate
- `D:/dev/git/Writ/.planning/phases/05-entity-lowering/05-RESEARCH.md` — Confirmed `AstExpr::StructLit` exists; confirmed `$ComponentName` naming; confirmed emission order decisions — relevant to integration test design

### Tertiary (LOW confidence)

- None — all findings sourced from live code, not web search.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already present; test infrastructure already established; no new libraries needed
- Architecture patterns: HIGH — all patterns sourced from confirmed working tests; helper signatures confirmed from source
- Pitfalls: HIGH — all pitfalls sourced from actual codebase inspection (STATE.md decisions, REQUIREMENTS.md unchecked items, parser behavior confirmed in dialogue phase snapshots)

**Research date:** 2026-02-27
**Valid until:** Until any changes to `lower/mod.rs` pass ordering or `AstExpr`/`AstStmt` variants — the snapshot infrastructure is stable indefinitely
