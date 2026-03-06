# Phase 69: Dialogue/Function Golden Tests - Research

**Researched:** 2026-03-18
**Domain:** Writ compiler golden test infrastructure, dialogue lowering, say/choice builtins
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**GOLD-01 (dlg_fn_mix):**
- Mix `dlg` blocks and `fn` declarations in the same file
- A `dlg` block that calls a helper `fn` (e.g., compute a value, format a string)
- A `fn` that calls a dialogue function (dialogue transition via direct call)
- Exercise: `@Speaker` text lines, `$ let` code escapes, `-> transition` syntax, speaker parameter (Tier 1)
- Avoid `$ choice` in this test — focus on fn/dlg interplay without triggering the known `::choice` serialization bug

**GOLD-02 (dlg_quest_pattern):**
- Full quest scenario: entity declaration, `dlg` blocks with speaker interaction, helper `fn` declarations, enum for quest state, `$ choice` blocks
- Exercise: entity + `dlg` + `fn` + enum match + `$ choice` + `@Speaker` + `-> transition`
- Risk: `$ choice` lowers to `::choice([::ChoiceOption(..., fn() {}), ...])` — known serialization failure (UnexpectedEof) in multi-function modules. If triggered, avoid `$ choice` and document the limitation

**File naming and registration:**
- GOLD-01: `writ-golden/tests/golden/dlg_fn_mix.writ`
- GOLD-02: `writ-golden/tests/golden/dlg_quest_pattern.writ`
- Register both in `writ-golden/tests/golden_tests.rs` as new Section L: Dialogue golden tests
- Each gets `#[test] fn test_dlg_fn_mix()` / `#[test] fn test_dlg_quest_pattern()` using `run_golden_test()`

**Snapshot workflow:**
- Write `.writ` source files
- Run `BLESS=1 cargo test -p writ-golden -- dlg_` to generate `.writil` blessed snapshots
- Verify both pass under `cargo test --workspace`

### Claude's Discretion
- Exact Writ source code content (specific variable names, enum variants, dialogue text)
- Number of functions/dialogues in each test file (enough to exercise the patterns)
- Whether to include `$ if` / `$ match` dialogue conditionals in addition to core patterns
- Doc comment style on test functions

### Deferred Ideas (OUT OF SCOPE)
- Fix `::choice` with `fn() {}` lambda serialization bug (UnexpectedEof in multi-function modules)
- Entity golden tests (standalone entity without dialogue)
- Locale override golden tests (`[Locale("ja")] dlg`)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| GOLD-01 | Golden test file with basic dialogue/function mix (functions and dialogues in same file, calling each other) | Confirmed: `run_golden_test()` infrastructure works. Critical: `say` signature mismatch must be fixed for `@Speaker` syntax to compile. |
| GOLD-02 | Golden test file with full quest pattern (entity + dialogue + functions + choices), blessed as golden test | Confirmed: entity compilation pipeline exists. `$ choice` risk confirmed — see Critical Blocker section. |
</phase_requirements>

---

## Summary

Phase 69 adds two golden test `.writ` files exercising `dlg` blocks (dialogue/function interplay). The infrastructure to add and bless them is mature and well-understood — `run_golden_test(name)` handles the full pipeline end-to-end. No changes to the test harness are required.

**Critical blocker discovered**: The `say` builtin signature in the implementation does not match the spec or the dialogue lowering. The spec defines `say(speaker: Entity, text: string)` (2 params). The dialogue lowering (`make_say`) generates `say(speaker_ref, text)` with 2 args. But `env.rs` and `builtins.rs` register `say` with only 1 param: `say(text: string) -> void`. This arity mismatch will cause a type error for any `dlg` block using `@Speaker` attribution lines. This must be fixed before the `.writ` files can compile.

**`$ choice` risk**: The known `UnexpectedEof` serialization bug in multi-function modules applies to GOLD-02. The plan must probe whether the bug triggers and adapt accordingly.

**Primary recommendation:** Fix the `say` signature mismatch first (update `env.rs` + `builtins.rs`), then write `.writ` test files, then bless snapshots.

---

## Critical Blocker: `say` Signature Mismatch

### What the spec says
`language-spec/spec/28_27_standard_library_builtins.md §1.27.4`:
```
say(speaker: Entity, text: string)  — 2 params
```

### What the dialogue lowering generates
`writ-compiler/src/lower/dialogue.rs`, `make_say()`:
```rust
// Emits say(speaker_ref, text) — 2 arguments
AstExpr::Call {
    callee: AstExpr::Ident { name: "say" },
    args: vec![
        AstArg { value: speaker_ref },  // arg 1: Entity reference
        AstArg { value: text },         // arg 2: string
    ],
}
```

