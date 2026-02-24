# Phase 4: Dialogue Lowering and Localization - Research

**Researched:** 2026-02-26
**Domain:** Compiler lowering pass — CST `DlgDecl` → `AstFnDecl`, speaker resolution, localization key generation, dialogue validation
**Confidence:** HIGH — all findings drawn directly from the codebase, language spec, and existing implementation patterns

---

<phase_requirements>
## Phase Requirements

| ID  | Description | Research Support |
|-----|-------------|-----------------|
| R8  | Desugar `dlg` declarations to `fn` declarations per spec §28.1–§28.2: speaker resolution (3-tier), say/choice/transition semantics, all `$` escape forms | CST `DlgDecl` and all `DlgLine` variants are fully defined in cst.rs; `AstFnDecl` target type exists; `lower_fn` pattern directly applicable; stub `todo!()` sites identified in mod.rs and stmt.rs |
| R9  | Generate localization keys for dialogue text via FNV-1a 32-bit hash of `namespace + method + speaker + content + occurrence_index`; manual `#key` overrides replace auto-keys; `say()` → `say_localized(speaker, key, fallback)` | Full algorithm specified in spec §25.2; `LoweringContext::next_loc_key()` counter already present as a placeholder; `loc_key` field already on `DlgLine::SpeakerLine`, `TextLine`, and `DlgChoiceArm` in cst.rs |
| R10 | Detect duplicate `#key` values within a `dlg` block and emit `LoweringError::DuplicateLocKey` (already defined in error.rs) | `LoweringError::DuplicateLocKey { key, first_span, second_span }` already defined; only the detection logic in the dialogue pass needs implementing |
| R11 | Enforce that `->` transitions are the last statement in their block; `->` before end → `LoweringError::NonTerminalTransition` (already defined in error.rs) | `LoweringError::NonTerminalTransition { span }` already defined; two stub `todo!()` sites in stmt.rs (`Stmt::Transition`) need implementing |
</phase_requirements>

---

## Summary

Phase 4 implements the most semantically complex lowering pass: transforming `DlgDecl` CST nodes into `AstFnDecl` AST nodes. The infrastructure is already in place — `LoweringContext` carries `speaker_stack`, `loc_key_counter`, and the error accumulator; the `LoweringError` enum already has all four dialogue-specific error variants pre-defined (`UnknownSpeaker`, `NonTerminalTransition`, `DuplicateLocKey`); and the CST `DlgDecl`, `DlgLine`, `DlgChoice`, `DlgIf`, `DlgMatch`, `DlgTransition`, and `DlgTextSegment` types are fully defined in cst.rs with no unknowns. The AST target types (`AstFnDecl`, `AstStmt::Return`, `AstStmt::Expr`) are also complete.

The main implementation work is: (1) create `writ-compiler/src/lower/dialogue.rs` with `lower_dialogue()` implementing the recursive `DlgLine` fold, speaker resolution with three-tier lookup, and the localization sub-pass; (2) implement the FNV-1a 32-bit key computation as a pure function; (3) wire the new module into `lower/mod.rs` at the two `todo!("Phase 4")` call sites; (4) handle the two `todo!("Phase 4")` stubs in `lower/stmt.rs` for `Stmt::DlgDecl` and `Stmt::Transition`; and (5) write snapshot tests covering all five success criteria.

The most tricky algorithmic problem is speaker scoping in `$ choice` branches — each arm must push a fresh speaker scope, so an `@Speaker` tag in one branch does not propagate to sibling branches. The spec example in §28.2 confirms that `Entity.getOrCreate<T>()` calls are hoisted to `let` bindings at the top of the function body (not inline in each `say_localized()` call), which constrains the code generation structure. The FNV-1a key computation input string must track `occurrence_index` per `(namespace, method, speaker, content)` tuple — requiring a `HashMap<(String, String, String, String), u32>` deduplication counter threaded through the dialogue lowering context.

**Primary recommendation:** Create `lower/dialogue.rs` following the same module pattern as `lower/operator.rs` — a single public entry point `lower_dialogue(dlg: DlgDecl, dlg_span: SimpleSpan, ctx: &mut LoweringContext) -> AstFnDecl` with all helpers private. Implement FNV-1a as an inline `fn fnv1a_32(input: &str) -> String` returning 8-char lowercase hex.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `chumsky::span::SimpleSpan` | 0.12.0 (already in Cargo.toml) | Span type for all AST nodes | Already used by every module; required for span preservation |
| `thiserror` | 2.0 (already in Cargo.toml) | `LoweringError` variants | Already used; `LoweringError::DuplicateLocKey` and `NonTerminalTransition` already defined here |
| `std::collections::HashMap` | std | Deduplication index tracking per `(namespace, method, speaker, content)` → `occurrence_index` counter | Needed for the `occurrence_index` component of FNV-1a key |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `insta` | 1 (already in dev-deps) | Snapshot testing | Phase 4 test plan; all tests use `insta::assert_debug_snapshot!` per established pattern |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Inline FNV-1a (10-line implementation) | `fnv` crate | The algorithm is 10 lines; adding a new crate adds dependency overhead; the spec mandates the exact algorithm already — inline is correct |
| `HashMap` for dedup counter | `BTreeMap` | `HashMap` is sufficient; ordering does not matter for correctness; `HashMap` has better average-case performance |

