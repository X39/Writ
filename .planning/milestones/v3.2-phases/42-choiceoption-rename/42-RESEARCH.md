# Phase 42: ChoiceOption Rename - Research

**Researched:** 2026-03-06
**Domain:** Pure rename across four compiler layers: spec text, lowering emit site, virtual module registration, resolver prelude
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **Rename scope (atomic across four layers)**
  - Spec (`29_28_lowering_reference.md`): Two occurrences — `Option("Good!", fn() { ... })` → `ChoiceOption("Good!", fn() { ... })` (lines 53 and 57). No other spec file uses `Option(...)` as a choice constructor.
  - Lowering (`writ-compiler/src/lower/dialogue.rs` line ~657): `name: "Option".to_string()` → `name: "ChoiceOption".to_string()` — this is the single emit site for the choice option AST node.
  - Virtual module (`writ-runtime/src/virtual_module.rs`): Add a `ChoiceOption` TypeDef or ExternDef registration (the current `Option` TypeDef at line 183 is `Option<T>` with 1 type param — it must not be renamed). The choice option constructor needs its own registration.
  - Prelude (`writ-compiler/src/resolve/prelude.rs`): The existing `"Option"` in `PRELUDE_TYPE_NAMES` is `Option<T>` — keep it. Add `"ChoiceOption"` as a recognized name so the resolver doesn't reject it.
- **Snapshot updates**: Insta snapshots in `writ-compiler/tests/snapshots/` that contain `name: "Option"` for choice option calls must be updated. Affected snapshots: `dlg_choice_basic`, `dlg_choice_label_key_emitted`, `dlg_choice_speaker_scope_isolation`, `integration_all_constructs`. Update via `cargo insta test --accept`.
- **Option<T> isolation**: All `"Option"` references in `check/env.rs`, `check/infer.rs`, `emit/collect.rs`, `lower/expr.rs`, `lower/optional.rs`, `lower/desugar.rs`, `resolve/prelude.rs`, and `virtual_module.rs` that refer to `Option<T>` must NOT be renamed.
- **Success verification**: After rename, a Writ script using `$ choice { "Good!" { ... } "Bad" { ... } }` must compile without error and emitted IL must contain `CALL_EXTERN` referencing a `ChoiceOption` ExternDef token.

### Claude's Discretion
- Exact TypeDef vs ExternDef representation for `ChoiceOption` in virtual module (TypeDef with kind=0, or ExternDef — whichever matches how the existing `Option(...)` call was handled before)
- Whether `ChoiceOption` goes into `PRELUDE_TYPE_NAMES` or a separate list
- Whether to add a new golden test fixture for the full choice path post-rename (recommended, but plan can decide)

### Deferred Ideas (OUT OF SCOPE)
- None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LANG-01 | The dialogue choice option type is renamed from `Option(...)` to `ChoiceOption(...)` in spec and implementation (lowering, virtual module, prelude) — resolves naming conflict with `Option<T>` | All four layers are identified; exact change at each site is documented; snapshot blessing pattern is verified |
</phase_requirements>

---

## Summary

Phase 42 is a pure rename of the dialogue choice option constructor across four layers. No new semantics are introduced. The rename resolves the naming collision where `Option(...)` (choice option constructor) and `Option<T>` (nullable wrapper type) share the same string identifier.

The four layers are fully verified from source inspection:
1. **Spec** (`language-spec/spec/29_28_lowering_reference.md`): Lines 53 and 57 in the §28.2 lowering example show `Option("Good!", fn() { ... })` — both become `ChoiceOption(...)`.
2. **Lowering** (`writ-compiler/src/lower/dialogue.rs`): The `lower_choice` function at line ~656 emits `name: "Option".to_string()` for each choice arm's `AstExpr::Call` callee — single change point.
3. **Virtual module** (`writ-runtime/src/virtual_module.rs`): `Option<T>` is registered as a TypeDef at line 183 with `kind=1` (Enum) and 1 generic param. `ChoiceOption` needs a separate registration. The resolution path means this is an ExternDef (function call, not type), not a TypeDef — but the exact mechanism requires careful analysis (see Architecture Patterns).
4. **Prelude** (`writ-compiler/src/resolve/prelude.rs`): `PRELUDE_TYPE_NAMES` contains `"Option"` (for `Option<T>`). Adding `"ChoiceOption"` here would block user `extern fn ChoiceOption(...)` declarations due to the `is_prelude_name` guard in `collector.rs`. The correct approach is a separate inbuilt-call list.