### What the type checker has registered
`writ-compiler/src/check/env.rs:256`:
```rust
("say", vec![("text", string_ty)], void_ty),  // 1 param only — WRONG
```

### What the emitter has registered
`writ-compiler/src/emit/collect/builtins.rs:77`:
```rust
("say", vec![0x01, 0x00, 0x04, 0x00]),  // 1 param sig blob — WRONG
```

### Impact
Any `dlg` block with `@Speaker text` lines will emit `say(speaker_ref, text)` with 2 args. The type checker will emit `ArityMismatch` (expected 1, found 2) and fail. **No existing golden tests use `dlg` blocks with `@Speaker` attribution** — all existing tests use `::say("text")` directly (1-arg form), which matches the broken implementation.

### Fix required (HIGH confidence)
Three files need updating to match the 2-arg spec:
1. `writ-compiler/src/check/env.rs` — add `entity_ty` as first param in `say` dialogue sig
2. `writ-compiler/src/emit/collect/builtins.rs` — update sig blob to 2 params
3. `writ-compiler/src/check/env.rs` — update `say_localized` if it also has a speaker param

**Note on `say_localized`**: The spec dialogue section (§1.14.9) shows `say_localized(speaker, key, fallback)` but the current env.rs has `("say_localized", vec![("key", string_ty), ("locale", string_ty)], void_ty)` — also a potential mismatch. The lowering generates `say_localized(speaker_ref, key, fallback)` with 3 args. Verify against the spec before fixing.

**Note on existing tests**: Changing `say` to 2 params will break `fn_log_say_choice.writ` which calls `::say("Test")` with 1 arg. That file and its golden snapshot must be updated or the 1-arg `::say` call changed to `::say(null, "Test")` or similar. Alternatively, keep `::say` as 1-arg and have the lowering adapt — but this conflicts with the spec.

---

## Standard Stack

### Core — Golden Test Infrastructure
| Component | Location | Purpose | Notes |
|-----------|----------|---------|-------|
| `compile_and_disassemble(src)` | `writ-golden/tests/golden_tests.rs` | Full pipeline: parse→lower→resolve→typecheck→emit→serialize→deserialize→disassemble | Runs on 16 MB stack thread |
| `run_golden_test(name)` | `writ-golden/tests/golden_tests.rs` | Read `.writ`, compile, compare or bless `.writil` | `BLESS=1` env var to generate |
| `.writ` source files | `writ-golden/tests/golden/` | Writ source for golden tests | Naming: `{category}_{description}.writ` |
| `.writil` snapshot files | `writ-golden/tests/golden/` | Blessed disassembly output | Generated by `BLESS=1 cargo test -p writ-golden -- {name}` |

### Test Registration Pattern
```rust
// In writ-golden/tests/golden_tests.rs

// ─── Section L: Dialogue golden tests ────────────────────────────────────────

/// Golden test: [description].
#[test]
fn test_dlg_fn_mix() {
    run_golden_test("dlg_fn_mix");
}

/// Golden test: [description].
#[test]
fn test_dlg_quest_pattern() {
    run_golden_test("dlg_quest_pattern");
}
```

### Bless Workflow
```bash
# Generate blessed snapshots for dlg_ tests only
BLESS=1 cargo test -p writ-golden -- dlg_

# Verify all tests pass including new ones
cargo test --workspace
```

---

## Architecture Patterns

### Dialogue Lowering Pipeline

`dlg name(params) { body }` lowers to `fn name(params) { hoisted_lets + lowered_body }`.

**Speaker resolution tiers:**
- Tier 1: `@paramName text` → `say(paramName, text)` — param used directly as Entity ref
- Tier 2: `@SpeakerName text` → hoisted `let _speakername = Entity.getOrCreate<SpeakerName>()` + `say(_speakername, text)` — singleton entity hoisted to top of fn body

**Lowered constructs (from `writ-compiler/src/lower/dialogue.rs`):**