**Installation:** No new dependencies required. All needed libraries are already present in `writ-compiler/Cargo.toml`.

---

## Architecture Patterns

### Recommended Project Structure

```
writ-compiler/src/lower/
├── mod.rs           # Entry point — wire Item::Dlg → lower_dialogue (remove todo!)
├── context.rs       # LoweringContext — already has speaker_stack and loc_key_counter
├── error.rs         # LoweringError — already has all dialogue error variants
├── dialogue.rs      # NEW: lower_dialogue(), lower_dlg_lines(), speaker resolution, FNV-1a
├── operator.rs      # Existing: operator desugaring (reference pattern)
├── fmt_string.rs    # Existing: used by dialogue text lowering (already handles {expr})
├── stmt.rs          # Fix two todo!() stubs: Stmt::DlgDecl, Stmt::Transition
├── expr.rs          # Existing: lower_expr() used for code escapes
└── optional.rs      # Existing: lower_type() used for params
```

### Pattern 1: Module Entry Point Matching operator.rs

**What:** Single pub function `lower_dialogue` is the only public export. All helpers (`lower_dlg_lines`, `lower_dlg_line`, `lower_speaker_line`, `lower_choice`, `lower_transition`, `compute_loc_key`) are private (`fn`, not `pub fn`).

**When to use:** Exactly as done in `operator.rs` — the orchestrator in `mod.rs` calls `lower_dialogue`, not any internal helpers.

**Example:**
```rust
// writ-compiler/src/lower/dialogue.rs
use writ_parser::cst::{DlgDecl, DlgLine, DlgTextSegment, DlgTransition, Spanned};
use crate::ast::decl::AstFnDecl;
use crate::lower::context::LoweringContext;

pub fn lower_dialogue(
    dlg: DlgDecl<'_>,
    dlg_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstFnDecl {
    // 1. Lower params (reuse lower_param from super::)
    // 2. Collect all unique speaker names → hoist getOrCreate let-bindings
    // 3. lower_dlg_lines(&dlg.body, dlg.name.0, ctx) → Vec<AstStmt>
    // 4. Combine hoisted lets + lowered body
    // 5. Return AstFnDecl { ... span: dlg_span }
    todo!()
}
```

### Pattern 2: Wire in mod.rs

**What:** Two `todo!("Phase 4")` call sites in `lower/mod.rs` must be replaced with `lower_dialogue` calls. The pattern is identical to how `lower_operator_impls` is wired:

```rust
// In lower/mod.rs:
Item::Dlg((dlg_decl, dlg_span)) => {
    decls.push(AstDecl::Fn(lower_dialogue(dlg_decl, dlg_span, &mut ctx)));
}
```

Note: Two sites must be updated — `lower()` and `lower_namespace()` both have `Item::Dlg` arms with `todo!()`.

### Pattern 3: Speaker Resolution (Three-Tier Lookup)

**What:** For each `@Speaker` reference in the CST, perform a three-tier lookup in order:

1. **Tier 1 — Local param/variable:** Check if `speaker_name` matches any `dlg` parameter name. If yes, emit `AstExpr::Ident { name: speaker_name }` directly.
2. **Tier 2 — Singleton entity:** If not a param, assume it is a `[Singleton]` entity type and emit `Entity.getOrCreate<SpeakerName>()` as a let-binding hoisted to the function top. Spec §13.2, §28.2 confirm this.
3. **Tier 3 — Unknown:** Emit `LoweringError::UnknownSpeaker { name, span }` via `ctx.emit_error()` and continue lowering the rest of the block (do not halt).

**Critical detail from spec §28.2:** The lowered output hoists ALL `getOrCreate` calls as `let _narrator = Entity.getOrCreate<Narrator>()` bindings at the TOP of the generated function — they are NOT repeated inline at each `say_localized()` call site. This means the lowering pass must do a pre-scan of the block to discover all singleton speakers, then emit the let-bindings before the body statements.

**Example (from spec §28.2):**
```rust
// Source:
// dlg greetPlayer(name: string) {
//     @Narrator Hey, {name}.
//     @Narrator
//     How are you?
//     ...
// }

// Lowered:
// fn greetPlayer(name: string) {
//     let _narrator = Entity.getOrCreate<Narrator>();  // hoisted
//     let _player = Entity.getOrCreate<Player>();      // hoisted
//     say_localized(_narrator, "a3f7c012", "Hey, " + name.into<string>() + ".");
//     say_localized(_narrator, "b92e1d44", "How are you?");
//     ...
// }
```

**Implementation approach for hoisting:** Collect `(speaker_name, speaker_span)` pairs during a first pass over the body, deduplicate by name, then emit `let _lowercase_name = Entity.getOrCreate<SpeakerName>()` bindings before the lowered body statements.