**Primary recommendation:** Add `"ChoiceOption"` to a new or existing list that bypasses the prelude shadow guard, register it as an ExternDef in `virtual_module.rs` (matching how `say`/`choice`/`log` work), and change the one emit site in `lower/dialogue.rs`. Bless four insta snapshots with `cargo insta test --accept`.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| insta | (workspace) | Snapshot testing for AST lowering | Already the project's snapshot framework; `cargo insta test --accept` is the established blessing pattern |
| cargo test | (Rust toolchain) | Run compiler tests | Standard Rust; all tests invoked this way |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cargo insta review | (workspace) | Interactive snapshot review | Alternative to `--accept`; use when reviewing diffs interactively |

**Installation:** No new dependencies needed. All tooling is already in the workspace.

## Architecture Patterns

### How the Compiler Resolves Bare-Name Calls (e.g., `ChoiceOption(...)`)

The critical path for `ChoiceOption` resolution:

1. **Lowering** emits `AstExpr::Call { callee: AstExpr::Ident { name: "ChoiceOption" }, ... }`.
2. **Collector (Pass 1)** scans top-level declarations. If the Writ script includes `pub extern fn ChoiceOption(...)`, it is inserted into the DefMap — UNLESS `is_prelude_name("ChoiceOption")` returns true (which blocks insertion and emits E0006 PreludeShadow).
3. **Resolver (Pass 2)** resolves the callee name via `scope.resolve_value("ChoiceOption")` → `resolve_type()`, which checks PRELUDE_TYPE_NAMES, then the DefMap.
4. **Type-checker** calls `find_fn_def_id(ctx, "ChoiceOption")` — checks DefMap for ExternFn entries by name.
5. **Emitter** sees `callee_def_id` pointing to an ExternDef token → emits `CALL_EXTERN`.

**The correct pattern** is what `say`, `choice`, and `log` use: the user Writ script declares `pub extern fn ChoiceOption(label: string, key: string, body: fn()) -> ChoiceOption;` — and that declaration registers into the DefMap as `DefKind::ExternFn`. The virtual module provides the ExternDef token for the IL module's ExternDef table.

### Pattern 1: ExternFn Declaration in User Script (Verified Pattern)

The `fn_log_say_choice.writ` golden fixture shows the established pattern:

```writ
// Source: writ-golden/tests/golden/fn_log_say_choice.writ
pub extern fn log(msg: string);
pub extern fn say(text: string);
pub extern fn choice();

pub fn main() {
    ::log("saying Test");
    ::say("Test");
    ::choice();
}
```

For `ChoiceOption`, the test fixture for the integration test would need:

```writ
pub extern fn ChoiceOption(label: string, key: string, body: fn());
pub extern fn choice(options: Array<ChoiceOption>);

dlg ask() {
    @Narrator What do you think?
    $ choice {
        "Good!" { @Narrator Great! }
        "Bad" { @Narrator Sorry. }
    }
}
```

After lowering, the `$ choice { ... }` block emits `choice([ChoiceOption("Good!", ..., fn() { ... }), ChoiceOption("Bad", ..., fn() { ... })])`.

### Pattern 2: Virtual Module Registration

The virtual_module builds the `writ-runtime` module programmatically. ExternDef is for function declarations, TypeDef is for type declarations. `ChoiceOption` as used in the lowered AST is called like a function, so it maps to ExternDef in the virtual module.

**IMPORTANT finding from source inspection:** The current `virtual_module.rs` does NOT register `Option` as an ExternDef for the choice constructor. The existing TypeDef at line 183 (`add_type_def("Option", "writ", 1, 0)`) is purely for `Option<T>` (nullable wrapper). The `Option(...)` choice constructor call was resolved through the user's `extern fn` declaration in the Writ script, not through the virtual module.

