# Phase 12: Lowering — Dialogue and Localization - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix 5 dialogue lowering bugs so localization keys include namespace prefixes, interpolation slot identities are preserved in loc content strings, choice label keys are emitted (not discarded), say/say_localized dispatch matches the spec, and speaker scope is isolated across branching constructs. All behaviors are defined by spec v0.4 sections 13, 25, and 28.

</domain>

<decisions>
## Implementation Decisions

### Adhere strictly to spec v0.4

All 5 requirements (DLG-01 through DLG-05) are fully specified in the language spec. No user-facing design decisions — implementation follows spec exactly.

### DLG-01: Namespace in loc keys
- Spec §25.2.1: FNV-1a hash input includes `namespace` as "the fully qualified namespace of the `dlg`"
- Current bug: `DlgLowerState.namespace` is hardcoded to `String::new()` (dialogue.rs:110)
- Fix: Thread actual namespace context from `LoweringContext` into `DlgLowerState`
- `LoweringContext` needs a `current_namespace` field that gets pushed/popped as namespace declarations are encountered during lowering
- For multi-segment namespaces (`namespace a::b;`), join segments with `::` to match Writ syntax in hash input

### DLG-02: Interpolation slot identity preservation
- Spec §25.2.1: content field has "interpolation slots preserved literally (e.g., `Hey, {name}.`)"
- Current bug: `raw_text_content()` replaces ALL `DlgTextSegment::Expr` with generic `{expr}` placeholder (dialogue.rs:428)
- Fix: Preserve original expression text. For `{name}` → `{name}`, for `{player.name}` → `{player.name}`
- Implementation detail: `DlgTextSegment::Expr` contains a CST `Expr` — will need to reconstruct source text from span or carry raw text through CST

### DLG-03: Choice label key emission
- Spec §25.3.3 CSV output shows choice labels have keys with Context=`choice`
- Current bug: Key is computed for collision detection then discarded — `let _ = key;` (dialogue.rs:545)
- Fix: Emit the key in the lowered output so it's available for CSV export and runtime lookup
- The key should appear in the `Option()` call or as a binding — exact AST shape to be determined by planner

### DLG-04: say vs say_localized dispatch
- Spec §28.1: `@speaker Text.` lowers to `say(speaker, "Text.")` — NOT `say_localized`
- Spec §28.4: `say_localized()` used only "when localization is active" (manual `#key` present)
- Spec §25.5: Runtime `say()` computes its own key for string table lookup at runtime
- Current bug: `make_say_localized()` called unconditionally for ALL lines (dialogue.rs:251, 288)
- Fix: No `#key` → emit `say(speaker, text)`. Has `#key` → emit `say_localized(speaker, key, fallback)`
- Auto-generated FNV keys are for CSV export tooling only, not for the lowered AST

### DLG-05: Speaker scope isolation in branches
- Spec: Speaker set inside one `$ if`/`$ match` branch must not leak into sibling branches or subsequent lines
- Current bug: `lower_dlg_if` and `lower_dlg_match` do NOT save/restore speaker scope (dialogue.rs:610-689)
- `lower_choice` already implements the correct pattern (save depth before, restore after — dialogue.rs:532, 551-553)
- Fix: Apply same save/restore pattern to `lower_dlg_if` (each branch) and `lower_dlg_match` (each arm)

### Claude's Discretion
- Internal AST shape for choice key emission (argument to Option() vs separate binding)
- How to reconstruct interpolation slot text from CST expressions (span-based vs carry raw text)
- Namespace representation for multi-segment paths in hash input string

</decisions>

<specifics>
## Specific Ideas

No specific requirements — implementation follows spec v0.4 exactly as written in sections 13 (Dialogue Blocks), 25 (Localization), and 28 (Lowering Reference).

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `lower_choice` speaker scope save/restore pattern (dialogue.rs:532-553): Exact pattern needed for DLG-05 fix in `lower_dlg_if` and `lower_dlg_match`
- `LoweringContext` speaker stack API (`push_speaker`, `pop_speaker`, `speaker_stack_depth`): Already built, just needs namespace tracking added
- `DlgLowerState` struct (dialogue.rs:21-29): Already has `namespace` field, just hardcoded to empty string
- `fnv1a_32()` hash function (dialogue.rs:406-415): Correct implementation, no changes needed
- `compute_or_use_loc_key()` (dialogue.rs:360-398): Already handles manual vs auto key logic

### Established Patterns
- Error accumulation: `ctx.emit_error()` — never halts pipeline
- Lowering context threading: `&mut LoweringContext` passed through all lowering functions
- Namespace handling: `lower_namespace()` in mod.rs recurses into child items but doesn't currently track namespace state

### Integration Points
- `LoweringContext` (context.rs): Needs new `current_namespace` field with push/pop for namespace blocks and set for declarative namespaces
- `lower_namespace()` in mod.rs: Must set/push namespace context before recursing into child `Item::Dlg` lowering
- `raw_text_content()` (dialogue.rs:423-432): Central fix point for DLG-02
- `make_say_localized()` (dialogue.rs:493-515): Needs companion `make_say()` function for DLG-04
- `lower_dlg_lines()` SpeakerLine/TextLine arms (dialogue.rs:238-292): Dispatch point for say vs say_localized

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 12-lowering-dialogue-and-localization*
*Context gathered: 2026-03-01*