### Pattern 4: Active Speaker Stack for `$ choice` Branch Scoping

**What:** `@Speaker` standalone (no text on same line) sets the active speaker for subsequent lines. Each `$ choice` arm creates a new scope — the active speaker inside one arm does not leak into sibling arms.

**When to use:** On entering a `$ choice` arm body, save the current `ctx.current_speaker()` state, process the arm body, then restore the saved state (pop back to the saved speaker or clear it). This is exactly the purpose of `LoweringContext::push_speaker()` / `pop_speaker()` already defined in `context.rs`.

**Implementation:**
```rust
// On entering a choice arm:
let saved_speaker = ctx.current_speaker().cloned(); // snapshot
// process arm body → Vec<AstStmt>
// after arm: restore
while ctx.current_speaker().is_some() {
    ctx.pop_speaker();
}
if let Some(s) = saved_speaker {
    ctx.push_speaker(s);
}
```

A cleaner approach: track the speaker stack depth before the arm, then drain back to that depth after. This handles nested scopes correctly.

### Pattern 5: FNV-1a 32-bit Key Computation

**What:** Per spec §25.2.1, the localization key is an 8-char lowercase hex string computed as FNV-1a 32-bit over `namespace + "\0" + method + "\0" + speaker + "\0" + content + "\0" + occurrence_index`.

**Algorithm (from spec §25.2.2):**
```rust
fn fnv1a_32(input: &str) -> String {
    const OFFSET_BASIS: u32 = 0x811c9dc5;
    const PRIME: u32 = 0x01000193;
    let mut hash: u32 = OFFSET_BASIS;
    for byte in input.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{:08x}", hash)
}
```

**Deduplication index:** Track a `HashMap<(String, String, String, String), u32>` where the key is `(namespace, method, speaker, content)` and the value is the next `occurrence_index` for that tuple. For each line, look up the tuple, use the current count as `occurrence_index`, then increment the count.

**Manual `#key` override:** When `DlgLine::SpeakerLine { loc_key: Some((key, key_span)), .. }` or `TextLine { loc_key: Some(..) }` is present, use that string directly as the key instead of computing FNV-1a. Track all manual keys in a `HashMap<String, SimpleSpan>` within the dialogue pass; if a duplicate is found, emit `LoweringError::DuplicateLocKey`.

**`loc_key_counter` in LoweringContext:** The existing `ctx.next_loc_key()` counter is a simple incrementing u32 — this was a placeholder for Phase 4. The actual implementation should use the FNV-1a computation described above. The `loc_key_counter` remains useful as a fallback occurrence index or can be repurposed.

### Pattern 6: `DlgLine` Recursive Fold

**What:** The central recursive function processes a `Vec<Spanned<DlgLine<'_>>>` and returns `Vec<AstStmt>`. Each `DlgLine` variant maps to specific output:

| CST DlgLine variant | Lowered AstStmt(s) |
|---------------------|-------------------|
| `SpeakerLine { speaker, text, loc_key }` | `AstStmt::Expr { expr: AstExpr::Call (say_localized) }` — speaker resolved by three-tier lookup |
| `SpeakerTag(speaker)` | No statement emitted; `ctx.push_speaker(SpeakerScope { name, span })` side effect |
| `TextLine { text, loc_key }` | `AstStmt::Expr { expr: AstExpr::Call (say_localized) }` using `ctx.current_speaker()` |
| `CodeEscape(DlgEscape::Statement(s))` | `lower_stmt(s, ctx)` — already fully implemented |
| `CodeEscape(DlgEscape::Block(stmts))` | `stmts.map(lower_stmt)` — inline the block |
| `Choice(DlgChoice)` | `AstStmt::Expr { expr: AstExpr::Call { callee: "choice", args: [array of Option(...) lambdas] } }` |
| `If(DlgIf)` | `AstStmt::Expr { expr: AstExpr::If { condition: lower_expr(cond), then_block: lower_dlg_lines(then), else_block: ... } }` |
| `Match(DlgMatch)` | `AstStmt::Expr { expr: AstExpr::Match { scrutinee: lower_expr(expr), arms: [...] } }` |
| `Transition(DlgTransition)` | `AstStmt::Return { value: Some(AstExpr::Call { callee: target, args }) }` + non-terminal validation |

### Pattern 7: Dialogue Text Lowering (DlgTextSegment → AstExpr)

**What:** `DlgTextSegment` is parallel to `StringSegment` — `Text(&str)` is literal text, `Expr(Box<Spanned<Expr>>)` is interpolation. Process identically to `lower_fmt_string` from `fmt_string.rs`. The `lower_fmt_string` function can NOT be called directly because it takes `Vec<Spanned<StringSegment>>` not `Vec<Spanned<DlgTextSegment>>` — but the logic is the same.

**Implementation:** In `dialogue.rs`, write `lower_dlg_text(segments: Vec<Spanned<DlgTextSegment<'_>>>, outer_span: SimpleSpan, ctx: &mut LoweringContext) -> AstExpr` following the same left-associative fold pattern as `lower_fmt_string`. The raw text (without `{expr}` interpolations) is used as the `fallback` argument to `say_localized()`.

