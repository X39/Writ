use writ_compiler::{lower, Ast, LoweringError};

// =========================================================
// Test helpers
// =========================================================

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

// =========================================================
// R3 — Optional Sugar Lowering
//
// T? in type position → Generic { name: "Option", args: [Named { name: "T" }] }
// null literal         → Path { segments: ["Option", "None"] }
// =========================================================

/// T? in parameter position → Option<T>
#[test]
fn optional_param_type() {
    let ast = lower_src("fn greet(name: string?) {}");
    insta::assert_debug_snapshot!(ast);
}

/// T? in return type position → Option<T>
#[test]
fn optional_return_type() {
    let ast = lower_src("fn find() -> int? { return null; }");
    insta::assert_debug_snapshot!(ast);
}

/// null literal → Option::None path expression
#[test]
fn null_literal_to_option_none() {
    let ast = lower_src("fn f() { let x: string? = null; }");
    insta::assert_debug_snapshot!(ast);
}

/// Nested nullable: List<string?>? → Option<Generic { name: "List", args: [Option<string>] }>
#[test]
fn nested_optional_type() {
    let ast = lower_src("fn f(x: List<string?>?) {}");
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R4 — Formattable String Lowering
//
// $"Hello {name}!" → "Hello " + name.into<string>() + "!"
// (Binary Add chain with GenericCall wrappers for interpolated segments)
// =========================================================

/// Simple interpolation: $"Hello {name}!" → Binary Add chain
#[test]
fn fmt_string_simple_interpolation() {
    let ast = lower_src(r#"fn f(name: string) { let x = $"Hello {name}!"; }"#);
    insta::assert_debug_snapshot!(ast);
}

/// No interpolation: $"plain string" → StringLit (single text segment, no concat)
#[test]
fn fmt_string_no_interpolation() {
    let ast = lower_src(r#"fn f() { let x = $"plain string"; }"#);
    insta::assert_debug_snapshot!(ast);
}

/// Multiple interpolations: $"a={a} b={b}" → chained Binary Add
#[test]
fn fmt_string_multiple_segments() {
    let ast = lower_src(r#"fn f(a: int, b: int) { let x = $"a={a} b={b}"; }"#);
    insta::assert_debug_snapshot!(ast);
}

/// Escaped braces: $"{{literal}}" — documents lexer's handling of {{ }}
#[test]
fn fmt_string_escaped_braces() {
    let ast = lower_src(r#"fn f() { let x = $"{{literal}}"; }"#);
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R5 — Compound Assignment Desugaring
//
// x += n → Assign { target: x, value: Binary { op: Add, left: x, right: n } }
// x = n  → Assign { target: x, value: n }  (no Binary wrapper for plain =)
//
// Note: The Writ parser does not support `mut` in function parameter position.
// Tests use `let mut` local variables to get a mutable binding.
// =========================================================

/// x += 1 → Assign { target: x, value: Binary { op: Add, left: x, right: 1 } }
#[test]
fn compound_add_assign() {
    let ast = lower_src("fn f() { let mut x: int = 0; x += 1; }");
    insta::assert_debug_snapshot!(ast);
}

/// x -= 2 → Assign { target: x, value: Binary { op: Sub, left: x, right: 2 } }
#[test]
fn compound_sub_assign() {
    let ast = lower_src("fn f() { let mut x: int = 0; x -= 2; }");
    insta::assert_debug_snapshot!(ast);
}

/// x *= 3 → Assign { target: x, value: Binary { op: Mul, left: x, right: 3 } }
#[test]
fn compound_mul_assign() {
    let ast = lower_src("fn f() { let mut x: int = 0; x *= 3; }");
    insta::assert_debug_snapshot!(ast);
}

/// x /= 4 → Assign { target: x, value: Binary { op: Div, left: x, right: 4 } }
#[test]
fn compound_div_assign() {
    let ast = lower_src("fn f() { let mut x: int = 0; x /= 4; }");
    insta::assert_debug_snapshot!(ast);
}

/// x %= 5 → Assign { target: x, value: Binary { op: Mod, left: x, right: 5 } }
#[test]
fn compound_mod_assign() {
    let ast = lower_src("fn f() { let mut x: int = 0; x %= 5; }");
    insta::assert_debug_snapshot!(ast);
}

/// Plain = does NOT produce a Binary wrapper
#[test]
fn plain_assign_passthrough() {
    let ast = lower_src("fn f() { let mut x: int = 0; x = 0; }");
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R6 — Operator Lowering
//
// operator SYMBOL (...) -> T { ... } inside impl → standalone impl Contract for Self
// Derived operators auto-generated from Eq and Ord
// =========================================================

/// operator + inside bare impl → impl Add<vec2, vec2> for vec2 { fn add(...) }
/// No empty base impl emitted (operator-only impl, no fn members, no contract)
#[test]
fn operator_binary_add_desugars_to_add_contract() {
    let ast = lower_src("impl vec2 { operator +(other: vec2) -> vec2 { vec2(0, 0) } }");
    insta::assert_debug_snapshot!(ast);
}

/// operator - with one param → impl Sub<vec2, vec2> for vec2 { fn sub(...) }
/// Confirms binary Sub (not unary Neg) when param is present
#[test]
fn operator_binary_sub_desugars_to_sub_contract() {
    let ast = lower_src("impl vec2 { operator -(other: vec2) -> vec2 { vec2(0, 0) } }");
    insta::assert_debug_snapshot!(ast);
}

/// operator -() with zero params → impl Neg<vec2> for vec2 { fn neg() -> vec2 { ... } }
/// CRITICAL: Must produce "Neg", not "Sub"
#[test]
fn operator_unary_neg_desugars_to_neg_contract() {
    let ast = lower_src("impl vec2 { operator -() -> vec2 { vec2(0, 0) } }");
    insta::assert_debug_snapshot!(ast);
}

/// operator !() → impl Not<MyBool> for MyBool { fn not() -> MyBool { ... } }
#[test]
fn operator_unary_not_desugars_to_not_contract() {
    let ast = lower_src("impl MyBool { operator !() -> MyBool { MyBool(false) } }");
    insta::assert_debug_snapshot!(ast);
}

/// operator == → two impl blocks: Eq + auto-derived Ne
#[test]
fn operator_eq_desugars_with_derived_ne() {
    let ast = lower_src("impl vec2 { operator ==(other: vec2) -> bool { true } }");
    insta::assert_debug_snapshot!(ast);
}

/// operator < → two impl blocks: Ord + auto-derived Gt
#[test]
fn operator_ord_desugars_with_derived_gt() {
    let ast = lower_src("impl MyNum { operator <(other: MyNum) -> bool { false } }");
    insta::assert_debug_snapshot!(ast);
}

/// operator == and operator < together → six impl blocks total:
/// Eq, Ord, Ne (derived), Gt (derived), LtEq (derived), GtEq (derived)
#[test]
fn operator_eq_and_ord_derives_all_four() {
    let ast = lower_src(
        "impl MyNum { operator ==(other: MyNum) -> bool { true } operator <(other: MyNum) -> bool { false } }",
    );
    insta::assert_debug_snapshot!(ast);
}

/// operator [] → impl Index<int, string> for MyList { fn index(idx: int) -> string { ... } }
#[test]
fn operator_index_desugars_to_index_contract() {
    let ast = lower_src("impl MyList { operator [](idx: int) -> string { string() } }");
    insta::assert_debug_snapshot!(ast);
}

/// operator []= → impl IndexSet<int, string> for MyList { fn index_set(idx: int, val: string) { } }
#[test]
fn operator_index_set_desugars_to_index_mut_contract() {
    let ast = lower_src("impl MyList { operator []=(idx: int, val: string) { } }");
    insta::assert_debug_snapshot!(ast);
}

/// Mixed fn and operator members → two impl blocks:
/// base impl with fn member + operator contract impl
#[test]
fn impl_mixed_fn_and_op_members() {
    let ast = lower_src(
        "impl vec2 { fn length() -> float { 0.0 } operator +(other: vec2) -> vec2 { vec2(0, 0) } }",
    );
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R7 — Concurrency Pass-Through
//
// spawn, join, cancel, defer, detached each map 1:1 to their
// AstExpr variant with span preserved and no semantic transformation.
// =========================================================

/// spawn doWork() → AstExpr::Spawn { expr: AstExpr::Call { callee: Ident("doWork") } }
#[test]
fn concurrency_spawn_passthrough() {
    let ast = lower_src("fn f() { spawn doWork(); }");
    insta::assert_debug_snapshot!(ast);
}

/// join h → AstExpr::Join { expr: AstExpr::Ident { name: "h" } }
#[test]
fn concurrency_join_passthrough() {
    let ast = lower_src("fn f(h: Handle) { join h; }");
    insta::assert_debug_snapshot!(ast);
}

/// cancel h → AstExpr::Cancel { expr: AstExpr::Ident { name: "h" } }
#[test]
fn concurrency_cancel_passthrough() {
    let ast = lower_src("fn f(h: Handle) { cancel h; }");
    insta::assert_debug_snapshot!(ast);
}

/// defer { cleanup(); } → AstExpr::Defer { expr: AstExpr::Block { ... } }
#[test]
fn concurrency_defer_passthrough() {
    let ast = lower_src("fn f() { defer { cleanup(); } }");
    insta::assert_debug_snapshot!(ast);
}

/// spawn detached doWork() → AstExpr::SpawnDetached { expr: AstExpr::Call { ... } }
#[test]
fn concurrency_detached_spawn_passthrough() {
    let ast = lower_src("fn f() { spawn detached doWork(); }");
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R8 — Dialogue Lowering
//
// dlg name(params) { body } → fn name(params) { hoisted_lets + lowered_body }
// @Speaker text → say_localized(speaker_ref, key, fallback)
// Speaker resolution: Tier 1 (param), Tier 2 (singleton → getOrCreate)
// $ choice / $ if / $ match → choice([...]) / if cond {...} / match expr {...}
// -> target → return target()
// =========================================================

/// Tier 1: @player where player is a dlg param → say_localized(player, key, text)
/// No Entity.getOrCreate hoisting — param used directly
#[test]
fn dlg_speaker_param_tier1() {
    let ast = lower_src("dlg greet(player: Entity) { @player Hello there. }");
    insta::assert_debug_snapshot!(ast);
}

/// Tier 2: @Narrator (not a param) → hoisted let _narrator = Entity.getOrCreate<Narrator>();
/// say_localized uses _narrator reference
#[test]
fn dlg_speaker_singleton_tier2() {
    let ast = lower_src("dlg intro() { @Narrator Welcome to the game. }");
    insta::assert_debug_snapshot!(ast);
}

/// @Speaker standalone sets active speaker; TextLine uses it
#[test]
fn dlg_speaker_tag_sets_active() {
    let ast = lower_src("dlg intro() { @Narrator\nHello.\nHow are you? }");
    insta::assert_debug_snapshot!(ast);
}

/// {name} in dialogue text → Binary Add concat chain (same as formattable string)
#[test]
fn dlg_text_interpolation() {
    let ast = lower_src("dlg greet(player: Entity) { @player Hey, {player}! }");
    insta::assert_debug_snapshot!(ast);
}

/// $ statement; → lowered as regular statement
#[test]
fn dlg_code_escape_statement() {
    let ast = lower_src("dlg example() { @Narrator Hello.\n$ let x: int = 42; }");
    insta::assert_debug_snapshot!(ast);
}

/// $ choice { "A" { text } "B" { text } } → choice([Option("A", fn() {...}), Option("B", fn() {...})])
#[test]
fn dlg_choice_basic() {
    let ast = lower_src(r#"dlg ask() { @Narrator What do you think? $ choice { "Good" { @Narrator Great! } "Bad" { @Narrator Sorry. } } }"#);
    insta::assert_debug_snapshot!(ast);
}

/// $ if cond { dialogue } else { dialogue } → if/else with say_localized in branches
#[test]
fn dlg_conditional_if() {
    let ast = lower_src("dlg check(flag: bool) { @Narrator\n$ if flag { Yes! } else { No! } }");
    insta::assert_debug_snapshot!(ast);
}

/// -> target at end of block → AstStmt::Return { Call { target() } }
#[test]
fn dlg_transition_at_end() {
    let ast = lower_src("dlg intro() { @Narrator Hello.\n-> farewell }");
    insta::assert_debug_snapshot!(ast);
}

/// -> target(arg) → return target(arg)
#[test]
fn dlg_transition_with_args() {
    let ast = lower_src("dlg intro(name: string) { @Narrator Hello.\n-> farewell(name) }");
    insta::assert_debug_snapshot!(ast);
}

/// Multiple @Narrator and @Player lines → both hoisted at top, not repeated
#[test]
fn dlg_multiple_speakers_hoisting() {
    let ast = lower_src("dlg chat() { @Narrator Hello.\n@Player Hi back.\n@Narrator How are you? }");
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R9 — Localization Key Generation
//
// Auto-keys: FNV-1a 32-bit → 8-char hex; identical lines get distinct keys via occurrence_index
// Manual #key overrides replace auto-generated keys
// =========================================================

/// Auto-generated loc keys are 8-char lowercase hex FNV-1a hashes
/// Two different lines produce different keys
#[test]
fn dlg_loc_key_is_8char_hex() {
    let ast = lower_src("dlg intro() { @Narrator Hello.\n@Narrator Goodbye. }");
    insta::assert_debug_snapshot!(ast);
}

/// Two identical-text lines in same dlg produce distinct keys (occurrence_index differs)
#[test]
fn dlg_loc_key_distinct_for_duplicate_text() {
    let ast = lower_src("dlg annoying() { @Guard Move along.\n@Guard Move along. }");
    insta::assert_debug_snapshot!(ast);
}

/// Manual #key override replaces auto-generated key in say_localized output
#[test]
fn dlg_loc_key_manual_override() {
    // Parser syntax: @Speaker text #key_name (trailing Hash + Ident on same line)
    let ast = lower_src("dlg intro() { @Narrator Welcome. #greeting }");
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R10 — Localization Key Collision Detection
//
// Duplicate #key within a dlg → LoweringError::DuplicateLocKey
// =========================================================

/// Duplicate #key within same dlg → LoweringError::DuplicateLocKey with both spans
#[test]
fn dlg_loc_key_duplicate_collision() {
    let (ast, errors) = lower_src_with_errors("dlg intro() { @Narrator Hello. #greet\n@Narrator Bye. #greet }");
    insta::assert_debug_snapshot!((ast, errors));
}

// =========================================================
// R11 — Dialogue Transition Validation
//
// -> must be last statement in its block
// -> before end → LoweringError::NonTerminalTransition
// =========================================================

/// -> before end of block → LoweringError::NonTerminalTransition + lowering continues
#[test]
fn dlg_non_terminal_transition_error() {
    let (ast, errors) = lower_src_with_errors("dlg intro() { -> farewell\n@Narrator This should not be here. }");
    insta::assert_debug_snapshot!((ast, errors));
}

/// TextLine without any prior @Speaker → LoweringError::UnknownSpeaker
#[test]
fn dlg_text_without_speaker_error() {
    let (ast, errors) = lower_src_with_errors("dlg orphan() { Hello world. }");
    insta::assert_debug_snapshot!((ast, errors));
}

/// @Speaker in one choice arm does NOT affect sibling arms
/// Outer @Narrator is a SpeakerTag (no text before the $ sigil) → pushed to stack.
/// Arm A: @Player (SpeakerLine) overrides for that arm's text.
/// Arm B: Me too. as TextLine → stack restored to Narrator (no Player leakage).
///
/// Note: whitespace (including newlines) is filtered as trivia — dialogue text parsing
/// is token-based, not line-based. @Speaker followed by non-sigil tokens = SpeakerLine.
/// @Speaker immediately followed by a sigil ($, @, ->, }) = SpeakerTag (push to stack).
#[test]
fn dlg_choice_speaker_scope_isolation() {
    // @Narrator before $ → SpeakerTag (no text between @ and $); Narrator pushed to stack
    // Arm A: @Player I choose A. → SpeakerLine for Player; does NOT push Player to stack
    // Arm B: Me too. → TextLine; current_speaker() = Narrator (stack intact from outer scope)
    let ast = lower_src(r#"dlg test() { @Narrator $ choice { "A" { @Player I choose A. } "B" { Me too. } } }"#);
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// Phase 12 — Dialogue Lowering Fixes (DLG-01 through DLG-05)
// =========================================================

/// DLG-01: Namespace is threaded into localization key generation.
/// Key should differ from the same dialogue without a namespace.
#[test]
fn dlg_namespace_in_loc_key() {
    let ast = lower_src(
        "namespace my_mod { dlg greet(player: Entity) { @player Hello there. } }"
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-01: Multi-segment namespace (a::b) threaded into loc key generation.
#[test]
fn dlg_namespace_multi_segment_in_loc_key() {
    let ast = lower_src(
        "namespace a::b { dlg greet(player: Entity) { @player Hello there. } }"
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-02: Interpolation slot {name} preserved as-written, not replaced with {expr}.
#[test]
fn dlg_interpolation_slot_preserved() {
    let ast = lower_src(
        "dlg greet(player: Entity, name: string) { @player Hello, {name}! }"
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-02: Interpolation with member access {player.name} preserved.
#[test]
fn dlg_interpolation_member_access_preserved() {
    let ast = lower_src(
        "dlg greet(player: Entity) { @player Hello, {player.name}! }"
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-03: Choice label localization key emitted as second arg to Option().
/// Without fix, key was discarded with `let _ = key`.
#[test]
fn dlg_choice_label_key_emitted() {
    // Choice arms: "label" [#key] { body } — no => separator
    let ast = lower_src(
        r#"dlg convo(player: Entity) { @player Hi. $ choice { "Ask about weather" { @player How is the weather? } "Leave" { @player Goodbye. } } }"#
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-04: Line without #key -> say(speaker, text), NOT say_localized.
#[test]
fn dlg_say_without_key() {
    let ast = lower_src(
        "dlg greet(player: Entity) { @player Hello there. }"
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-04: Line with manual #key -> say_localized(speaker, key, fallback).
#[test]
fn dlg_say_localized_with_key() {
    let ast = lower_src(
        "dlg greet(player: Entity) { @player Hello there. #greeting_01 }"
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-04: Mix of keyed and non-keyed lines in same dialogue.
#[test]
fn dlg_say_mixed_key_dispatch() {
    let ast = lower_src(
        r#"dlg convo(player: Entity) { @player Hello there. @player How are you? #how_are_you @player Goodbye. }"#
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-05: Speaker set inside $ if branch doesn't leak to subsequent lines.
/// @player before $ is a SpeakerTag (pushes to stack). After if-branch, stack is restored.
#[test]
fn dlg_speaker_scope_isolation_if() {
    // @player immediately followed by $ → SpeakerTag (pushes player to stack)
    // $ if branch pushes @Narrator inside; DLG-05 restores after branch
    // "After branch." → TextLine, finds player on stack
    let ast = lower_src(
        r#"dlg scene(player: Entity) { @player $ if true { @Narrator A narrator appears. } After branch. }"#
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-05: Speaker set in one $ if branch doesn't leak to else branch.
/// @player before $ is a SpeakerTag (pushes to stack). Each branch gets isolated scope.
#[test]
fn dlg_speaker_scope_isolation_if_else() {
    // @player immediately followed by $ → SpeakerTag (pushes player to stack)
    // then-branch: @Narrator (SpeakerLine) doesn't leak to else-branch
    // else-branch: "Else text." → TextLine, finds player on stack (restored from before then-branch)
    let ast = lower_src(
        r#"dlg scene(player: Entity) { @player $ if true { @Narrator Branch A. } else { Else text. } }"#
    );
    insta::assert_debug_snapshot!(ast);
}

/// DLG-05: Speaker set inside $ match arm doesn't leak to subsequent lines.
/// @player before $ is a SpeakerTag (pushes to stack). Match arms get isolated scope.
#[test]
fn dlg_speaker_scope_isolation_match() {
    // @player immediately followed by $ → SpeakerTag (pushes player to stack)
    // Each match arm pushes @Narrator; DLG-05 restores after each arm
    // "After match." → TextLine, finds player on stack
    let ast = lower_src(
        r#"dlg scene(player: Entity, mood: int) { @player $ match mood { 1 => { @Narrator The player is happy. } _ => { @Narrator The player is sad. } } After match. }"#
    );
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R12 — Entity Lowering
//
// entity Name { ... } → AstDecl::Entity with properties, component slots, hooks, inherent impl
// [Singleton] propagates to entity attrs
// =========================================================

/// Properties lower to AstStructField with correct types and defaults; no inherent impl (no methods)
#[test]
fn entity_property_fields() {
    let ast = lower_src("entity Guard { name: string, level: int = 1 }");
    insta::assert_debug_snapshot!(ast);
}

/// use Health { current: 80, max: 80 } → AstComponentSlot with overrides (host-managed descriptor)
#[test]
fn entity_component_use_clause() {
    let ast = lower_src("entity Guard { name: string, use Health { current: 80, max: 80 } }");
    insta::assert_debug_snapshot!(ast);
}

/// Empty use clause produces AstComponentSlot with empty overrides vec
#[test]
fn entity_empty_use_clause() {
    let ast = lower_src("entity Guard { use Speaker {} }");
    insta::assert_debug_snapshot!(ast);
}

/// on create { ... } → impl OnCreate for Guard { fn on_create(self) { ... } }
#[test]
fn entity_lifecycle_on_create() {
    let ast = lower_src("entity Guard { on create { let x: int = 42; } }");
    insta::assert_debug_snapshot!(ast);
}

/// on interact(who: Entity) { ... } → impl OnInteract for Guard { fn on_interact(who: Entity) { ... } }
#[test]
fn entity_lifecycle_on_interact_with_params() {
    let ast = lower_src("entity Guard { on interact(who: Entity) { let msg: string = \"hello\"; } }");
    insta::assert_debug_snapshot!(ast);
}

/// on destroy { } → impl OnDestroy for Guard { fn on_destroy(self) { } }
#[test]
fn entity_lifecycle_on_destroy() {
    let ast = lower_src("entity Guard { on destroy { } }");
    insta::assert_debug_snapshot!(ast);
}

/// Entity method → inherent impl (contract: None); NOT in a contract impl
#[test]
fn entity_methods_inherent_impl() {
    let ast = lower_src("entity Guard { name: string, fn greet() -> string { return \"hello\"; } }");
    insta::assert_debug_snapshot!(ast);
}

/// [Singleton] attribute propagates to AstStructDecl.attrs
#[test]
fn entity_singleton_attribute() {
    let ast = lower_src("[Singleton] entity Narrator { name: string }");
    insta::assert_debug_snapshot!(ast);
}

/// Full entity with all four member types; deterministic emission order:
/// AstEntityDecl with properties, component_slots, inherent_impl, hooks
#[test]
fn entity_full_declaration() {
    let ast = lower_src(
        "entity Guard { name: string, use Health { current: 80, max: 80 }, fn greet() -> string { return \"hello\"; } on create { let ready: bool = true; } }"
    );
    insta::assert_debug_snapshot!(ast);
}

/// Two distinct AstComponentSlot entries in source order
#[test]
fn entity_multiple_use_clauses() {
    let ast = lower_src("entity Guard { use Health { current: 80 }, use Sprite { image: \"guard.png\" } }");
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R13 — Component Field Flattening
//
// use Component { field: val } → AstComponentSlot descriptor with overrides
// Fields not overridden left absent from initializer (type checker fills defaults)
// =========================================================

/// Overrides contain only `current: 50`, NOT `max` (not overridden → absent; type checker fills default)
#[test]
fn entity_component_partial_override() {
    let ast = lower_src("entity Guard { use Health { current: 50 } }");
    insta::assert_debug_snapshot!(ast);
}

/// Empty use clause — overrides vec is empty (same behavior as entity_empty_use_clause)
#[test]
fn entity_component_no_override() {
    let ast = lower_src("entity Guard { use Health {} }");
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// Entity Error Tests
//
// Duplicate use, duplicate property, unknown lifecycle, property-component collision
// All errors accumulated without halting
// =========================================================

/// Duplicate use Health → LoweringError::DuplicateUseClause; first use clause still produces field and impl
#[test]
fn entity_duplicate_use_clause_error() {
    let (ast, errors) = lower_src_with_errors("entity Guard { use Health { current: 80 }, use Health { max: 100 } }");
    insta::assert_debug_snapshot!((ast, errors));
}

/// Duplicate property `name` → LoweringError::DuplicateProperty; second property skipped
#[test]
fn entity_duplicate_property_error() {
    let (ast, errors) = lower_src_with_errors("entity Guard { name: string, name: int }");
    insta::assert_debug_snapshot!((ast, errors));
}

/// on explode → LoweringError::UnknownLifecycleEvent with event name "explode"
#[test]
fn entity_unknown_lifecycle_event_error() {
    let (ast, errors) = lower_src_with_errors("entity Guard { on explode { } }");
    insta::assert_debug_snapshot!((ast, errors));
}

/// Property `Health` collides with `use Health` → LoweringError::PropertyComponentCollision
#[test]
fn entity_property_component_collision_error() {
    let (ast, errors) = lower_src_with_errors("entity Guard { Health: int, use Health { current: 80 } }");
    insta::assert_debug_snapshot!((ast, errors));
}

// =========================================================
// R14 — Phase 13: Entity Model Conformance
//
// ENT-01: All 6 lifecycle hooks (create, destroy, interact, finalize, serialize, deserialize)
// ENT-02: Components as host-managed AstComponentSlot descriptors
// ENT-03: Implicit self/mut self in hooks and operators
// ENT-04: AstDecl::Entity variant
// MISC-01: IndexSet contract name (not IndexMut)
// =========================================================

/// on finalize hook → AstEntityHook with OnFinalize contract, implicit mut self
#[test]
fn entity_lifecycle_on_finalize() {
    let ast = lower_src("entity Guard { on finalize { let done: bool = true; } }");
    insta::assert_debug_snapshot!(ast);
}

/// on serialize hook → AstEntityHook with OnSerialize contract, implicit mut self
#[test]
fn entity_lifecycle_on_serialize() {
    let ast = lower_src("entity Guard { on serialize { let data: int = 0; } }");
    insta::assert_debug_snapshot!(ast);
}

/// on deserialize hook → AstEntityHook with OnDeserialize contract, implicit mut self
#[test]
fn entity_lifecycle_on_deserialize() {
    let ast = lower_src("entity Guard { on deserialize { let loaded: bool = false; } }");
    insta::assert_debug_snapshot!(ast);
}

/// All six hooks in one entity — none skipped
#[test]
fn entity_all_six_hooks() {
    let ast = lower_src(
        "entity Guard { \
            on create { let a: int = 1; } \
            on destroy { let b: int = 2; } \
            on interact(who: Entity) { let c: int = 3; } \
            on finalize { let d: int = 4; } \
            on serialize { let e: int = 5; } \
            on deserialize { let f: int = 6; } \
        }"
    );
    insta::assert_debug_snapshot!(ast);
}

/// Component slot model — use clause produces AstComponentSlot descriptor, not struct field
#[test]
fn entity_component_slot_model() {
    let ast = lower_src("entity Guard { use Health { current: 80, max: 100 } }");
    insta::assert_debug_snapshot!(ast);
}

/// Component slot with no overrides — empty overrides vec
#[test]
fn entity_component_slot_no_overrides() {
    let ast = lower_src("entity Guard { use Speaker {} }");
    insta::assert_debug_snapshot!(ast);
}

/// Multiple component slots — each gets its own AstComponentSlot
#[test]
fn entity_multiple_component_slots() {
    let ast = lower_src("entity Guard { use Health { current: 80 }, use Sprite { image: \"guard.png\" } }");
    insta::assert_debug_snapshot!(ast);
}

/// Implicit mut self in entity hook — on create gets mut self as first param
#[test]
fn entity_hook_implicit_mut_self() {
    let ast = lower_src("entity Guard { on create { let x: int = 1; } }");
    insta::assert_debug_snapshot!(ast);
}

/// Implicit self in operator — Add gets immutable self
#[test]
fn operator_implicit_self_immutable() {
    let ast = lower_src("impl Vec2 { operator +(other: Vec2) -> Vec2 { return other; } }");
    insta::assert_debug_snapshot!(ast);
}

/// Implicit mut self in IndexSet operator
#[test]
fn operator_implicit_mut_self_index_set() {
    let ast = lower_src("impl MyList { operator []=(idx: int, val: string) { } }");
    insta::assert_debug_snapshot!(ast);
}

/// IndexSet contract name is "IndexSet" not "IndexMut" (MISC-01)
#[test]
fn operator_index_set_contract_name() {
    let ast = lower_src("impl MyList { operator []=(idx: int, val: string) { } }");
    insta::assert_debug_snapshot!(ast);
}

/// Full entity with properties, component slots, methods, and hooks — complete structure
#[test]
fn entity_full_with_component_slots_and_all_hooks() {
    let ast = lower_src(
        "entity Guard { \
            name: string, \
            use Health { current: 80 }, \
            fn greet() -> string { return \"hello\"; } \
            on create { let ready: bool = true; } \
            on finalize { let done: bool = true; } \
        }"
    );
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R2 — Pipeline Infrastructure / lower_fn
//
// Basic fn with params and return type → AstFnDecl
// =========================================================

/// Basic function with params and return type lowered correctly
#[test]
fn fn_basic_with_params_and_return() {
    let ast = lower_src("fn add(a: int, b: int) -> int { return a; }");
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// Pass-through Item Lowering
//
// struct, enum, contract, component, extern, const, global, namespace, using
// These are mechanical CST → AST mappings in lower/mod.rs
// =========================================================

/// Struct and enum declarations pass through lowering correctly
#[test]
fn passthrough_struct_and_enum() {
    let ast = lower_src(r#"struct Point { x: float, y: float }
enum Color { Red, Green, Blue(intensity: int) }"#);
    insta::assert_debug_snapshot!(ast);
}

/// Contract and component declarations pass through lowering
#[test]
fn passthrough_contract_and_component() {
    let ast = lower_src(r#"contract Drawable {
    fn draw(x: int, y: int) -> bool;
}
extern component Health {
    current: int,
    max: int,
}"#);
    insta::assert_debug_snapshot!(ast);
}

/// Extern, const, and global declarations pass through lowering
#[test]
fn passthrough_extern_const_global() {
    let ast = lower_src(r#"extern fn print(msg: string);
const MAX_LEVEL: int = 99;
global mut score: int = 0;"#);
    insta::assert_debug_snapshot!(ast);
}

/// Namespace and using declarations pass through lowering
#[test]
fn passthrough_namespace_and_using() {
    let ast = lower_src(r#"namespace game::core;
using std::io;"#);
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// R15 — Integration Snapshot
//
// Full program: fn + operator impl + dlg + entity
// All passes exercised in one test
// =========================================================

/// Integration test: all construct types lowered end-to-end in one program
#[test]
fn integration_all_constructs() {
    let ast = lower_src(r#"fn describe(name: string?) -> string {
    return "unknown";
}

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

// =========================================================
// R15 — Localization Key Determinism
//
// Same source lowered twice must produce identical AST output
// Verifies FNV-1a keys are not pointer/allocation dependent
// =========================================================

/// Lowering the same dialogue source twice produces identical output
#[test]
fn localization_keys_are_deterministic() {
    let src: &'static str = r#"dlg intro() {
    @Narrator Hello.
    @Narrator Goodbye.
    @Narrator Welcome back. #welcome
}"#;

    let (items1, parse_errs1) = writ_parser::parse(src);
    assert!(parse_errs1.is_empty(), "parse 1 errors: {:?}", parse_errs1);
    let (ast1, errors1) = lower(items1.expect("parse 1 returned None"));
    assert!(errors1.is_empty(), "run 1 lowering errors: {:?}", errors1);

    let (items2, parse_errs2) = writ_parser::parse(src);
    assert!(parse_errs2.is_empty(), "parse 2 errors: {:?}", parse_errs2);
    let (ast2, errors2) = lower(items2.expect("parse 2 returned None"));
    assert!(errors2.is_empty(), "run 2 lowering errors: {:?}", errors2);

    assert_eq!(
        format!("{ast1:?}"),
        format!("{ast2:?}"),
        "Two lowering runs of identical source produced different AST output — localization keys may be non-deterministic"
    );
}

// =========================================================
// Phase 10: Parser -- Core Syntax (Lowering)
// =========================================================

// ---------------------------------------------------------
// PARSE-01: new keyword construction lowering
// ---------------------------------------------------------

/// `new Point { x: 1, y: 2 }` lowers to AstExpr::New with correct fields
#[test]
fn lower_new_construction_basic() {
    let ast = lower_src("fn f() { let p = new Point { x: 1, y: 2 }; }");
    insta::assert_debug_snapshot!(ast);
}

/// `new Point {}` lowers to AstExpr::New with empty fields
#[test]
fn lower_new_construction_empty() {
    let ast = lower_src("fn f() { let p = new Point {}; }");
    insta::assert_debug_snapshot!(ast);
}

/// `new List<int> {}` lowers with generic type
#[test]
fn lower_new_construction_generic() {
    let ast = lower_src("fn f() { let p = new List<int> {}; }");
    insta::assert_debug_snapshot!(ast);
}

// ---------------------------------------------------------
// PARSE-02: hex/binary literal lowering
// ---------------------------------------------------------

/// Hex and binary literals lower as IntLit preserving raw text
#[test]
fn lower_hex_binary_literals() {
    let ast = lower_src("fn f() { let a = 0xFF; let b = 0b1010; }");
    insta::assert_debug_snapshot!(ast);
}

/// Hex/binary literals with underscore separators lower correctly
/// 0xFF_FF → 65535, 0b1010_0101 → 165
#[test]
fn lower_hex_binary_underscore_separators() {
    let ast = lower_src("fn f() { let a = 0xFF_FF; let b = 0b1010_0101; }");
    insta::assert_debug_snapshot!(ast);
}

/// Uppercase prefix hex/binary literals lower correctly
/// 0XFF → 255, 0B1010 → 10
#[test]
fn lower_hex_binary_uppercase_prefix() {
    let ast = lower_src("fn f() { let a = 0XFF; let b = 0B1010; }");
    insta::assert_debug_snapshot!(ast);
}

/// Zero values in hex and binary lower correctly
/// 0x0 → 0, 0b0 → 0, plain 42 → 42
#[test]
fn lower_hex_binary_zero_and_decimal() {
    let ast = lower_src("fn f() { let a = 0x0; let b = 0b0; let c = 42; }");
    insta::assert_debug_snapshot!(ast);
}

// ---------------------------------------------------------
// DECL-01: struct lifecycle hooks lowering
// ---------------------------------------------------------

/// Struct with lifecycle hook lowers to AstStructMember::OnHook
#[test]
fn lower_struct_lifecycle_hook() {
    let ast = lower_src("struct Foo { x: int, on create { let y = 1; } }");
    insta::assert_debug_snapshot!(ast);
}

/// Struct with multiple hooks lowers correctly
#[test]
fn lower_struct_multiple_hooks() {
    let ast = lower_src("struct Bar { on create { }, on finalize { } }");
    insta::assert_debug_snapshot!(ast);
}

// ---------------------------------------------------------
// DECL-02: self/mut self parameter lowering
// ---------------------------------------------------------

/// `fn foo(self)` lowers with AstFnParam::SelfParam { mutable: false }
#[test]
fn lower_self_param() {
    let ast = lower_src("fn foo(self) { }");
    insta::assert_debug_snapshot!(ast);
}

/// `fn bar(mut self, x: int)` lowers with mutable SelfParam + regular param
#[test]
fn lower_mut_self_with_regular_param() {
    let ast = lower_src("fn bar(mut self, x: int) { }");
    insta::assert_debug_snapshot!(ast);
}

// ---------------------------------------------------------
// EXPR-01: shift operator lowering
// ---------------------------------------------------------

/// `a << b` and `a >> b` lower as BinaryOp::Shl and BinaryOp::Shr
#[test]
fn lower_shift_operators() {
    let ast = lower_src("fn f() { let x = a << b; let y = c >> d; }");
    insta::assert_debug_snapshot!(ast);
}

// ---------------------------------------------------------
// EXPR-02: BitAnd/BitOr operator overloading lowering
// ---------------------------------------------------------

/// `operator &` and `operator |` in impl blocks lower to AstOpSymbol::BitAnd/BitOr
#[test]
fn lower_bitand_bitor_operators() {
    let ast = lower_src("impl Flags { operator &(other: Flags) -> Flags { } operator |(other: Flags) -> Flags { } }");
    insta::assert_debug_snapshot!(ast);
}

// =========================================================
// Phase 11: Lowering tests for new declarations/expressions
// =========================================================

/// SpawnDetached lowers to AstExpr::SpawnDetached (not nested Spawn+Detached)
#[test]
fn lower_spawn_detached() {
    let ast = lower_src("pub fn test() { spawn detached playSound(\"beep\"); }");
    insta::assert_debug_snapshot!(ast);
}

/// impl<T> with generics passes through to AstImplDecl.generics
#[test]
fn lower_impl_with_generics() {
    let ast = lower_src("impl<T> Printable<T> for Container<T> { fn print() { } }");
    insta::assert_debug_snapshot!(ast);
}

/// pub extern fn preserves visibility in AstExternDecl
#[test]
fn lower_pub_extern_fn() {
    let ast = lower_src("pub extern fn log(msg: string);");
    insta::assert_debug_snapshot!(ast);
}

/// extern fn with dotted name preserves qualifier
#[test]
fn lower_extern_fn_dotted_name() {
    let ast = lower_src("extern fn Entity.getOrCreate<T>() -> T;");
    insta::assert_debug_snapshot!(ast);
}

/// defer with block body lowers correctly
#[test]
fn lower_defer_block() {
    let ast = lower_src("pub fn test() { defer { cleanup(); } }");
    insta::assert_debug_snapshot!(ast);
}

/// Attribute with = separator lowers correctly
#[test]
fn lower_attr_eq_separator() {
    let ast = lower_src("[Import(lib = \"physics\")] extern fn applyForce();");
    insta::assert_debug_snapshot!(ast);
}

/// Contract with operator signatures lowers correctly
#[test]
fn lower_contract_with_op_sigs() {
    let ast = lower_src("contract Addable<T> { operator +(other: T) -> T; }");
    insta::assert_debug_snapshot!(ast);
}