| Dialogue Syntax | Lowered To |
|-----------------|-----------|
| `@Speaker text` | `say(speaker_ref, "text")` |
| `@Speaker text #key` | `say_localized(speaker_ref, "key", "text")` |
| `@Speaker` (tag) | Pushes speaker to speaker stack — no statement emitted |
| `text` (under active speaker) | `say(current_speaker_ref, "text")` |
| `$ let x = expr;` | Regular `let x = expr;` statement |
| `$ { block }` | Block of regular statements |
| `$ if cond { dlg }` | `if cond { lowered_dlg }` |
| `$ match expr { arm }` | `match expr { lowered_arm }` |
| `$ choice { "label" { dlg } }` | `choice([ChoiceOption("label", key, fn() { lowered_dlg })])` |
| `-> target` | `return target()` |
| `-> target(args)` | `return target(args)` |

**Speaker scope isolation**: Speaker stacks are saved/restored across `$ if`, `$ match`, and `$ choice` arms (DLG-05 fix). A speaker set in one branch does not leak to sibling branches.

### Existing Golden Test Structure

The harness lives in `writ-golden/tests/golden_tests.rs`. Sections A-K are established:
- A: compile_and_disassemble (harness tests)
- B: bless_golden / run_golden_test (harness tests)
- C: Scaffold
- D: Function IL golden tests
- E: Variable golden tests
- F: Expression golden tests
- G: Control flow golden tests
- H: Type golden tests
- I: Function golden tests (additional)
- J: Advanced feature golden tests
- K: Comprehensive golden tests (quest_system)

**New section**: L — Dialogue golden tests (dlg_fn_mix, dlg_quest_pattern)

### GOLD-01: dlg_fn_mix Pattern

Tier 1 speaker (parameter) — avoids the `Entity.getOrCreate` generic call complexity:
```writ
// Helper function called from dlg
fn compute_mood(energy: int) -> int {
    if energy > 50 { 1 } else { 0 }
}

// Dialogue that calls a helper fn via $ code escape
dlg npc_greet(npc: Entity, player: Entity) {
    @npc Hello, traveler.
    $ let mood: int = compute_mood(100);
    $ if mood == 1 {
        @npc I feel great today.
    } else {
        @npc Been a long day.
    }
    -> npc_farewell(npc, player)
}

// Transition target dlg
dlg npc_farewell(npc: Entity, player: Entity) {
    @npc Safe travels.
}

// fn that calls a dlg by direct function call
fn start_encounter(npc: Entity, player: Entity) {
    npc_greet(npc, player);
}
```

**Why Tier 1 (parameter) speakers**: Avoids `Entity.getOrCreate<T>()` which is a generic call requiring entity type resolution and components — complex to resolve in the current type checker. Tier 1 just uses the param as an `Entity` Ident, which the checker handles as an untyped/unknown reference.

**Concern with `Entity` type**: Even Tier 1 speaker resolution emits `say(entity_param, text)`. If `say` is fixed to `say(speaker: Entity, text: string)`, the `entity_param` (typed as `Entity`) will type-check correctly.

### GOLD-02: dlg_quest_pattern Concern

The CONTEXT.md explicitly warns: `$ choice` lowers to `ChoiceOption(..., fn() {})` which triggers the known `UnexpectedEof` serialization bug in multi-function modules. Since GOLD-02 is a multi-function module (entity + dlg + fns + enum), `$ choice` WILL trigger the bug.

**Recommended approach**: Write GOLD-02 WITHOUT `$ choice`. Use `$ if` / `$ match` for branching instead. Document the limitation as a comment in the test file.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Snapshot comparison | Custom diff logic | `run_golden_test(name)` | Already has unified diff, BOM handling, CRLF normalization |
| Test registration | Custom test harness | `#[test]` + `run_golden_test()` | Standard pattern matches all 31 existing tests |
| Blessing snapshots | Manual file creation | `BLESS=1 cargo test -p writ-golden -- {name}` | Handles all path resolution |
| Speaker entity hoisting | Manual let-binding | `dlg` params (Tier 1) | Tier 2 singletons require `Entity.getOrCreate<T>()` which has type-check complexity |

---

## Common Pitfalls

### Pitfall 1: `say` Arity Mismatch (CRITICAL)
**What goes wrong:** `dlg` blocks with `@Speaker text` compile through lowering successfully but fail the type checker with `ArityMismatch: say expects 1 arg, found 2`. The golden test compilation panics with `type error(s): ...`.
**Why it happens:** `env.rs` registers `say(text: string)` (1 param) but lowering generates `say(speaker_ref, text)` (2 args). The spec mandates 2 params.
**How to avoid:** Fix `env.rs` + `builtins.rs` before writing `.writ` test files. Also update `fn_log_say_choice.writ` if `::say` is changed to require 2 args.
**Warning signs:** Type error at compilation: `type error(s): ArityMismatch` for any `dlg` block.