**Key question:** What is the `fallback` text for `say_localized`? Per spec §28.4, the fallback is the full interpolated text (already lowered to a concatenation expression). The `content` for the FNV-1a hash is the **raw text with interpolation slots preserved literally** (e.g., `"Hey, {name}."` per spec §25.2.1).

### Pattern 8: `-> Transition` Lowering and Validation

**What:** `DlgLine::Transition` and `Stmt::Transition` both lower to `AstStmt::Return`. The validation rule: if a `Transition` is followed by more statements in the same block, emit `LoweringError::NonTerminalTransition { span }`.

**Implementation:** When processing `Vec<Spanned<DlgLine>>`, enumerate with index. If a `DlgLine::Transition` appears at index `i < len - 1` (not the last element), emit `NonTerminalTransition` and continue lowering. Also lower `Stmt::Transition` in `lower_stmt()` for the case where `->` appears inside a code-escaped block.

**Lowering rule:**
```rust
// -> target        → AstStmt::Return { value: Some(AstExpr::Call { callee: Ident("target"), args: [] }) }
// -> target(args)  → AstStmt::Return { value: Some(AstExpr::Call { callee: Ident("target"), args: lower_args(...) }) }
```

### Pattern 9: `$ choice` Lowering

**What:** Per spec §28.1, `$ choice { ... }` lowers to `choice([Option("label", fn() { ... }), ...])`. Each arm becomes a lambda (closure with no parameters) wrapping the arm's dialogue body.

**Example output (from spec §28.2):**
```
choice([
    Option("Good!", fn() {
        reputation += 1;
        say_localized(_narrator, "...", "Glad to hear it.");
    }),
    Option("Not great", fn() {
        say_localized(_player, "...", "Things are rough.");
        say_localized(_narrator, "...", "Sorry to hear that.");
    }),
]);
```

**AstExpr construction:**
```rust
AstExpr::Call {
    callee: Box::new(AstExpr::Ident { name: "choice".to_string(), span }),
    args: vec![AstArg {
        name: None,
        value: AstExpr::ArrayLit {
            elements: arms.map(|arm| AstExpr::Call {
                callee: Box::new(AstExpr::Ident { name: "Option".to_string(), span }),
                args: vec![
                    AstArg { value: AstExpr::StringLit { value: arm.label, span }, .. },
                    AstArg { value: AstExpr::Lambda { params: [], body: lower_dlg_lines(arm.body), span }, .. },
                ],
                span,
            }),
        },
        span,
    }],
    span,
}
```

**Choice arm speaker scoping:** Save the current speaker stack depth before entering each arm, restore it after. Each `$ choice` arm is an independent scope.

**Choice label `#key`:** Same collision detection as dialogue line keys. `DlgChoiceArm::loc_key` field in cst.rs is already typed as `Option<Spanned<&str>>`.

### Pattern 10: Stmt::DlgDecl and Stmt::Transition

**What:** `stmt.rs` has two `todo!()` stubs:
- `Stmt::DlgDecl(spanned_dlg)` — a `dlg` declared inline inside a function body. Lowers to `AstDecl::Fn` (but since `lower_stmt` returns `AstStmt`, this is unusual). Looking at the spec and CST, this is a statement-position dialogue — it should produce a nested fn declaration. However, `lower_stmt` returns `AstStmt`, not `AstDecl`. **Resolution:** Check if the planner should lower `Stmt::DlgDecl` to an `AstStmt::Expr` containing a block or treat it as an error. The CST includes it as a stmt variant, so it must be handled. The most likely correct lowering is to emit a synthetic `let name = fn() { ... }` binding — but the spec does not explicitly say. This is an **open question** (see below).
- `Stmt::Transition(spanned_transition)` — `->` appearing inside a code block (inside a `$` escape). Lowers to `AstStmt::Return` with the transition target call.

### Anti-Patterns to Avoid

- **Emitting `SimpleSpan::new(0, 0)` on synthetic nodes:** Every generated `let _narrator = Entity.getOrCreate<Narrator>()` binding must use the originating `@Narrator` span, not a tombstone span. This is a hard constraint from R14.
- **Halting lowering on error:** `ctx.emit_error()` accumulates without halting; the lowering of the rest of the block must continue. For unknown speakers, emit `AstExpr::Error { span }` as a placeholder in the `say_localized()` call site.
- **Leaking speaker state across choice branches:** Push/pop the speaker stack at choice arm boundaries. Failure to do this is the most likely scoping bug.
- **Re-computing occurrence_index globally instead of per-dlg:** The occurrence index resets for each `dlg` block — it is not global across all `dlg` declarations.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| FNV-1a hash | Custom hash algorithm | Inline 10-line implementation per spec §25.2.2 | The spec mandates the exact algorithm; using any crate risks spec drift; the implementation is trivial |
| `say_localized` call construction | Custom `AstExpr` builder DSL | Direct `AstExpr::Call { ... }` construction | Same pattern used in operator.rs for contract impls; no abstraction needed at this scale |
| Speaker scope stack | Custom data structure | `LoweringContext::push_speaker()/pop_speaker()` already implemented | The context already has the right data structure; do not bypass it |