This means: Adding `ChoiceOption` to `virtual_module.rs` as an ExternDef is the new behavior that wasn't present before (the old code relied on user-declared `extern fn Option(...)`). The CONTEXT.md says "whichever matches how the existing `Option(...)` call was handled before" — which is as an `extern fn` in the user script, not in `virtual_module.rs`.

**Resolution for Claude's Discretion item:** `ChoiceOption` does NOT need to be added to `virtual_module.rs`. The correct approach is to add it as a `pub extern fn ChoiceOption(...)` declaration in the Writ scripts that use dialogue `$ choice { ... }`. The virtual module is for `writ-runtime`'s built-in types/contracts, not for user-facing ExternFn declarations.

### Pattern 3: Prelude vs Non-Prelude Names

From `resolve/collector.rs`:

```rust
// Source: writ-compiler/src/resolve/collector.rs
fn try_insert(...) {
    if is_prelude_name(name) {
        // Emits E0006 PreludeShadow error — blocks insertion
        return;
    }
    // ... insert into DefMap
}
```

From `resolve/prelude.rs`:

```rust
// Source: writ-compiler/src/resolve/prelude.rs
pub const PRELUDE_TYPE_NAMES: &[&str] = &["Option", "Result", "Range", "Array", "Entity"];
```

**Key constraint:** If `"ChoiceOption"` is added to `PRELUDE_TYPE_NAMES`, then `pub extern fn ChoiceOption(...)` declarations in Writ scripts will be rejected with E0006. Therefore `"ChoiceOption"` must NOT go into `PRELUDE_TYPE_NAMES`.