### Pitfall 2: `$ choice` Serialization Bug (KNOWN)
**What goes wrong:** Modules with multiple functions + `$ choice` fail with `UnexpectedEof` during `Module::from_bytes` in the harness.
**Why it happens:** `ChoiceOption(..., fn() {})` creates lambda closures that serialize incorrectly in multi-function modules (TYPE-12 / choice serialization bug, carried from v4.0).
**How to avoid:** Do NOT use `$ choice` in either GOLD-01 or GOLD-02. Use `$ if` / `$ match` for branching instead. Document the limitation.
**Warning signs:** `compile_and_disassemble: Module::from_bytes failed after successful compile`.

### Pitfall 3: Tier 2 Speaker Entity Resolution
**What goes wrong:** `@Narrator` (not a param) triggers `Entity.getOrCreate<Narrator>()` hoisting. If `Narrator` is not declared as an entity in the test file, resolver emits `UndefinedVariable` or name resolution fails.
**Why it happens:** Tier 2 hoisting generates `let _narrator = Entity.getOrCreate<Narrator>()`. If `Narrator` type is unknown, type checking fails.
**How to avoid:** Use Tier 1 speakers exclusively for GOLD-01 (pass speakers as `Entity` params). For GOLD-02 with entities, declare the entity type in the same file.
**Warning signs:** `resolution error(s): undefined variable '_narrator'` or similar.

### Pitfall 4: `fn_log_say_choice.writ` Breakage from `say` Fix
**What goes wrong:** After fixing `say` to 2 params, the existing `fn_log_say_choice.writ` which calls `::say("Test")` (1 arg) will fail type checking.
**Why it happens:** The existing test was written to the 1-arg (broken) `say` signature.
**How to avoid:** When fixing `say`, also update `fn_log_say_choice.writ` to use `::say(null, "Test")` or similar 2-arg form, and re-bless its `.writil` snapshot.
**Warning signs:** `test test_fn_log_say_choice ... FAILED` after the `say` signature fix.

### Pitfall 5: `-> transition` Must Be Last Statement
**What goes wrong:** If `-> target` is placed before the end of a `dlg` block, the lowerer emits `NonTerminalTransition` error and compilation fails.
**Why it happens:** `->` lowers to `return target()` which is only valid as the final statement.
**How to avoid:** Always place `-> transition` as the last line in its block. In choice arms or if-branches, it must be the last item in that branch.
**Warning signs:** `lowering error(s): NonTerminalTransition`.

### Pitfall 6: `$ code escape` Needs Explicit Types
**What goes wrong:** `$ let x = compute_mood(100);` without a type annotation may fail type inference if the return type of `compute_mood` is not fully resolved.
**Why it happens:** The Writ type checker has limited inference for let bindings without annotations.
**How to avoid:** Annotate types explicitly in `$ let x: int = compute_mood(100);`.
**Warning signs:** Type error on `$` escape assignments.

---

## Code Examples

### Test Registration (Section L)
```rust
// Source: writ-golden/tests/golden_tests.rs

// ─── Section L: Dialogue golden tests ─────────────────────────────────────────

/// Golden test: dialogue/function mix — dlg blocks and fn declarations calling each other.
///
/// Exercises @Speaker (Tier 1 param), $ let code escape, $ if conditional,
/// -> transition syntax, and fn-to-dlg and dlg-to-fn call patterns.
/// Regression anchor for GOLD-01.
#[test]
fn test_dlg_fn_mix() {
    run_golden_test("dlg_fn_mix");
}

/// Golden test: full quest pattern — entity, dlg blocks, helper fns, enum match.
///
/// Exercises entity declaration, @Speaker (Tier 1), $ match dialogue conditional,
/// -> transition, enum state machine, and mixed fn/dlg module structure.
/// Avoids $ choice due to known ChoiceOption lambda serialization bug.
/// Regression anchor for GOLD-02.
#[test]
fn test_dlg_quest_pattern() {
    run_golden_test("dlg_quest_pattern");
}
```

