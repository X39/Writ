---
phase: quick-2
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - writ-golden/tests/golden/quest_system.writ
  - writ-golden/tests/golden/quest_system.writil
  - writ-golden/tests/golden_tests.rs
autonomous: true
requirements: [QUEST-GOLDEN]

must_haves:
  truths:
    - "quest_system.writ compiles through the full pipeline without errors"
    - "quest_system golden test passes (compile -> disassemble -> compare blessed output)"
    - "The .writ source demonstrates enums, functions, control flow, arrays, Option, match, defer, atomic, dialogue builtins, and log calls in a cohesive quest-system scenario"
  artifacts:
    - path: "writ-golden/tests/golden/quest_system.writ"
      provides: "Comprehensive quest system demo source file"
      min_lines: 80
    - path: "writ-golden/tests/golden/quest_system.writil"
      provides: "Blessed IL output for the quest system golden test"
    - path: "writ-golden/tests/golden_tests.rs"
      provides: "Test harness entry point for quest_system golden test"
      contains: "test_quest_system"
  key_links:
    - from: "writ-golden/tests/golden_tests.rs"
      to: "writ-golden/tests/golden/quest_system.writ"
      via: "run_golden_test(\"quest_system\")"
      pattern: "run_golden_test.*quest_system"
---

<objective>
Create a comprehensive quest system golden test file that serves as both a compiler regression test and a language showcase/demo. The file models a quest system using only features the compiler currently supports (no struct construction or entity definitions, which still fail codegen).

Purpose: Provide a single large golden test that exercises many Writ features in combination, acting as an integration-level regression test and a readable demo of the language.
Output: quest_system.writ + quest_system.writil + test harness entry
</objective>

<execution_context>
@C:/Users/msili/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/msili/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@writ-golden/tests/golden_tests.rs (test harness — follow Section pattern for new test)
@writ-golden/tests/golden/type_enum_match.writ (enum + match pattern)
@writ-golden/tests/golden/fn_log_say_choice.writ (dialogue builtins pattern)
@writ-golden/tests/golden/adv_option_match.writ (Option pattern)
@writ-golden/tests/golden/adv_defer.writ (defer pattern)
@writ-golden/tests/golden/adv_atomic.writ (atomic pattern)
@writ-golden/tests/golden/ctrl_for_array.writ (for-in pattern)
@writ-golden/tests/golden/ctrl_break_continue.writ (break/continue pattern)
@writ-golden/tests/golden/fn_recursion.writ (recursion pattern)
@writ-golden/tests/golden/fn_multi_return.writ (early return pattern)

<interfaces>
<!-- From golden_tests.rs -->
pub fn run_golden_test(name: &str);   // reads {name}.writ, compiles, compares to {name}.writil
pub fn compile_and_disassemble(src: &str) -> String;
// BLESS=1 env var causes run_golden_test to write .writil instead of comparing
// Test naming convention: #[test] fn test_{name}() { run_golden_test("{name}"); }
</interfaces>

CRITICAL CONSTRAINTS — Features that do NOT compile (must avoid):
- struct construction: `new StructName { field: value }` — codegen aborts with error nodes
- struct field access: `instance.field` — same failure
- entity definitions: `entity Name { ... }` — not supported in codegen
- dialogue blocks: `dlg name(...) { ... }` — not supported in codegen
- string interpolation: `$"text {expr}"` — may not be supported in codegen
- impl blocks: `impl Contract for Type { ... }` — not supported in codegen

Features that DO compile (use these):
- Enums with variants (unit variants): `enum Name { A, B, C }` + `match` on them
- Functions: params, return types, recursion, early return, tail expressions
- Variables: let, let mut, reassignment, shadowing
- Expressions: int/float arithmetic, comparisons, booleans, string concat (+)
- Control flow: if/else, while, for-in over arrays, break, continue
- Arrays: init with `[a, b, c]`, index, for-in iteration
- Option: `Option::Some(v)`, `Option::None`, null literal, `match` on Option
- defer blocks, atomic blocks
- Builtins: `::log::info(string)`, `::say(string)`, `::choice([...])`, `::ChoiceOption(string, string, fn() {})`
- Global mut variables
- Constants (const)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create quest_system.writ source file</name>
  <files>writ-golden/tests/golden/quest_system.writ</files>
  <action>
Create `writ-golden/tests/golden/quest_system.writ` — a comprehensive quest system demo that combines many Writ language features into a cohesive, readable scenario. The file should be 80-130 lines and model a quest system using the features that actually compile.

Design the quest system around these language features:

1. **Enum: QuestStatus** with variants `NotStarted`, `Active`, `Completed`, `Failed` — used with match expressions.

2. **Enum: QuestType** with variants `MainStory`, `SideQuest`, `Daily` — for categorization.

3. **Constants:** `const MAX_QUESTS: int = 10;` and `const XP_MULTIPLIER: int = 2;`

4. **Global mut state:** `global mut active_quest_count: int = 0;` and `global mut total_xp: int = 0;`

