# Phase 42: ChoiceOption Rename - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Rename the dialogue choice option constructor from `Option` to `ChoiceOption` across all four layers — spec text, lowering emit site, virtual module registration, and resolver prelude — resolving the naming conflict with `Option<T>`. No new behavior is added; this is a pure rename. `Option<T>` (nullable wrapper type) is out of scope and must not be touched.

</domain>

<decisions>
## Implementation Decisions

### Rename scope (atomic across four layers)
- **Spec** (`29_28_lowering_reference.md`): Two occurrences — `Option("Good!", fn() { ... })` → `ChoiceOption("Good!", fn() { ... })` (lines 53 and 57). No other spec file uses `Option(...)` as a choice constructor.
- **Lowering** (`writ-compiler/src/lower/dialogue.rs` line ~657): `name: "Option".to_string()` → `name: "ChoiceOption".to_string()` — this is the single emit site for the choice option AST node.
- **Virtual module** (`writ-runtime/src/virtual_module.rs`): Add a `ChoiceOption` TypeDef or ExternDef registration (the current `Option` TypeDef at line 183 is `Option<T>` with 1 type param — it must not be renamed). The choice option constructor needs its own registration.
- **Prelude** (`writ-compiler/src/resolve/prelude.rs`): The existing `"Option"` in `PRELUDE_TYPE_NAMES` is `Option<T>` — keep it. Add `"ChoiceOption"` as a recognized name so the resolver doesn't reject it.

### Snapshot updates
- Insta snapshots in `writ-compiler/tests/snapshots/` that contain `name: "Option"` for choice option calls must be updated. Affected snapshots: `dlg_choice_basic`, `dlg_choice_label_key_emitted`, `dlg_choice_speaker_scope_isolation`, `integration_all_constructs`. Update via `cargo insta test --accept` (the established project pattern).

### Option<T> isolation
- `"Option"` references in `check/env.rs`, `check/infer.rs`, `emit/collect.rs`, `lower/expr.rs`, `lower/optional.rs`, `lower/desugar.rs`, `resolve/prelude.rs`, and `virtual_module.rs` that refer to the nullable wrapper type `Option<T>` must NOT be renamed. The distinction is context: choice constructor → rename, nullable type → leave alone.

### Success verification
- After rename, a Writ script using `$ choice { "Good!" { ... } "Bad" { ... } }` must compile without error and the emitted IL must contain `CALL_EXTERN` referencing a `ChoiceOption` ExternDef token.
- `fn_log_say_choice.writil` golden test (which uses `::choice()` without options) must still pass — no `ChoiceOption` references in that fixture.

### Claude's Discretion
- Exact TypeDef vs ExternDef representation for `ChoiceOption` in virtual module (TypeDef with kind=0, or ExternDef — whichever matches how the existing `Option(...)` call was handled before)
- Whether `ChoiceOption` goes into `PRELUDE_TYPE_NAMES` or a separate list
- Whether to add a new golden test fixture for the full choice path post-rename (recommended, but plan can decide)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-compiler/src/lower/dialogue.rs`: Single emit site at the `lower_choice` function (~line 657). Change `name: "Option"` to `name: "ChoiceOption"` here.
- `writ-runtime/src/virtual_module.rs`: Pattern for adding TypeDefs is established (`add_type_def("Name", "writ", type_params, kind)`). ChoiceOption needs 0 type params (it is not generic).
- `writ-compiler/src/resolve/prelude.rs`: `PRELUDE_TYPE_NAMES: &[&str]` — add `"ChoiceOption"` here.

### Established Patterns
- Spec update: splatted file `29_28_lowering_reference.md` — two string replacements only.
- Snapshot blessing: `cargo insta test --accept` — project pattern for re-blessing after intentional name changes.
- Rename safety: grep `"Option"` across source to distinguish `Option<T>` context (has `<`, appears in type position, `Generic { name: "Option"`, `args:`) vs choice option context (appears as a bare call, `name: "Option"` in a `Call` or `ExternCall` AST node).

### Integration Points
- Lowering → Resolver: `lower/dialogue.rs` emits AST node with name `"ChoiceOption"`. Resolver must recognize it (via prelude or virtual module lookup).
- Resolver → Emitter: After resolution, `emit/` code emits `CALL_EXTERN` for `ChoiceOption`. The virtual module must register the def so the token is valid.
- Snapshot tests: `writ-compiler/tests/lowering_tests.rs::dlg_choice_basic` and related tests will auto-fail and need `--accept` reblessing.

</code_context>

<specifics>
## Specific Ideas

- From `language-spec/todos.md`: The working form was `::Option("Good!", fn() { ... })` — after rename this becomes `::ChoiceOption("Good!", fn() { ... })`. The full working script shown in todos.md can serve as a reference for the integration test.
- Phase 41 simplified `fn_log_say_choice.writ` to avoid `Option(...)` calls — the golden test is not affected by this rename and must continue passing.

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope.

</deferred>

---

*Phase: 42-choiceoption-rename*
*Context gathered: 2026-03-06*