Since `ChoiceOption` is not a prelude type (it's an extern call, not a type), and the prelude check only runs for types in the DefMap, adding `"ChoiceOption"` to the prelude is not needed. The `extern fn ChoiceOption(...)` declaration pattern naturally handles resolution through the DefMap.

**Conclusion for Claude's Discretion item:** Do NOT add `"ChoiceOption"` to `PRELUDE_TYPE_NAMES` or any separate prelude list. It is resolved through the standard `extern fn` declaration → DefMap → ExternDef path.

### Pattern 4: Snapshot Blessing

```bash
# From project root — runs failing tests and blesses updated snapshots
cargo insta test --accept -p writ-compiler
# Or review interactively
cargo insta review
```

The four affected snapshots contain `name: "Option"` in choice arm callee positions:
- `lowering_tests__dlg_choice_basic.snap` — 2 occurrences
- `lowering_tests__dlg_choice_label_key_emitted.snap` — 2 occurrences
- `lowering_tests__dlg_choice_speaker_scope_isolation.snap` — 2 occurrences
- `lowering_tests__integration_all_constructs.snap` — 2 occurrences

After the rename, each `name: "Option"` in a choice arm callee position becomes `name: "ChoiceOption"`. Span numbers do not change (same character positions in source).

### Recommended Project Structure (for new test fixture)

```
writ-golden/tests/golden/
├── fn_log_say_choice.writ        # Existing (no change needed — uses ::choice() with no args)
├── fn_log_say_choice.writc       # Existing golden binary
├── fn_log_say_choice.writil      # Existing golden IL
├── dlg_choice_option.writ        # NEW: tests ChoiceOption path post-rename (optional)
├── dlg_choice_option.writc       # NEW: blessed binary (via bless_golden)
└── dlg_choice_option.writil      # NEW: blessed IL (via bless_golden)
```

### Anti-Patterns to Avoid

- **Renaming `Option<T>` registrations**: The `add_type_def("Option", ...)` in `virtual_module.rs` at line 183 is for the nullable wrapper type and must NOT be renamed. The `option_is_enum_with_one_generic_param` test in `virtual_module.rs` verifies this.
- **Adding `"ChoiceOption"` to `PRELUDE_TYPE_NAMES`**: Would block `extern fn ChoiceOption(...)` declarations via the `is_prelude_name` guard in `collector.rs`.
- **Touching `virtual_module.rs` test assertions**: The test `type_defs_include_all_nine_types` asserts exactly 9 type_defs including `"Option"` — this is `Option<T>` and must remain.
- **Partial rename**: Changing only the lowering emit site without updating snapshots causes failing tests that must be blessed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Snapshot update | Manual text editing of `.snap` files | `cargo insta test --accept` | Insta recomputes spans and formats from actual output — manual edits risk stale spans or formatting drift |
| Option<T> isolation grep | Custom regex | `grep -n '"Option"'` with context review | The context distinguishes choice vs nullable: choice context has bare `name: "Option"` in a Call with 3 args; nullable context has Generic with `<T>` args |

**Key insight:** The snapshot blessing pattern (`cargo insta test --accept`) is faster and more reliable than manual `.snap` editing. Run tests to fail, then accept — never pre-edit snapshots.

## Common Pitfalls

### Pitfall 1: Renaming Option<T> by Accident

**What goes wrong:** Grepping for `"Option"` and replacing all occurrences also renames the nullable wrapper type `Option<T>`, breaking type system code.
**Why it happens:** Both the choice constructor and the nullable type share the string `"Option"`.
**How to avoid:** Distinguish by context:
  - Choice constructor: `name: "Option".to_string()` inside `lower_choice` function, in a `AstExpr::Call` callee — there is exactly ONE such site in `lower/dialogue.rs`.
  - Nullable type: Appears in `check/env.rs`, `lower/optional.rs`, `lower/desugar.rs`, `resolve/prelude.rs`, `virtual_module.rs` — these always involve `<T>`, `generic_params`, or `kind=Enum`.
**Warning signs:** If `virtual_module.rs` tests start failing (e.g., `option_is_enum_with_one_generic_param`), the wrong occurrence was renamed.

### Pitfall 2: Forgetting Snapshot Updates

**What goes wrong:** The four lowering test snapshots still contain `name: "Option"` in choice arm positions after the rename, causing test failures.
**Why it happens:** Insta snapshots are stored files — they don't auto-update with source changes.
**How to avoid:** After changing `lower/dialogue.rs`, immediately run `cargo insta test --accept -p writ-compiler` to bless all updated snapshots.
**Warning signs:** `cargo test -p writ-compiler` shows `snapshot mismatch` failures in any of the four dlg_choice or integration tests.

### Pitfall 3: ChoiceOption Not Resolvable Without extern fn Declaration

**What goes wrong:** The lowering emits `ChoiceOption(...)` calls, but there is no `extern fn ChoiceOption(...)` declaration in the Writ script being compiled, causing resolution failures.
**Why it happens:** `ChoiceOption` is not a built-in inbuilt — it must be declared as `pub extern fn ChoiceOption(...)` in the compilation unit (just like `say`, `choice`, `log`).
**How to avoid:** Any integration test or golden fixture that uses `$ choice { ... }` must include a `pub extern fn ChoiceOption(...)` declaration. The lowering tests only check the AST (pre-resolution), so they do not need this.
**Warning signs:** Type-checker diagnostics show `E0001 UnresolvedName: ChoiceOption` when compiling a full pipeline test.

### Pitfall 4: Virtual Module TypeDef Count Assertion

**What goes wrong:** If `ChoiceOption` is erroneously added to `virtual_module.rs` as a TypeDef, the `type_defs_include_all_nine_types` test fails because it asserts exactly 9 types.
**Why it happens:** The test is a count assertion (`assert_eq!(module.type_defs.len(), 9)`).
**How to avoid:** Do not add `ChoiceOption` as a TypeDef to `virtual_module.rs`. Add it as an `extern fn` declaration in Writ scripts or as an ExternDef if needed — but current analysis shows it belongs in user-space `extern fn` declarations.

## Code Examples

### Single Emit Site to Change

```rust
// Source: writ-compiler/src/lower/dialogue.rs (lower_choice function, ~line 654)
// BEFORE:
AstExpr::Call {
    callee: Box::new(AstExpr::Ident {
        name: "Option".to_string(),    // <-- change this
        span: arm_span,
    }),
    // ...
}

// AFTER:
AstExpr::Call {
    callee: Box::new(AstExpr::Ident {
        name: "ChoiceOption".to_string(),  // <-- renamed
        span: arm_span,
    }),
    // ...
}
```

### Spec Change (two occurrences in §28.2)

```markdown
<!-- Source: language-spec/spec/29_28_lowering_reference.md, lines 53 and 57 -->
<!-- BEFORE: -->
    choice([
        Option("Good!", fn() { ... }),
        Option("Not great", fn() { ... }),
    ]);

<!-- AFTER: -->
    choice([
        ChoiceOption("Good!", fn() { ... }),
        ChoiceOption("Not great", fn() { ... }),
    ]);
```

### Snapshot Pattern After Rename

Each choice arm callee in snapshots changes from:

```
callee: Ident {
    name: "Option",
    span: 52..79,
},
```

to:

```
callee: Ident {
    name: "ChoiceOption",
    span: 52..79,
},
```

The span numbers stay the same (source text positions are unchanged — `ChoiceOption` has more characters than `Option` but the spans come from the lowering which synthesizes arm_span, not from parsing `ChoiceOption` in source).

Wait — actually the lowering emits `span: arm_span` for the Ident, not a character-count-based span. So spans are unchanged in the snapshots. Only `name: "Option"` → `name: "ChoiceOption"`.

### Golden Test Fixture for Integration Verification (optional, Claude's discretion)

```writ
// New file: writ-golden/tests/golden/dlg_choice_option.writ
pub extern fn say(speaker: Entity, text: string);
pub extern fn choice(options: Array<ChoiceOption>);
pub extern fn ChoiceOption(label: string, key: string, body: fn());
extern fn Entity.getOrCreate<T>() -> T;

dlg ask() {
    @Narrator What do you think?
    $ choice {
        "Good!" { @Narrator Great! }
        "Bad" { @Narrator Sorry. }
    }
}
```

### Insta Test Command

```bash
# Run from the Writ workspace root
cargo insta test --accept -p writ-compiler

# Or to only run the affected tests:
cargo test -p writ-compiler dlg_choice
cargo insta accept
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Option(...)` as choice constructor name | `ChoiceOption(...)` | Phase 42 | Eliminates naming conflict with `Option<T>` nullable wrapper |
| Snapshot reflects `name: "Option"` in choice arms | Snapshot reflects `name: "ChoiceOption"` | Phase 42 (blessing) | Four `.snap` files updated |

**Deprecated/outdated:**
- `name: "Option".to_string()` in `lower_choice`: Replaced by `name: "ChoiceOption".to_string()`
- `Option("Good!", fn() { ... })` in spec §28.2: Replaced by `ChoiceOption("Good!", fn() { ... })`

## Open Questions

1. **Does `ChoiceOption` need to be added to `virtual_module.rs`?**
   - What we know: `Option(...)` as a choice constructor was resolved through user-space `extern fn Option(...)` declarations, not through `virtual_module.rs`. The `virtual_module.rs` only registers `Option<T>` (the nullable type) as a TypeDef.
   - What's unclear: Whether any integration path currently relies on a virtual-module ExternDef for `ChoiceOption`.
   - Recommendation: Do NOT add to `virtual_module.rs` for this phase. The user-space `extern fn ChoiceOption(...)` pattern suffices. If a future integration test compiles a `$ choice { ... }` script end-to-end without an `extern fn ChoiceOption` declaration, it will fail at resolution — that's the signal to add a built-in prelude ExternDef.

2. **Should a new golden fixture be added?**
   - What we know: The CONTEXT.md marks this as "recommended, but plan can decide." The existing `fn_log_say_choice.writ` uses `::choice()` with no-args form (simplified in Phase 41) and does not exercise `ChoiceOption`.
   - What's unclear: Whether the success criterion "A Writ script using `$ choice { ... }` compiles and emits CALL_EXTERN for ChoiceOption" can be validated via a unit test instead of a golden fixture.
   - Recommendation: Add a new unit test in `typecheck_tests.rs` or `emit_body_tests.rs` that compiles a minimal `$ choice { ... }` script with `extern fn ChoiceOption(...)` declared and asserts the emitted IL contains `CALL_EXTERN` with a `ChoiceOption` ExternDef token. This is faster to write than a full golden fixture.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test + insta snapshot testing |
| Config file | `Cargo.toml` (workspace) |
| Quick run command | `cargo test -p writ-compiler dlg_choice` |
| Full suite command | `cargo test -p writ-compiler && cargo test -p writ-golden && cargo test -p writ-runtime` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LANG-01 | `lower_choice` emits `ChoiceOption` name (not `Option`) | unit snapshot | `cargo test -p writ-compiler dlg_choice_basic` | ✅ (needs blessing) |
| LANG-01 | All four dlg_choice snapshots reflect rename | snapshot | `cargo insta test --accept -p writ-compiler` | ✅ (needs blessing) |
| LANG-01 | Integration snapshot reflects rename | snapshot | `cargo test -p writ-compiler integration_all_constructs` | ✅ (needs blessing) |
| LANG-01 | `Option<T>` typedef unchanged in virtual module | unit | `cargo test -p writ-runtime option_is_enum_with_one_generic_param` | ✅ |
| LANG-01 | End-to-end: `$ choice { ... }` compiles with CALL_EXTERN for ChoiceOption | integration | `cargo test -p writ-compiler choice_option_emits_call_extern` | ❌ Wave 0 |
| LANG-01 | String "Option" absent as choice constructor name in spec | manual review | grep check | ✅ (after edit) |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-compiler dlg_choice`
- **Per wave merge:** `cargo test -p writ-compiler && cargo test -p writ-runtime`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `writ-compiler/tests/emit_body_tests.rs` — add `choice_option_emits_call_extern` test covering REQ LANG-01 success criterion 3 (CALL_EXTERN references ChoiceOption ExternDef token). This test compiles a minimal Writ script with `extern fn ChoiceOption(...)` + `$ choice { ... }` and asserts IL contains `CALL_EXTERN` pointing to an ExternDef named `ChoiceOption`.

## Sources

### Primary (HIGH confidence)

- Direct source inspection — `writ-compiler/src/lower/dialogue.rs`: `lower_choice` function at ~line 654; confirmed single emit site with `name: "Option".to_string()`
- Direct source inspection — `writ-runtime/src/virtual_module.rs`: Line 183 `add_type_def("Option", "writ", 1, 0)` is `Option<T>` with `kind=Enum=1` and 1 generic param; confirmed separate from choice constructor
- Direct source inspection — `writ-compiler/src/resolve/prelude.rs`: `PRELUDE_TYPE_NAMES` contains `"Option"`; adding `"ChoiceOption"` here would trigger `is_prelude_name` guard in `collector.rs`
- Direct source inspection — `language-spec/spec/29_28_lowering_reference.md`: Lines 53 and 57 contain `Option(...)` choice constructor calls in §28.2
- Direct source inspection — `writ-compiler/tests/snapshots/`: Four snapshot files confirmed to contain `name: "Option"` for choice arm callees
- Direct source inspection — `writ-compiler/src/resolve/collector.rs`: `try_insert` calls `is_prelude_name` and blocks insertion if true (PreludeShadow)
- Direct source inspection — `writ-compiler/src/check/check_expr.rs`: `find_fn_def_id` resolves to ExternFn in DefMap; `check_call` fast-path uses this for known function names
- Direct source inspection — `writ-golden/tests/golden/fn_log_say_choice.writ`: Established pattern for `pub extern fn` declarations for inbuilt calls

### Secondary (MEDIUM confidence)

- `language-spec/todos.md`: Confirms working form was `::Option("Good!", fn() { ... })` via user-space extern fn path (not virtual module)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; cargo test + insta are already in use
- Architecture: HIGH — all four layers verified from direct source inspection; resolution path traced end-to-end
- Pitfalls: HIGH — Option<T) isolation risk confirmed by reviewing all `"Option"` occurrences in source; prelude guard traced to exact code

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable domain — no external dependencies)