### Recommended GOLD-01 Source (`dlg_fn_mix.writ`)
```writ
// dlg_fn_mix: exercises fn/dlg interplay without $ choice.
// Tier 1 speaker (parameter) avoids Entity.getOrCreate<T> complexity.

// Helper fn called from dlg via $ code escape
fn compute_mood(energy: int) -> int {
    if energy > 50 { 1 } else { 0 }
}

// Dialogue that calls a helper fn, uses $ let, $ if, -> transition
dlg npc_greet(npc: Entity, player: Entity) {
    @npc Hello, traveler.
    $ let mood: int = compute_mood(80);
    $ if mood == 1 {
        @npc I feel great today!
    } else {
        @npc Been a long day.
    }
    -> npc_farewell(npc, player)
}

// Transition target dialogue
dlg npc_farewell(npc: Entity, player: Entity) {
    @npc Safe travels, friend.
}

// fn that triggers dialogue by direct function call
fn start_encounter(npc: Entity, player: Entity) {
    npc_greet(npc, player);
}
```

### Recommended GOLD-02 Source (`dlg_quest_pattern.writ`)
```writ
// dlg_quest_pattern: full quest pattern without $ choice (known serialization bug).
// Uses $ match for branching instead.

enum QuestState {
    Pending,
    Active,
    Done,
}

// Entity with properties (no component slots — keeps type-check surface minimal)
entity QuestGiver {
    name: string = "Elder",
    quest_state: QuestState = QuestState::Pending,
}

fn is_quest_active(state: QuestState) -> bool {
    match state {
        QuestState::Active => { true }
        QuestState::Pending => { false }
        QuestState::Done => { false }
    }
}

dlg quest_intro(giver: Entity, player: Entity) {
    @giver Greetings, adventurer.
    $ let active: bool = is_quest_active(QuestState::Pending);
    $ match active {
        true => {
            @giver A quest awaits you!
            -> quest_details(giver, player)
        }
        false => {
            @giver Come back later.
        }
    }
}

dlg quest_details(giver: Entity, player: Entity) {
    @giver Find the lost artifact.
    @player I accept the quest.
}

fn start_quest(giver: Entity, player: Entity) {
    ::log::info("Starting quest dialogue.");
    quest_intro(giver, player);
}
```

**Note**: Entity with no component slots simplifies the type-check surface. `QuestGiver.quest_state` is a property of enum type — exercises entity + enum pattern. Actual construction of `QuestGiver` (with `new QuestGiver {}`) is not needed for the dialogue tests since speakers are passed as `Entity` params.

### `say` Signature Fix (if needed)
```rust
// writ-compiler/src/check/env.rs — fix say to 2-param spec signature
// BEFORE:
("say", vec![("text", string_ty)], void_ty),
// AFTER:
("say", vec![("speaker", entity_ty), ("text", string_ty)], void_ty),

// writ-compiler/src/emit/collect/builtins.rs — fix sig blob
// BEFORE: 1 param: param_count=1, string, void return
("say", vec![0x01, 0x00, 0x04, 0x00]),
// AFTER: 2 params: param_count=2, entity(?), string, void return
// entity TypeRef tag TBD — check encoding spec §2.15.3
```

**IMPORTANT**: The entity TypeRef tag for `say`'s first parameter must be determined by looking at how `Entity` types are encoded in type blobs. Check `writ-compiler/src/emit/collect/encoding.rs` for the correct tag.

---

## State of the Art

| Old Approach | Current Approach | Status | Impact |
|--------------|------------------|--------|--------|
| `::say("text")` — 1-arg form | `say(speaker, text)` — 2-arg spec form | Mismatch (broken) | Must fix before dlg golden tests work |
| No dlg golden tests | dlg_fn_mix + dlg_quest_pattern | Adding in Phase 69 | First ever dlg blocks in golden suite |
| `$ choice` in dlg | Avoid `$ choice` | Known bug deferred | GOLD-01 and GOLD-02 both avoid `$ choice` |

**Deprecated/outdated:**
- `say(text: string)` (1-arg form): Not spec-compliant. Was used in all existing golden tests as a workaround.

---

## Open Questions

1. **`say_localized` speaker parameter**
   - What we know: `make_say_localized` generates `say_localized(speaker_ref, key, fallback)` — 3 args
   - Current `env.rs`: `("say_localized", vec![("key", string_ty), ("locale", string_ty)], void_ty)` — 2 params
   - What's unclear: Does `say_localized` also need a `speaker` first param fix?
   - Recommendation: Yes — fix to 3 params to match lowering. Verify against spec §1.27.4.

2. **Entity TypeRef tag for `say` sig blob**
   - What we know: The sig blob needs an entity type tag for the speaker parameter
   - What's unclear: What byte tag encodes an `Entity` reference type in the binary sig format
   - Recommendation: Check `writ-compiler/src/emit/collect/encoding.rs` for `TyKind::Entity` encoding

