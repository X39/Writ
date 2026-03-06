# Phase 69: Dialogue/Function Golden Tests - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Add golden test files that exercise dialogue/function mix patterns. Two tests: (1) basic dialogue/function interplay and (2) full quest pattern with entity + dialogue + functions + choices. Both must compile, produce blessed `.writil` snapshots, and pass under `cargo test --workspace`.

</domain>

<decisions>
## Implementation Decisions

### Test file content — GOLD-01 (dlg_fn_mix)
- Mix `dlg` blocks and `fn` declarations in the same file
- A `dlg` block that calls a helper `fn` (e.g., compute a value, format a string)
- A `fn` that calls a dialogue function (dialogue transition via direct call)
- Exercise: `@Speaker` text lines, `$ let` code escapes, `-> transition` syntax, speaker parameter (Tier 1)
- Avoid `$ choice` in this test — focus on fn/dlg interplay without triggering the known `::choice` serialization bug

### Test file content — GOLD-02 (dlg_quest_pattern)
- Full quest scenario: entity declaration, `dlg` blocks with speaker interaction, helper `fn` declarations, enum for quest state, `$ choice` blocks
- Exercise: entity + `dlg` + `fn` + enum match + `$ choice` + `@Speaker` + `-> transition`
- **Risk**: `$ choice` lowers to `::choice([::ChoiceOption(..., fn() {}), ...])` which is known to cause serialization failure (UnexpectedEof) in multi-function modules. If this bug triggers, the test should be written to avoid `$ choice` and document the limitation. The planner/researcher should verify whether the bug applies before writing test content.

### File naming and registration
- GOLD-01: `writ-golden/tests/golden/dlg_fn_mix.writ` (follows `category_description` convention)
- GOLD-02: `writ-golden/tests/golden/dlg_quest_pattern.writ`
- Register both in `writ-golden/tests/golden_tests.rs` as new Section L: Dialogue golden tests
- Each gets a `#[test] fn test_dlg_fn_mix()` / `#[test] fn test_dlg_quest_pattern()` using `run_golden_test()`

### Snapshot workflow
- Write `.writ` source files
- Run `BLESS=1 cargo test -p writ-golden -- dlg_` to generate `.writil` blessed snapshots
- Verify both pass under `cargo test --workspace`

### Claude's Discretion
- Exact Writ source code content (specific variable names, enum variants, dialogue text)
- Number of functions/dialogues in each test file (enough to exercise the patterns)
- Whether to include `$ if` / `$ match` dialogue conditionals in addition to core patterns
- Doc comment style on test functions

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Golden test infrastructure
- `writ-golden/tests/golden_tests.rs` — Test harness: `compile_and_disassemble()`, `run_golden_test()`, test registration pattern
- `writ-golden/tests/golden/quest_system.writ` — Existing comprehensive test (uses `::say`/`::log` from `fn`, NOT `dlg` blocks)
- `writ-golden/tests/golden/fn_log_say_choice.writ` — Tests dialogue builtins from regular `fn` context

### Dialogue syntax (for writing test content)
- `language-spec/spec/10_dialogue.md` — Dialogue declaration syntax, `@Speaker`, `$ choice`, `-> transition`
- `writ-compiler/src/lower/dialogue.rs` — How `dlg` lowers to `fn` (speaker resolution, choice lowering)
- `writ-compiler/tests/lowering_tests.rs` — Unit tests showing `dlg` syntax examples (Tier 1/2 speakers, choices, transitions)

### Entity syntax (for GOLD-02)
- `language-spec/spec/11_entities.md` — Entity declaration syntax, component slots, lifecycle hooks

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `compile_and_disassemble()` in `golden_tests.rs`: Full pipeline (parse→lower→resolve→typecheck→emit_bodies→serialize→deserialize→disassemble). Used by all 31 existing golden tests.
- `run_golden_test(name)`: Reads `.writ`, compiles, compares against `.writil` (or blesses with `BLESS=1`). Direct reuse — no changes needed.

### Established Patterns
- Test naming: `test_{category}_{description}` function calling `run_golden_test("{category}_{description}")`
- File naming: `{category}_{description}.writ` with matching `.writil` blessed snapshot
- Test organization: Sections A-K in `golden_tests.rs`, each with doc comments
- Dialogue lowering: `dlg name(params) { body }` → `fn name(params) { hoisted_lets + lowered_body }` — dialogue blocks compile through the same pipeline as functions

### Integration Points
- New `.writ` files go in `writ-golden/tests/golden/`
- New test functions go in `writ-golden/tests/golden_tests.rs` (Section L)
- `.writil` snapshot files generated via `BLESS=1` env var

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The test content should be representative of real game scripting patterns (dialogue trees, quest logic, NPC interactions).

</specifics>

<deferred>
## Deferred Ideas

- Fix `::choice` with `fn() {}` lambda serialization bug (UnexpectedEof in multi-function modules) — separate milestone, carried as known tech debt
- Entity golden tests (standalone entity without dialogue) — could be a future test addition
- Locale override golden tests (`[Locale("ja")] dlg`) — future quality phase

</deferred>

---

*Phase: 69-dialogue-function-golden-tests*
*Context gathered: 2026-03-18*