**Key insight:** The spec is unusually precise about the FNV-1a algorithm and its inputs (§25.2.1, §25.2.2). Hand-rolling matches the spec exactly; using an off-the-shelf crate risks subtle mismatches in the input string format that would break localization key stability.

---

## Common Pitfalls

### Pitfall 1: Speaker Hoisting Order

**What goes wrong:** Emitting `let _narrator = Entity.getOrCreate<Narrator>()` inline at each `say_localized()` call instead of hoisting to function top.
**Why it happens:** A naive recursive fold emits everything inline.
**How to avoid:** Pre-scan the `DlgLine` list recursively to collect all singleton speakers that appear anywhere in the body (including inside `$ if`, `$ match`, and `$ choice` branches). Emit the `let` bindings BEFORE the lowered body.
**Warning signs:** Snapshot output shows `getOrCreate` inside a `say_localized` call rather than as a `let` at the function top.

### Pitfall 2: Choice Arm Speaker Scope Leak

**What goes wrong:** `@Speaker` in one choice arm affects sibling arms.
**Why it happens:** Forgetting to snapshot and restore the speaker stack depth at arm boundaries.
**How to avoid:** Before `lower_dlg_lines(arm.body, ...)`, record `let depth_before = ctx.speaker_stack.len()`. After: drain stack back to `depth_before` using `ctx.pop_speaker()`.
**Warning signs:** Snapshot shows second choice arm using speaker from first arm even though no `@Speaker` tag appears in the second arm.

### Pitfall 3: FNV-1a Input String Construction

**What goes wrong:** Missing the null byte separators (`\0`) between fields, or including extra fields, producing wrong keys.
**Why it happens:** The spec format `namespace + "\0" + method + "\0" + speaker + "\0" + content + "\0" + occurrence_index` has 4 null-byte separators; easy to miscopy.
**How to avoid:** Write a single `format!("{}\0{}\0{}\0{}\0{}", namespace, method, speaker, content, occurrence_index)` string and hash the entire thing. Add a determinism test that computes the same key twice from identical inputs and asserts they match.
**Warning signs:** Two compiler runs on identical source produce different keys; the snapshot for the key does not match the reference value computed by hand.

### Pitfall 4: `loc_key_counter` vs FNV-1a

**What goes wrong:** Using the simple `ctx.next_loc_key()` counter as the actual localization key instead of the FNV-1a hash.
**Why it happens:** The counter exists in `LoweringContext` and returns a `u32` — easy to confuse with the key.
**How to avoid:** The `loc_key_counter` in LoweringContext is a placeholder/counter for occurrence tracking. The actual emitted key must be the 8-char hex FNV-1a string. Keep the counter for occurrence_index tracking but do NOT emit it directly as the key value.
**Warning signs:** Snapshot shows keys like `"0"`, `"1"`, `"2"` instead of 8-char hex strings like `"a3f7c012"`.

### Pitfall 5: `DlgTextSegment` vs `StringSegment` Type Mismatch

**What goes wrong:** Attempting to call `lower_fmt_string(dlg_text_segments, ...)` directly — it only accepts `Vec<Spanned<StringSegment>>`.
**Why it happens:** `DlgTextSegment` and `StringSegment` are structurally identical but distinct types.
**How to avoid:** Write `lower_dlg_text()` in `dialogue.rs` that mirrors `lower_fmt_string` but accepts `Vec<Spanned<DlgTextSegment>>`. Do not attempt to convert between the types.
**Warning signs:** Compilation error: type mismatch between `DlgTextSegment` and `StringSegment`.

### Pitfall 6: `TextLine` Without Active Speaker

**What goes wrong:** A `DlgLine::TextLine` appears without any prior `@Speaker` having set the active speaker — `ctx.current_speaker()` returns `None`.
**Why it happens:** Dialogue blocks can technically contain text lines before any speaker attribution. The spec does not explicitly define the error behavior here.
**How to avoid:** Treat `None` active speaker for a `TextLine` as an `LoweringError::UnknownSpeaker` with an empty speaker name and the line's span. Continue lowering with `AstExpr::Error` as the speaker placeholder.
**Warning signs:** Panic or unwrap failure on `ctx.current_speaker().unwrap()`.

### Pitfall 7: Non-Terminal Transition Detection Scope

