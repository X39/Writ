//! Integration tests for name resolution Pass 1 (declaration collector).

use writ_compiler::ast::Ast;
use writ_compiler::lower::lower;
use writ_compiler::resolve::collector::collect_declarations;
use writ_compiler::resolve::def_map::{DefKind, DefMap};
use writ_diagnostics::{Diagnostic, FileId};

// =========================================================
// Test helpers
// =========================================================

/// Parse and lower a single source string, returning AST.
fn parse_and_lower(src: &'static str) -> Ast {
    let (items, parse_errors) = writ_parser::parse(src);
    let items = items.expect("parse returned None");
    let error_msgs: Vec<String> = parse_errors.iter().map(|e| format!("{e:?}")).collect();
    assert!(error_msgs.is_empty(), "parse errors: {:?}", error_msgs);
    let (ast, lower_errors) = lower(items);
    assert!(lower_errors.is_empty(), "lowering errors: {:?}", lower_errors);
    ast
}

/// Parse and lower a single source file, then collect declarations.
fn collect_src(src: &'static str) -> (DefMap, Vec<Diagnostic>) {
    collect_src_with_path(src, "src/test.writ")
}

/// Parse and lower with a specific file path.
fn collect_src_with_path(src: &'static str, path: &str) -> (DefMap, Vec<Diagnostic>) {
    let ast = parse_and_lower(src);
    let file_id = FileId(0);
    let asts: Vec<(FileId, &Ast)> = vec![(file_id, &ast)];
    let file_paths: Vec<(FileId, &str)> = vec![(file_id, path)];
    collect_declarations(&asts, &file_paths)
}

/// Parse and lower multiple source files, then collect declarations.
/// Each entry: (file_path, source_code, description).
fn collect_multi(files: &[(&str, &'static str, &str)]) -> (DefMap, Vec<Diagnostic>) {
    let mut asts_owned = Vec::new();
    for (i, (_, src, _)) in files.iter().enumerate() {
        let ast = parse_and_lower(src);
        asts_owned.push((FileId(i as u32), ast));
    }

    let asts: Vec<(FileId, &Ast)> = asts_owned.iter().map(|(id, ast)| (*id, ast)).collect();
    let file_paths: Vec<(FileId, &str)> = files
        .iter()
        .enumerate()
        .map(|(i, (path, _, _))| (FileId(i as u32), *path))
        .collect();

    collect_declarations(&asts, &file_paths)
}

fn has_error_code(diags: &[Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| d.code == code)
}

fn count_error_code(diags: &[Diagnostic], code: &str) -> usize {
    diags.iter().filter(|d| d.code == code).count()
}

// =========================================================
// Pass 1: All 10 declaration kinds
// =========================================================

#[test]
fn collect_all_declaration_kinds() {
    let (def_map, diags) = collect_src(
        r#"
pub fn greet() {}
pub struct Potion { pub name: string }
pub entity Player { pub health: int }
pub enum Direction { North, South }
pub contract Movable { fn move_to(x: int, y: int); }
impl Movable for Potion { fn move_to(self, x: int, y: int) {} }
pub extern fn get_time() -> int;
pub const MAX_HP: int = 100;
pub global mut tick_count: int = 0;
"#,
    );

    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);

    // Check all pub kinds are present in by_fqn
    assert!(def_map.get("greet").is_some(), "fn greet");
    assert!(def_map.get("Potion").is_some(), "struct Potion");
    assert!(def_map.get("Player").is_some(), "entity Player");
    assert!(def_map.get("Direction").is_some(), "enum Direction");
    assert!(def_map.get("Movable").is_some(), "contract Movable");
    assert!(def_map.get("get_time").is_some(), "extern fn get_time");
    assert!(def_map.get("MAX_HP").is_some(), "const MAX_HP");
    assert!(def_map.get("tick_count").is_some(), "global tick_count");

    // Check kinds
    assert_eq!(def_map.get_entry(def_map.get("greet").unwrap()).kind, DefKind::Fn);
    assert_eq!(def_map.get_entry(def_map.get("Potion").unwrap()).kind, DefKind::Struct);
    assert_eq!(def_map.get_entry(def_map.get("Player").unwrap()).kind, DefKind::Entity);
    assert_eq!(def_map.get_entry(def_map.get("Direction").unwrap()).kind, DefKind::Enum);
    assert_eq!(def_map.get_entry(def_map.get("Movable").unwrap()).kind, DefKind::Contract);
    assert_eq!(def_map.get_entry(def_map.get("get_time").unwrap()).kind, DefKind::ExternFn);
    assert_eq!(def_map.get_entry(def_map.get("MAX_HP").unwrap()).kind, DefKind::Const);
    assert_eq!(def_map.get_entry(def_map.get("tick_count").unwrap()).kind, DefKind::Global);

    // Impl block tracked separately
    assert!(!def_map.impl_blocks.is_empty(), "impl block should be tracked");
    assert_eq!(
        def_map.get_entry(def_map.impl_blocks[0]).kind,
        DefKind::Impl
    );
}

// =========================================================
// Extern component (separate test due to parser syntax)
// =========================================================

#[test]
fn collect_extern_component() {
    let (def_map, diags) = collect_src(
        r#"
extern component Health {
    current: int,
    max: int,
}
"#,
    );

    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);

    // Extern component without pub is private
    let file_privates = def_map.file_private.get(&FileId(0)).unwrap();
    assert!(file_privates.contains_key("Health"), "Health in file_private");
    let health_id = file_privates["Health"];
    assert_eq!(def_map.get_entry(health_id).kind, DefKind::ExternComponent);
}