5. **Functions showcasing different features:**
   - `fn calculate_reward(base_xp: int, quest_type: QuestType) -> int` — match on QuestType to apply multipliers, uses arithmetic
   - `fn is_quest_available(status: QuestStatus) -> bool` — match returning bool
   - `fn find_first_active(statuses: QuestStatus[]) -> Option<int>` — for-in loop with early return via Option, uses break/index tracking
   - `fn complete_quest(status: QuestStatus, base_xp: int, quest_type: QuestType) -> QuestStatus` — if/else + match, guard logic, calls calculate_reward, updates globals in atomic block
   - `fn process_quest_log(statuses: QuestStatus[]) -> int` — for-in loop counting completed quests, uses mut variable + if
   - `fn announce_quest(quest_type: QuestType)` — uses ::say and ::log::info for dialogue builtins
   - `fn present_quest_choice()` — uses ::choice and ::ChoiceOption builtins to show quest selection dialogue

6. **fn main()** — orchestrates the demo:
   - Declares arrays of QuestStatus and QuestType values
   - Calls the various functions, captures results in let bindings
   - Uses defer for cleanup logging
   - Exercises atomic block for global state updates
   - Calls announce_quest and present_quest_choice for dialogue coverage
   - Uses Option match on find_first_active result

IMPORTANT: Do NOT use struct construction, entity definitions, dialogue blocks, impl blocks, or string interpolation. All string values must be plain string literals concatenated with `+` if needed. Use only features confirmed to compile in the existing golden test suite.

Follow the exact syntax patterns from existing golden tests:
- `::log::info("text")` for logging (root-qualified)
- `::say("text")` for dialogue say
- `::choice([...])` and `::ChoiceOption("label", "key", fn() {})` for choices
- `Option::Some(value)` and `Option::None` for optionals
- `match expr { Enum::Variant => { body } }` for match arms
- `let x: Type = expr;` for bindings (explicit types)
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-golden -- test_harness_pass 2>&1 | grep -c "ok"</automated>
  </verify>
  <done>quest_system.writ exists with 80+ lines covering enums, functions, match, control flow, arrays, Option, defer, atomic, and dialogue builtins in a cohesive quest-system theme</done>
</task>

<task type="auto">
  <name>Task 2: Bless the golden output and register the test</name>
  <files>writ-golden/tests/golden/quest_system.writil, writ-golden/tests/golden_tests.rs</files>
  <action>
Step A: Add the test function to `writ-golden/tests/golden_tests.rs`.

Add a new section at the end of the file (after Section J), following the existing pattern:

```rust
// --- Section K: Comprehensive golden tests ---------------------------------

/// Golden test: comprehensive quest system exercising enums, functions, match,
/// control flow, arrays, Option, defer, atomic, and dialogue builtins.
///
/// Integration-level regression test — a single large file combining many
/// language features in a realistic game-scripting scenario.
#[test]
fn test_quest_system() {
    run_golden_test("quest_system");
}
```

Step B: Bless the golden output by running with BLESS=1:

```bash
BLESS=1 cargo test -p writ-golden -- test_quest_system
```

This creates `quest_system.writil` with the compiler's actual output.

Step C: Verify the blessed output was created and the test passes without BLESS:

```bash
cargo test -p writ-golden -- test_quest_system
```

Step D: Run ALL golden tests to confirm no regressions:

```bash
cargo test -p writ-golden
```

If the quest_system.writ file from Task 1 fails to compile, diagnose the error and fix the .writ source. Common issues:
- If a feature doesn't compile, remove that specific usage and replace with a simpler alternative
- Ensure all enum variants used in match are from enums defined in the same file
- Ensure all function signatures match their call sites
- Ensure all variables are declared before use with explicit types

The test MUST pass. Iterate on the .writ source until compilation succeeds, then bless.
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-golden 2>&1 | tail -5</automated>
  </verify>
  <done>quest_system.writil exists with blessed IL output, test_quest_system passes, all 30 golden tests pass (29 + quest_system, 1 ignored struct_new), zero regressions</done>
</task>

</tasks>

<verification>
- `cargo test -p writ-golden` passes with 30 passed, 0 failed, 1 ignored
- `quest_system.writ` is 80+ lines and exercises at least 8 distinct language features
- `quest_system.writil` exists and contains `.module` directive
- `golden_tests.rs` contains `test_quest_system` function
</verification>

<success_criteria>
- The quest system golden test compiles, blesses, and passes the golden comparison
- All existing 29 golden tests continue to pass (zero regressions)
- The .writ source file reads as a cohesive quest-system demo, not a disconnected feature grab-bag
- At least these features are exercised in combination: enum definition, enum match, functions with params/returns, if/else, while or for-in, arrays, Option (Some/None/match), defer, atomic, dialogue builtins (say/choice/log)
</success_criteria>

<output>
After completion, create `.planning/quick/2-create-comprehensive-quest-system-golden/2-SUMMARY.md`
</output>