**What goes wrong:** Checking only the immediate `DlgLine` list for non-terminal transitions, missing `->` inside nested blocks (`$ if`, `$ match`, `$ choice` arms) that are themselves terminal in their sub-block.
**Why it happens:** Applying the non-terminal check at the wrong level.
**How to avoid:** The non-terminal check is PER BLOCK — a `->` at the end of a `$ choice` arm body is valid (it is the last statement in THAT arm's block). Only flag as non-terminal if `->` is not the last element of the specific `Vec<Spanned<DlgLine>>` being processed at that recursion level.
**Warning signs:** False positive errors on `->` in choice arms.

---

## Code Examples

Verified patterns from the codebase and spec:

### say_localized Call Construction

```rust
// Source: spec §28.4 + AstExpr definition in writ-compiler/src/ast/expr.rs
// Emits: say_localized(speaker_ref, "a3f7c012", fallback_text_expr)
AstExpr::Call {
    callee: Box::new(AstExpr::Ident {
        name: "say_localized".to_string(),
        span: line_span,
    }),
    args: vec![
        AstArg { name: None, value: speaker_ref_expr, span: line_span },
        AstArg { name: None, value: AstExpr::StringLit { value: loc_key, span: line_span }, span: line_span },
        AstArg { name: None, value: fallback_text_expr, span: line_span },
    ],
    span: line_span,
}
```

### Entity.getOrCreate<T>() Construction

```rust
// Source: spec §28.2 + AstExpr::GenericCall in writ-compiler/src/ast/expr.rs
// Emits: Entity.getOrCreate<Narrator>()
AstExpr::GenericCall {
    callee: Box::new(AstExpr::MemberAccess {
        object: Box::new(AstExpr::Ident { name: "Entity".to_string(), span }),
        field: "getOrCreate".to_string(),
        field_span: span,
        span,
    }),
    type_args: vec![AstType::Named { name: speaker_name.to_string(), span }],
    args: vec![],
    span,
}
```

### Hoisted let-binding for singleton speaker

```rust
// Source: spec §28.2
// Emits: let _narrator = Entity.getOrCreate<Narrator>();
AstStmt::Let {
    mutable: false,
    name: format!("_{}", speaker_name.to_lowercase()),
    name_span: speaker_span,
    ty: None,
    value: /* Entity.getOrCreate<Narrator>() as above */,
    span: speaker_span,
}
```

### FNV-1a Implementation

```rust
// Source: spec §25.2.2 (exact algorithm mandated)
fn fnv1a_32(input: &str) -> String {
    const OFFSET_BASIS: u32 = 0x811c9dc5;
    const PRIME: u32 = 0x01000193;
    let mut hash: u32 = OFFSET_BASIS;
    for &byte in input.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{:08x}", hash)
}

// Called as:
fn compute_loc_key(
    namespace: &str,
    method: &str,
    speaker: &str,
    content: &str,
    occurrence_index: u32,
) -> String {
    let input = format!("{}\0{}\0{}\0{}\0{}", namespace, method, speaker, content, occurrence_index);
    fnv1a_32(&input)
}
```

### Transition Lowering

```rust
// Source: spec §13.6, §28.1
// -> target       → AstStmt::Return { value: Some(AstExpr::Call { target, [] }) }
// -> target(args) → AstStmt::Return { value: Some(AstExpr::Call { target, args }) }
fn lower_transition(trans: DlgTransition<'_>, trans_span: SimpleSpan, ctx: &mut LoweringContext) -> AstStmt {
    let (target_name, target_span) = trans.target;
    let args: Vec<AstArg> = trans.args.unwrap_or_default()
        .into_iter()
        .map(|(e, e_span)| AstArg { name: None, value: lower_expr((e, e_span), ctx), span: e_span })
        .collect();
    AstStmt::Return {
        value: Some(AstExpr::Call {
            callee: Box::new(AstExpr::Ident { name: target_name.to_string(), span: target_span }),
            args,
            span: trans_span,
        }),
        span: trans_span,
    }
}
```

### Snapshot Test Pattern (per lowering_tests.rs)

```rust
// Source: writ-compiler/tests/lowering_tests.rs — established pattern
#[test]
fn dlg_speaker_singleton_tier2() {
    // Tests that @Narrator (not a param) → Entity.getOrCreate<Narrator>() hoisted let
    let ast = lower_src("dlg intro { @Narrator Hello. }");
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn dlg_unknown_speaker_emits_error() {
    // Tests that @Unknown emits LoweringError::UnknownSpeaker and continues
    // Note: lower_src panics on lowering errors; need lower_src_with_errors helper
    let (ast, errors) = lower_src_with_errors("dlg intro { @Unknown Hello. }");
    insta::assert_debug_snapshot!((ast, errors));
}
```

**Note on test helper:** The current `lower_src()` asserts `lower_errors.is_empty()`. A new `lower_src_with_errors()` helper that returns `(Ast, Vec<LoweringError>)` without asserting is needed for the error-path snapshot tests (unknown speaker, non-terminal transition, duplicate key).

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `ctx.next_loc_key()` as u32 counter (placeholder) | FNV-1a 32-bit hash → 8-char hex string | Phase 4 implementation | Counter was always a placeholder per R9; actual hash is the correct output |
| `todo!("Phase 4: dialogue lowering")` in mod.rs | `lower_dialogue()` from new dialogue.rs module | Phase 4 implementation | Removes two panic sites |
| `todo!("Phase 4: dialogue statement lowering")` in stmt.rs | Handled `Stmt::DlgDecl` and `Stmt::Transition` | Phase 4 implementation | Removes two more panic sites |

**Deprecated/outdated:**
- `loc_key_counter` raw value as output: was always a placeholder, now used only as occurrence_index tracking input to FNV-1a.

---

## Open Questions

1. **`Stmt::DlgDecl` lowering target**
   - What we know: `Stmt::DlgDecl(Spanned<DlgDecl>)` appears in `Stmt` enum (a `dlg` declared inside a fn body or code block). `lower_stmt` must return `AstStmt`, not `AstDecl::Fn`.
   - What's unclear: The spec §13 only describes `dlg` at top level or within namespace blocks; inline `dlg` inside a fn body is not explicitly specified. The CST allows it (the parser accepts it), but the lowering semantics are ambiguous.
   - Recommendation: Treat `Stmt::DlgDecl` as equivalent to a nested named closure: lower to `AstStmt::Let { name: dlg.name, value: AstExpr::Lambda { params, body } }`. If this is incorrect, it surfaces easily in snapshot tests. Alternatively, emit a `LoweringError::Generic` with message "inline dlg declarations not yet supported" and an `AstStmt::Error` placeholder — deferring until spec is clarified.

2. **Namespace context for FNV-1a key**
   - What we know: The FNV-1a input requires `namespace` (e.g., `"dialogue"`). The lowering pipeline receives `Vec<Spanned<Item>>` without namespace context — namespace is determined by the enclosing `NamespaceDecl`.
   - What's unclear: The current `LoweringContext` has no `current_namespace` field. The namespace would need to be threaded through from the namespace-block processing in `lower_namespace()`.
   - Recommendation: Add `current_namespace: Vec<String>` to `LoweringContext` (set when entering namespace blocks, reset on exit). For dialogues outside any namespace, use empty string `""`. If adding namespace context to `LoweringContext` is out of scope for Phase 4, use the empty string and document it as a known simplification — the FNV-1a key will still be unique within the compilation unit.

3. **Tier 2 (Singleton entity) determination without type info**
   - What we know: Speaker resolution Tier 2 says "check `[Singleton]` entities." But at lowering time, the compiler does not yet have type information — it cannot look up whether `Narrator` is actually a `[Singleton]` entity (that requires name resolution, which is a later phase per project Non-Requirements).
   - What's unclear: Should Tier 2 be treated as "any name not found in params is assumed to be a singleton"? Or should lowering defer to name resolution?
   - Recommendation: Per the spec §13.2 and the spec lowering example §28.2, the project's practical approach is: if the speaker name is not a param/local, assume it is a Singleton entity and emit `Entity.getOrCreate<T>()`. The name resolution phase downstream will produce a proper error if the type doesn't exist. This is consistent with the "lowering continues even on error" philosophy. Tier 3 (UnknownSpeaker error) would only be triggered if the implementation has additional context to distinguish — for Phase 4, treat any non-param speaker as Tier 2.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `insta` 1.x with `ron` feature |
| Config file | none — snapshots in `writ-compiler/tests/snapshots/` |
| Quick run command | `INSTA_UPDATE=always cargo test -p writ-compiler` |
| Full suite command | `INSTA_UPDATE=always cargo test -p writ-compiler` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| R8 | `dlg name { @Speaker text }` → `fn name { let _s = getOrCreate; say_localized(...) }` | snapshot | `cargo test -p writ-compiler dlg_speaker_singleton_tier2` | No — Wave 0 |
| R8 | Tier 1: `dlg d(p: Entity) { @p Hello. }` → `say_localized(p, ...)` (param direct ref) | snapshot | `cargo test -p writ-compiler dlg_speaker_param_tier1` | No — Wave 0 |
| R8 | `$ choice { "A" { @A Hello. } "B" { @B Bye. } }` → choice with lambda arms, scoped speakers | snapshot | `cargo test -p writ-compiler dlg_choice_branch_speaker_scoping` | No — Wave 0 |
| R8 | `$ if cond { @N Yes. } else { @N No. }` → `if cond { say_localized } else { say_localized }` | snapshot | `cargo test -p writ-compiler dlg_conditional_if` | No — Wave 0 |
| R8 | `$ match s { A => { @N Case A. } }` → `match s { ... }` with say_localized arms | snapshot | `cargo test -p writ-compiler dlg_conditional_match` | No — Wave 0 |
| R8 | `{expr}` in dialogue text → formattable string lowering (concat chain) | snapshot | `cargo test -p writ-compiler dlg_text_interpolation` | No — Wave 0 |
| R8 | Tier 3: `@Unknown Hello.` → `LoweringError::UnknownSpeaker` + lowering continues | snapshot | `cargo test -p writ-compiler dlg_unknown_speaker_error` | No — Wave 0 |
| R9 | Auto-key: two identical-text lines in same dlg produce distinct keys (occurrence_index differs) | snapshot | `cargo test -p writ-compiler dlg_loc_key_distinct_for_duplicate_text` | No — Wave 0 |
| R9 | Manual `#key` override replaces auto-key in say_localized output | snapshot | `cargo test -p writ-compiler dlg_loc_key_manual_override` | No — Wave 0 |
| R10 | Duplicate `#key` within a dlg block → `LoweringError::DuplicateLocKey` | snapshot | `cargo test -p writ-compiler dlg_loc_key_duplicate_collision` | No — Wave 0 |
| R11 | `-> target` at end of block → `AstStmt::Return { AstExpr::Call }` | snapshot | `cargo test -p writ-compiler dlg_transition_lowering` | No — Wave 0 |
| R11 | `->` before end of block → `LoweringError::NonTerminalTransition` | snapshot | `cargo test -p writ-compiler dlg_non_terminal_transition_error` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-compiler`
- **Per wave merge:** `cargo test -p writ-compiler`
- **Phase gate:** Full suite green (all 29 existing + new dialogue tests) before `/gsd:verify-work`

### Wave 0 Gaps

All snapshot test files need to be written in Phase 4:
- [ ] `writ-compiler/tests/lowering_tests.rs` — add R8/R9/R10/R11 test functions (file exists, add to it)
- [ ] `lower_src_with_errors()` helper needed in `lowering_tests.rs` — returns `(Ast, Vec<LoweringError>)` without asserting errors are empty

*(No new framework install needed — `insta` already in dev-dependencies)*

---

## Sources

### Primary (HIGH confidence)

- `D:/dev/git/Writ/language-spec/spec/14_13_dialogue_blocks_dlg.md` — Complete dialogue syntax, speaker resolution tiers, $ forms, choices, transitions, localization keys, text interpolation
- `D:/dev/git/Writ/language-spec/spec/29_28_lowering_reference.md` — Lowering rules table, full dialogue lowering example (§28.1, §28.2), localized lowering (§28.4), runtime function signatures (§28.5)
- `D:/dev/git/Writ/language-spec/spec/26_25_localization.md` — FNV-1a 32-bit algorithm (§25.2.2), key input string format (§25.2.1), deduplication index (§25.2.3)
- `D:/dev/git/Writ/writ-parser/src/cst.rs` — `DlgDecl`, `DlgLine`, `DlgTextSegment`, `DlgEscape`, `DlgChoice`, `DlgChoiceArm`, `DlgIf`, `DlgElse`, `DlgMatch`, `DlgMatchArm`, `DlgTransition` — all fully defined, no unknowns
- `D:/dev/git/Writ/writ-compiler/src/lower/context.rs` — `LoweringContext` with `speaker_stack`, `loc_key_counter`, `push_speaker()`, `pop_speaker()`, `current_speaker()`, `next_loc_key()` — all already implemented
- `D:/dev/git/Writ/writ-compiler/src/lower/error.rs` — `LoweringError::UnknownSpeaker`, `NonTerminalTransition`, `DuplicateLocKey` — all already defined
- `D:/dev/git/Writ/writ-compiler/src/lower/mod.rs` — Two `todo!("Phase 4")` sites identified: `Item::Dlg` in `lower()` and in `lower_namespace()`
- `D:/dev/git/Writ/writ-compiler/src/lower/stmt.rs` — Two `todo!("Phase 4")` sites identified: `Stmt::DlgDecl` and `Stmt::Transition`
- `D:/dev/git/Writ/writ-compiler/src/lower/fmt_string.rs` — Reference pattern for `DlgTextSegment` lowering (parallel structure)
- `D:/dev/git/Writ/writ-compiler/src/lower/operator.rs` — Module structure pattern to follow: single pub entry point, all helpers private
- `D:/dev/git/Writ/writ-compiler/tests/lowering_tests.rs` — Established snapshot test patterns: `lower_src()`, `insta::assert_debug_snapshot!`, `INSTA_UPDATE=always`
- `D:/dev/git/Writ/writ-compiler/src/ast/` — `AstFnDecl`, `AstStmt`, `AstExpr` — output types fully defined

### Secondary (MEDIUM confidence)

- `D:/dev/git/Writ/.planning/STATE.md` — Key decisions: assert_debug_snapshot over assert_ron_snapshot; lower_src takes &'static str; INSTA_UPDATE=always for snapshot acceptance

### Tertiary (LOW confidence)

- Treatment of `Stmt::DlgDecl` lowering target: not explicitly specced, recommendation is LOW confidence (see Open Questions #1)
- Namespace context for FNV-1a in Phase 4: simplification to empty string is LOW confidence for correctness but HIGH confidence for unblocking implementation (see Open Questions #2)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies needed; all tools already present
- Architecture: HIGH — CST types fully defined, AST targets fully defined, infrastructure already in place, four precise `todo!()` call sites identified
- Pitfalls: HIGH — drawn from spec, codebase structure, and analogy to Phase 3 patterns
- Open questions: MEDIUM — three genuinely ambiguous points that require planner decisions

**Research date:** 2026-02-26
**Valid until:** Stable — this research covers a spec-locked domain with no external library changes; valid until spec changes