// =========================================================
// Namespace handling: declarative
// =========================================================

#[test]
fn declarative_namespace_prefixes_names() {
    let (def_map, diags) = collect_src(
        r#"
namespace survival;
pub struct Potion { pub name: string }
pub fn brew() {}
"#,
    );

    assert!(diags.iter().all(|d| d.code != "E0001" && d.code != "E0002"),
        "unexpected errors: {:?}", diags);

    assert!(def_map.get("survival::Potion").is_some(), "survival::Potion");
    assert!(def_map.get("survival::brew").is_some(), "survival::brew");
    assert!(def_map.get("Potion").is_none(), "bare Potion should not exist");
}

// =========================================================
// Namespace handling: block
// =========================================================

#[test]
fn block_namespace_nesting() {
    let (def_map, diags) = collect_src(
        r#"
namespace a {
    namespace b {
        pub struct X {}
    }
    pub struct Y {}
}
"#,
    );

    assert!(diags.iter().all(|d| d.code != "E0001"), "unexpected duplicate errors: {:?}", diags);

    assert!(def_map.get("a::b::X").is_some(), "a::b::X");
    assert!(def_map.get("a::Y").is_some(), "a::Y");
}

// =========================================================
// Multi-file namespace merging
// =========================================================

#[test]
fn multi_file_namespace_merge() {
    let (def_map, diags) = collect_multi(&[
        (
            "src/survival/potions.writ",
            r#"
namespace survival;
pub struct Potion { pub name: string }
"#,
            "file1",
        ),
        (
            "src/survival/weapons.writ",
            r#"
namespace survival;
pub struct Weapon { pub damage: int }
"#,
            "file2",
        ),
    ]);

    assert!(diags.iter().all(|d| d.code != "E0001"), "unexpected duplicate errors: {:?}", diags);

    assert!(def_map.get("survival::Potion").is_some(), "survival::Potion");
    assert!(def_map.get("survival::Weapon").is_some(), "survival::Weapon");

    // Both should be in the same namespace member list
    let members = def_map.pub_members_of("survival");
    assert_eq!(members.len(), 2, "survival should have 2 members");
}

// =========================================================
// Visibility: private defs are file-scoped
// =========================================================

#[test]
fn private_declarations_file_scoped() {
    let (def_map, diags) = collect_src(
        r#"
namespace survival;
struct PrivateHelper {}
pub struct PublicPotion { pub name: string }
"#,
    );

    // No errors expected (no explicit "priv" keyword needed — default is private)
    assert!(diags.iter().all(|d| d.code != "E0001"), "unexpected errors: {:?}", diags);

    // Public def is in by_fqn
    assert!(def_map.get("survival::PublicPotion").is_some());

    // Private def is NOT in by_fqn
    assert!(def_map.get("survival::PrivateHelper").is_none());
    assert!(def_map.get("PrivateHelper").is_none());

    // Private def IS in file_private
    let file_privates = def_map.file_private.get(&FileId(0)).unwrap();
    assert!(file_privates.contains_key("PrivateHelper"));
}

