use chumsky::span::{SimpleSpan, Span as _};
use rustc_hash::FxHashMap;
use writ_compiler::Ast;
use writ_compiler::check::ir::{TypedExpr, TypedStmt};
use writ_compiler::check::ty::{Ty, TyInterner, TyKind};
use writ_compiler::emit::body::BodyEmitter;
use writ_compiler::emit::body::stmt::emit_stmt;
use writ_compiler::emit::metadata::{MetadataToken, TableId};
use writ_compiler::emit::module_builder::ModuleBuilder;
use writ_compiler::resolve::def_map::{DefEntry, DefId, DefKind, DefMap, DefVis};
use writ_diagnostics::{FileId, Severity};
use writ_module::Instruction;

fn emit_metadata(source: &'static str) -> ModuleBuilder {
    let (parsed, parse_errors) = writ_parser::parse(source);
    assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
    let (ast, lower_errors) = writ_compiler::lower(parsed.expect("parser produced no CST"));
    assert!(lower_errors.is_empty(), "lower errors: {lower_errors:?}");

    let asts: Vec<(FileId, &Ast)> = vec![(FileId(0), &ast)];
    let paths = [(FileId(0), "src/test.writ")];
    let (resolved, resolve_diagnostics) = writ_compiler::resolve::resolve(&asts, &paths, &[]);
    assert!(
        resolve_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error),
        "resolve diagnostics: {resolve_diagnostics:?}"
    );

    let (typed, interner, _type_env, type_diagnostics) =
        writ_compiler::check::typecheck(resolved, &asts, &[]);
    assert!(
        type_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error),
        "type diagnostics: {type_diagnostics:?}"
    );

    let (builder, emit_diagnostics) = writ_compiler::emit::emit(&typed, &asts, &interner);
    assert!(
        emit_diagnostics.is_empty(),
        "emit diagnostics: {emit_diagnostics:?}"
    );
    builder
}

#[test]
fn generic_impl_contracts_emit_non_null_contract_tokens() {
    let builder = emit_metadata(
        r#"
pub contract GenericContract<T> {}
pub class GenericTarget<T> {}
impl<T> GenericContract<T> for GenericTarget<T> {}

pub class List<T> {}
impl<T> Iterable<T> for List<T> {}

pub class Cursor<T> {}
impl<T> Iterator<T> for Cursor<T> {}

pub class Counter {}
impl Iterable<int> for Counter {}
"#,
    );

    let contracts: Vec<MetadataToken> = builder
        .finalized_impl_defs()
        .iter()
        .map(|implementation| implementation.contract_token)
        .collect();

    let user_generic_contract = MetadataToken::new(TableId::ContractDef, 1);
    let iterable_contract = MetadataToken::new(TableId::ContractDef, 14);
    let iterator_contract = MetadataToken::new(TableId::ContractDef, 15);

    assert!(
        contracts.contains(&user_generic_contract),
        "GenericContract<T> must resolve through its DefMap ContractDef"
    );
    assert_eq!(
        contracts
            .iter()
            .filter(|token| **token == iterable_contract)
            .count(),
        2,
        "both Iterable<T> and Iterable<int> must retain the Iterable contract token"
    );
    assert_eq!(
        contracts
            .iter()
            .filter(|token| **token == iterator_contract)
            .count(),
        1,
        "Iterator<T> must retain the Iterator contract token"
    );
    assert!(
        contracts.iter().all(|token| !token.is_null()),
        "no declared contract impl may emit a NULL contract token: {contracts:?}"
    );
}

fn span() -> SimpleSpan {
    SimpleSpan::new((), 0..0)
}

#[test]
fn class_for_loop_emits_matching_iterable_dispatch_tokens() {
    let mut def_map = DefMap::new();
    let class_id = def_map.arena.alloc(DefEntry {
        id: None,
        kind: DefKind::Class,
        vis: DefVis::Pub,
        file_id: FileId(0),
        namespace: String::new(),
        name: "List".to_string(),
        name_span: span(),
        generics: vec!["T".to_string()],
        span: span(),
    });

    let mut interner = TyInterner::new();
    let int_ty = interner.int();
    let class_ty = interner.intern(TyKind::Class(class_id));
    let builder = ModuleBuilder::new();
    let struct_fields: FxHashMap<DefId, Vec<(String, Ty)>> = FxHashMap::default();
    let mut emitter = BodyEmitter::new(&builder, &interner, &struct_fields);
    emitter.locals.insert("list".to_string(), 0);
    emitter.regs.alloc(class_ty);

    emit_stmt(
        &mut emitter,
        &TypedStmt::For {
            binding: "value".to_string(),
            binding_span: span(),
            binding_ty: int_ty,
            mutable: false,
            iterable: TypedExpr::Var {
                ty: class_ty,
                span: span(),
                name: "list".to_string(),
            },
            body: vec![],
            span: span(),
            iterable_contract_def_id: None,
            iterator_contract_def_id: None,
        },
    );

    let dispatches: Vec<(u32, u16)> = emitter
        .instructions
        .iter()
        .filter_map(|instruction| match instruction {
            Instruction::CallVirt {
                contract_idx, slot, ..
            } => Some((*contract_idx, *slot)),
            _ => None,
        })
        .collect();

    assert_eq!(
        dispatches,
        vec![
            (MetadataToken::new(TableId::ContractDef, 14).0, 0),
            (MetadataToken::new(TableId::ContractDef, 15).0, 0),
        ],
        "for-in emission and ImplDef collection must use identical contract tokens"
    );
}