3. **Impact on `fn_log_say_choice.writ`**
   - What we know: It calls `::say("Test")` with 1 arg — will break after 2-param fix
   - What's unclear: Should it become `::say(null, "Test")` or be removed from the test?
   - Recommendation: Update to `::say(null, "Test")` — `null` coerces to `Entity?`. Re-bless the snapshot.

4. **`$ choice` serialization bug in GOLD-02**
   - What we know: `$ choice` → `ChoiceOption` with `fn() {}` lambda fails in multi-function modules
   - What's unclear: Exact trigger conditions — maybe a single-dlg file works?
   - Recommendation: Avoid `$ choice` entirely in both tests as decided in CONTEXT.md. Use `$ match`.

5. **Entity properties compilation**
   - What we know: Entity declarations compile through the lowering pipeline (lowering tests pass)
   - What's unclear: Whether entity properties with enum-type defaults (`quest_state: QuestState = QuestState::Pending`) compile through the type checker and emitter without errors
   - Recommendation: Start GOLD-02 with minimal entity (just string properties), then add enum fields iteratively.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + `cargo test` |
| Config file | `Cargo.toml` (workspace) |
| Quick run command | `cargo test -p writ-golden -- dlg_` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GOLD-01 | dlg_fn_mix compiles and matches blessed snapshot | golden | `cargo test -p writ-golden -- test_dlg_fn_mix` | No — Wave 0 |
| GOLD-02 | dlg_quest_pattern compiles and matches blessed snapshot | golden | `cargo test -p writ-golden -- test_dlg_quest_pattern` | No — Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-golden -- dlg_`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-golden/tests/golden/dlg_fn_mix.writ` — covers GOLD-01
- [ ] `writ-golden/tests/golden/dlg_quest_pattern.writ` — covers GOLD-02
- [ ] `writ-golden/tests/golden/dlg_fn_mix.writil` — blessed snapshot (generated by BLESS=1)
- [ ] `writ-golden/tests/golden/dlg_quest_pattern.writil` — blessed snapshot (generated by BLESS=1)
- [ ] `say` signature fix in `writ-compiler/src/check/env.rs` — prerequisite for dlg compilation
- [ ] `say` sig blob fix in `writ-compiler/src/emit/collect/builtins.rs` — prerequisite for dlg compilation
- [ ] Test registration (Section L) in `writ-golden/tests/golden_tests.rs`

---

## Sources

### Primary (HIGH confidence)
- `writ-golden/tests/golden_tests.rs` — Complete harness: compile_and_disassemble, run_golden_test, bless_golden, existing sections A-K
- `writ-compiler/src/lower/dialogue.rs` — Dialogue lowering: speaker resolution tiers, make_say/make_say_localized, lower_choice, lower_transition
- `writ-compiler/src/check/env.rs:256` — Type env dialogue sig table (1-arg say — confirmed mismatch)
- `writ-compiler/src/emit/collect/builtins.rs:77` — Emitter dialogue sig blobs (1-param say — confirmed mismatch)
- `language-spec/spec/28_27_standard_library_builtins.md §1.27.4` — Spec: say(speaker, text) — 2 params
- `language-spec/spec/15_14_dialogue_blocks_dlg.md` — Dialogue syntax reference
- `language-spec/spec/16_15_entities.md` — Entity syntax, lifecycle hooks, construction
- `writ-compiler/tests/lowering_tests.rs` — Lowering unit tests confirming dlg→fn lowering patterns
- `writ-golden/tests/golden/quest_system.writ` — Existing comprehensive test (no dlg blocks)
- `writ-golden/tests/golden/fn_log_say_choice.writ` — Existing test: ::say("text") 1-arg form

### Secondary (MEDIUM confidence)
- `.planning/STATE.md §Accumulated Context` — Choice serialization bug confirmed: "UnexpectedEof in multi-function modules"
- `writ-compiler/tests/snapshots/lowering_tests__dlg_say_without_key.snap` — Confirmed: lowering generates say(player, "text") with 2 args

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — fully verified by reading source files
- Architecture (dlg lowering): HIGH — read full dialogue.rs, confirmed against snapshots
- Critical blocker (say mismatch): HIGH — verified in env.rs, builtins.rs, and spec
- `$ choice` risk: HIGH — confirmed in STATE.md + CONTEXT.md
- Entity compilation: MEDIUM — lowering tests pass, full pipeline not directly verified for entities with enum fields
- Fix details (entity TypeRef byte tag): LOW — encoding.rs not yet read

**Research date:** 2026-03-18
**Valid until:** Stable (no external dependencies — all findings from project source)