// =========================================================
// Prelude shadow errors
// =========================================================

#[test]
fn prelude_shadow_types() {
    let (_, diags) = collect_src("pub struct Option {}");
    assert!(has_error_code(&diags, "E0002"), "Option should trigger prelude shadow");

    let (_, diags) = collect_src("pub struct Entity {}");
    assert!(has_error_code(&diags, "E0002"), "Entity should trigger prelude shadow");

    let (_, diags) = collect_src("pub struct Result {}");
    assert!(has_error_code(&diags, "E0002"), "Result should trigger prelude shadow");

    let (_, diags) = collect_src("pub struct Array {}");
    assert!(has_error_code(&diags, "E0002"), "Array should trigger prelude shadow");

    let (_, diags) = collect_src("pub struct Range {}");
    assert!(has_error_code(&diags, "E0002"), "Range should trigger prelude shadow");
}

#[test]
fn prelude_shadow_contracts() {
    let (_, diags) = collect_src("pub contract Add { fn add(self, other: int) -> int; }");
    assert!(has_error_code(&diags, "E0002"), "Add should trigger prelude shadow");

    let (_, diags) = collect_src("pub contract Eq { fn eq(self, other: int) -> bool; }");
    assert!(has_error_code(&diags, "E0002"), "Eq should trigger prelude shadow");

    let (_, diags) = collect_src("pub contract Iterator { fn next(mut self) -> int; }");
    assert!(has_error_code(&diags, "E0002"), "Iterator should trigger prelude shadow");

    let (_, diags) = collect_src("pub contract Error { fn message(self) -> string; }");
    assert!(has_error_code(&diags, "E0002"), "Error should trigger prelude shadow");
}

// =========================================================
// Duplicate definition errors
// =========================================================

#[test]
fn duplicate_definition_same_file() {
    let (_, diags) = collect_src(
        r#"
pub struct Foo {}
pub struct Foo {}
"#,
    );

    assert!(has_error_code(&diags, "E0001"), "duplicate Foo should produce E0001");
}

#[test]
fn duplicate_definition_across_files() {
    let (_, diags) = collect_multi(&[
        (
            "src/a.writ",
            r#"
namespace shared;
pub struct Item {}
"#,
            "file1",
        ),
        (
            "src/b.writ",
            r#"
namespace shared;
pub struct Item {}
"#,
            "file2",
        ),
    ]);

    assert!(has_error_code(&diags, "E0001"), "duplicate shared::Item should produce E0001");
}

// =========================================================
// W0004: Namespace/path mismatch
// =========================================================

#[test]
fn namespace_path_mismatch_warning() {
    let (_, diags) = collect_src_with_path(
        r#"
namespace survival;
pub struct Potion {}
"#,
        "src/combat/weapons.writ",
    );

    assert!(
        has_error_code(&diags, "W0004"),
        "file path src/combat/weapons.writ should not match namespace survival"
    );
}

#[test]
fn namespace_path_match_no_warning() {
    let (_, diags) = collect_src_with_path(
        r#"
namespace survival;
pub struct Potion {}
"#,
        "src/survival/potions.writ",
    );

    assert!(
        !has_error_code(&diags, "W0004"),
        "file path src/survival/potions.writ should match namespace survival"
    );
}

// =========================================================
// Prelude module tests (unit-level, run here for convenience)
// =========================================================

#[test]
fn prelude_coverage() {
    use writ_compiler::resolve::prelude::*;

    // All 5 primitives
    assert!(is_prelude_name("int"));
    assert!(is_prelude_name("float"));
    assert!(is_prelude_name("bool"));
    assert!(is_prelude_name("string"));
    assert!(is_prelude_name("void"));

    // All 5 types
    assert!(is_prelude_name("Option"));
    assert!(is_prelude_name("Result"));
    assert!(is_prelude_name("Range"));
    assert!(is_prelude_name("Array"));
    assert!(is_prelude_name("Entity"));

    // All 17 contracts
    for name in PRELUDE_CONTRACT_NAMES {
        assert!(is_prelude_name(name), "contract {name} should be in prelude");
    }

    // 27 total
    let total = PRELUDE_PRIMITIVE_NAMES.len()
        + PRELUDE_TYPE_NAMES.len()
        + PRELUDE_CONTRACT_NAMES.len();
    assert_eq!(total, 27, "prelude should have 27 names total");

    // Non-prelude
    assert!(!is_prelude_name("Foo"));
    assert!(!is_prelude_name("my_func"));
}

// =========================================================
// Pass 2: Scope chain and type resolution
// =========================================================

use writ_compiler::resolve;
// ResolvedType used for type-level assertions if needed
#[allow(unused_imports)]
use writ_compiler::resolve::ir::ResolvedType;

/// Helper: resolve a single source file and return (NameResolvedAst, diagnostics).
fn resolve_src(src: &'static str) -> (resolve::ir::NameResolvedAst, Vec<Diagnostic>) {
    resolve_src_with_path(src, "src/test.writ")
}

fn resolve_src_with_path(src: &'static str, path: &str) -> (resolve::ir::NameResolvedAst, Vec<Diagnostic>) {
    let ast = parse_and_lower(src);
    let file_id = FileId(0);
    let asts: Vec<(FileId, &writ_compiler::ast::Ast)> = vec![(file_id, &ast)];
    let file_paths: Vec<(FileId, &str)> = vec![(file_id, path)];
    resolve::resolve(&asts, &file_paths)
}

fn resolve_multi(files: &[(&str, &'static str, &str)]) -> (resolve::ir::NameResolvedAst, Vec<Diagnostic>) {
    let mut asts_owned = Vec::new();
    for (i, (_, src, _)) in files.iter().enumerate() {
        let ast = parse_and_lower(src);
        asts_owned.push((FileId(i as u32), ast));
    }

    let asts: Vec<(FileId, &writ_compiler::ast::Ast)> = asts_owned.iter().map(|(id, ast)| (*id, ast)).collect();
    let file_paths: Vec<(FileId, &str)> = files
        .iter()
        .enumerate()
        .map(|(i, (path, _, _))| (FileId(i as u32), *path))
        .collect();

    resolve::resolve(&asts, &file_paths)
}

#[test]
fn scope_resolve_primitive_types() {
    let (resolved, diags) = resolve_src(
        "pub fn foo(x: int, y: float, z: bool, s: string) {}",
    );
    // No errors expected for primitive types
    let type_errors: Vec<_> = diags.iter().filter(|d| d.code == "E0003").collect();
    assert!(type_errors.is_empty(), "primitive types should resolve: {:?}", type_errors);
    assert!(!resolved.decls.is_empty(), "should have resolved declarations");
}

#[test]
fn scope_resolve_same_namespace_type() {
    let (_, diags) = resolve_src(
        r#"
namespace game;
pub struct Point { pub x: int, pub y: int }
pub fn make_point() -> Point { return new Point { x: 0, y: 0 }; }
"#,
    );
    let type_errors: Vec<_> = diags.iter().filter(|d| d.code == "E0003").collect();
    assert!(type_errors.is_empty(), "Point should resolve in same namespace: {:?}", type_errors);
}

#[test]
fn scope_resolve_using_import() {
    let (_, diags) = resolve_multi(&[
        (
            "src/survival/items.writ",
            r#"
namespace survival;
pub struct HealthPotion { pub amount: int }
"#,
            "items",
        ),
        (
            "src/combat/fight.writ",
            r#"
namespace combat;
using survival;
pub fn use_potion(p: HealthPotion) {}
"#,
            "fight",
        ),
    ]);
    let type_errors: Vec<_> = diags.iter().filter(|d| d.code == "E0003").collect();
    assert!(
        type_errors.is_empty(),
        "HealthPotion should resolve via using survival: {:?}",
        type_errors
    );
}

#[test]
fn scope_resolve_qualified_path() {
    let (_, diags) = resolve_multi(&[
        (
            "src/survival/items.writ",
            r#"
namespace survival;
pub struct HealthPotion { pub amount: int }
"#,
            "items",
        ),
        (
            "src/combat/fight.writ",
            r#"
namespace combat;
pub fn use_potion(p: survival::HealthPotion) {}
"#,
            "fight",
        ),
    ]);
    let type_errors: Vec<_> = diags.iter().filter(|d| d.code == "E0003").collect();
    assert!(
        type_errors.is_empty(),
        "survival::HealthPotion should resolve via qualified path: {:?}",
        type_errors
    );
}

#[test]
fn scope_visibility_violation() {
    let (_, diags) = resolve_multi(&[
        (
            "src/a.writ",
            r#"
namespace shared;
struct PrivateHelper { pub x: int }
"#,
            "file1",
        ),
        (
            "src/b.writ",
            r#"
namespace shared;
pub fn use_helper(h: PrivateHelper) {}
"#,
            "file2",
        ),
    ]);
    // PrivateHelper is private and should cause E0005 or E0003
    let errors: Vec<_> = diags.iter().filter(|d| d.code == "E0005" || d.code == "E0003").collect();
    assert!(
        !errors.is_empty(),
        "using private type from another file should produce an error"
    );
}

#[test]
fn scope_unused_import_warning() {
    let (_, diags) = resolve_multi(&[
        (
            "src/survival/items.writ",
            r#"
namespace survival;
pub struct HealthPotion { pub amount: int }
"#,
            "items",
        ),
        (
            "src/test.writ",
            r#"
using survival;
pub fn hello() {}
"#,
            "test",
        ),
    ]);
    assert!(
        has_error_code(&diags, "W0001"),
        "unused using survival should produce W0001"
    );
}

#[test]
fn scope_used_import_no_warning() {
    let (_, diags) = resolve_multi(&[
        (
            "src/survival/items.writ",
            r#"
namespace survival;
pub struct HealthPotion { pub amount: int }
"#,
            "items",
        ),
        (
            "src/test.writ",
            r#"
using survival;
pub fn use_it(p: HealthPotion) {}
"#,
            "test",
        ),
    ]);
    assert!(
        !has_error_code(&diags, "W0001"),
        "used import should NOT produce W0001: {:?}",
        diags.iter().filter(|d| d.code == "W0001").collect::<Vec<_>>()
    );
}

#[test]
fn scope_resolve_array_type() {
    let (_, diags) = resolve_src(
        "pub fn foo(xs: int[]) {}",
    );
    let type_errors: Vec<_> = diags.iter().filter(|d| d.code == "E0003").collect();
    assert!(type_errors.is_empty(), "int[] should resolve: {:?}", type_errors);
}

#[test]
fn scope_resolve_generic_type() {
    let (_, diags) = resolve_src(
        r#"
pub struct Container<T> { pub value: T }
"#,
    );
    let type_errors: Vec<_> = diags.iter().filter(|d| d.code == "E0003").collect();
    assert!(type_errors.is_empty(), "generic T should resolve inside Container: {:?}", type_errors);
}

#[test]
fn scope_impl_resolves_target_and_contract() {
    let (resolved, diags) = resolve_src(
        r#"
pub struct Point { pub x: int, pub y: int }
pub contract Drawable { fn draw(self); }
impl Drawable for Point { fn draw(self) {} }
"#,
    );
    let type_errors: Vec<_> = diags.iter().filter(|d| d.code == "E0003").collect();
    assert!(type_errors.is_empty(), "impl target and contract should resolve: {:?}", type_errors);
    // Check that we got an Impl decl
    let has_impl = resolved.decls.iter().any(|d| matches!(d, resolve::ir::ResolvedDecl::Impl { .. }));
    assert!(has_impl, "should have a resolved Impl decl");
}

#[test]
fn scope_ambiguous_name_error() {
    let (_, diags) = resolve_multi(&[
        (
            "src/a/items.writ",
            r#"
namespace a;
pub struct Widget { pub x: int }
"#,
            "a",
        ),
        (
            "src/b/items.writ",
            r#"
namespace b;
pub struct Widget { pub y: int }
"#,
            "b",
        ),
        (
            "src/test.writ",
            r#"
using a;
using b;
pub fn use_widget(w: Widget) {}
"#,
            "test",
        ),
    ]);
    assert!(
        has_error_code(&diags, "E0004"),
        "Widget from both a and b should produce E0004 ambiguity error"
    );
}

// =========================================================
// Wave 3: Validation and error quality
// =========================================================

// ---------------------------------------------------------
// Attribute target validation
// ---------------------------------------------------------

#[test]
fn validate_singleton_on_entity_ok() {
    let (_, diags) = resolve_src(
        r#"
[Singleton]
pub entity GameManager { pub score: int }
"#,
    );
    assert!(
        !has_error_code(&diags, "E0006"),
        "[Singleton] on entity should NOT produce E0006: {:?}",
        diags.iter().filter(|d| d.code == "E0006").collect::<Vec<_>>()
    );
}

#[test]
fn validate_singleton_on_struct_error() {
    let (_, diags) = resolve_src(
        r#"
[Singleton]
pub struct Oops { pub x: int }
"#,
    );
    assert!(
        has_error_code(&diags, "E0006"),
        "[Singleton] on struct should produce E0006"
    );
}

#[test]
fn validate_singleton_on_fn_error() {
    let (_, diags) = resolve_src(
        r#"
[Singleton]
pub fn broken() {}
"#,
    );
    assert!(
        has_error_code(&diags, "E0006"),
        "[Singleton] on fn should produce E0006"
    );
}

#[test]
fn validate_conditional_on_fn_ok() {
    let (_, diags) = resolve_src(
        r#"
[Conditional]
pub fn maybe_run() {}
"#,
    );
    assert!(
        !has_error_code(&diags, "E0006"),
        "[Conditional] on fn should NOT produce E0006: {:?}",
        diags.iter().filter(|d| d.code == "E0006").collect::<Vec<_>>()
    );
}

#[test]
fn validate_conditional_on_entity_error() {
    let (_, diags) = resolve_src(
        r#"
[Conditional]
pub entity BadEntity { pub x: int }
"#,
    );
    assert!(
        has_error_code(&diags, "E0006"),
        "[Conditional] on entity should produce E0006"
    );
}

// ---------------------------------------------------------
// Fuzzy suggestion quality (E0003 with help text)
// ---------------------------------------------------------

#[test]
fn suggestion_for_close_type_name() {
    let (_, diags) = resolve_src(
        r#"
pub struct HealthPotion { pub amount: int }
pub fn use_it(p: HelthPotion) {}
"#,
    );
    // Should produce E0003 with a suggestion
    assert!(
        has_error_code(&diags, "E0003"),
        "HelthPotion should produce E0003 (unresolved name)"
    );
    // Check the help text contains a suggestion
    let e0003 = diags.iter().find(|d| d.code == "E0003").unwrap();
    assert!(
        e0003.help.contains("did you mean"),
        "E0003 for HelthPotion should have a 'did you mean' suggestion in help: {:?}",
        e0003.help
    );
}

#[test]
fn no_suggestion_for_unrelated_name() {
    let (_, diags) = resolve_src(
        r#"
pub fn use_it(p: ZzzzXxxx999) {}
"#,
    );
    assert!(
        has_error_code(&diags, "E0003"),
        "ZzzzXxxx999 should produce E0003 (unresolved name)"
    );
    // Should NOT have a "did you mean" suggestion (nothing is close)
    let e0003 = diags.iter().find(|d| d.code == "E0003").unwrap();
    assert!(
        !e0003.help.contains("did you mean"),
        "ZzzzXxxx999 should NOT have a 'did you mean' suggestion: {:?}",
        e0003.help
    );
}

// ---------------------------------------------------------
// Generic shadow warning (W0003)
// ---------------------------------------------------------

#[test]
fn generic_shadow_warning() {
    let (_, diags) = resolve_src(
        r#"
pub struct Outer { pub x: int }
pub struct Container<Outer> { pub value: Outer }
"#,
    );
    assert!(
        has_error_code(&diags, "W0003"),
        "generic param 'Outer' should shadow existing type and produce W0003"
    );
}

#[test]
fn generic_no_shadow() {
    let (_, diags) = resolve_src(
        r#"
pub struct Container<T> { pub value: T }
"#,
    );
    assert!(
        !has_error_code(&diags, "W0003"),
        "generic T should NOT produce W0003 since there's no existing type T: {:?}",
        diags.iter().filter(|d| d.code == "W0003").collect::<Vec<_>>()
    );
}
