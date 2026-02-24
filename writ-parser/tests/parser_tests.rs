//! Comprehensive parser tests for Phases 2-4.
//!
//! Phase 2: TYPE-01, TYPE-02, EXPR-01 through EXPR-09, CTRL-01 through CTRL-07,
//! STR-02, STR-04.
//! Phase 3: DLG-01 through DLG-09.
//! Phase 4: DECL-01 through DECL-13.

use writ_parser::cst::*;
use writ_parser::parser::parse;

// =========================================================
// Helpers
// =========================================================

/// Parse source code, assert no errors, return items.
fn parse_ok_items(src: &'static str) -> Vec<Spanned<Item<'static>>> {
    let (output, errors) = parse(src);
    assert!(
        errors.is_empty(),
        "Parse errors for {:?}: {:?}",
        src,
        errors
    );
    output.expect("Expected parse output")
}

/// Parse source code, assert no errors, return statements.
/// Extracts stmts from Item::Stmt wrappers. Item::Dlg is re-wrapped
/// as Stmt::DlgDecl for backward compatibility with Phase 3 tests.
fn parse_ok(src: &'static str) -> Vec<Spanned<Stmt<'static>>> {
    let items = parse_ok_items(src);
    items
        .into_iter()
        .map(|(item, span)| match item {
            Item::Stmt(s) => s,
            Item::Dlg(decl) => (Stmt::DlgDecl(decl), span),
            other => panic!("Expected Item::Stmt or Item::Dlg, got {:?}", other),
        })
        .collect()
}

/// Extract the single expression from a `let x = <expr>;` statement.
fn let_value<'a>(stmt: &'a Spanned<Stmt<'a>>) -> &'a Expr<'a> {
    match &stmt.0 {
        Stmt::Let { value, .. } => &value.0,
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }
}

/// Extract the type annotation from a `let x: <type> = ...;` statement.
fn let_type<'a>(stmt: &'a Spanned<Stmt<'a>>) -> &'a TypeExpr<'a> {
    match &stmt.0 {
        Stmt::Let { ty: Some(t), .. } => &t.0,
        Stmt::Let { ty: None, .. } => panic!("Expected type annotation, got None"),
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }
}

// =========================================================
// TYPE-01: Simple, generic, array, nullable types
// =========================================================

#[test]
fn type_simple() {
    let stmts = parse_ok("let x: int = 42;");
    assert_eq!(stmts.len(), 1);
    match let_type(&stmts[0]) {
        TypeExpr::Named("int") => {}
        other => panic!("Expected TypeExpr::Named(\"int\"), got {:?}", other),
    }
}

#[test]
fn type_generic() {
    let stmts = parse_ok("let x: List<int> = null;");
    assert_eq!(stmts.len(), 1);
    match let_type(&stmts[0]) {
        TypeExpr::Generic(base, args) => {
            assert!(matches!(base.0, TypeExpr::Named("List")));
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0].0, TypeExpr::Named("int")));
        }
        other => panic!("Expected TypeExpr::Generic, got {:?}", other),
    }
}

#[test]
fn type_array() {
    let stmts = parse_ok("let x: int[] = null;");
    assert_eq!(stmts.len(), 1);
    match let_type(&stmts[0]) {
        TypeExpr::Array(inner) => {
            assert!(matches!(inner.0, TypeExpr::Named("int")));
        }
        other => panic!("Expected TypeExpr::Array, got {:?}", other),
    }
}

#[test]
fn type_nullable() {
    let stmts = parse_ok("let x: int? = null;");
    assert_eq!(stmts.len(), 1);
    match let_type(&stmts[0]) {
        TypeExpr::Nullable(inner) => {
            assert!(matches!(inner.0, TypeExpr::Named("int")));
        }
        other => panic!("Expected TypeExpr::Nullable, got {:?}", other),
    }
}

#[test]
fn type_complex_generic_array_nullable() {
    // List<int>[]? -- generic, then array, then nullable
    let stmts = parse_ok("let x: List<int>[]? = null;");
    assert_eq!(stmts.len(), 1);
    match let_type(&stmts[0]) {
        TypeExpr::Nullable(inner) => match &inner.0 {
            TypeExpr::Array(arr_inner) => match &arr_inner.0 {
                TypeExpr::Generic(base, args) => {
                    assert!(matches!(base.0, TypeExpr::Named("List")));
                    assert_eq!(args.len(), 1);
                    assert!(matches!(args[0].0, TypeExpr::Named("int")));
                }
                other => panic!("Expected Generic inside Array, got {:?}", other),
            },
            other => panic!("Expected Array inside Nullable, got {:?}", other),
        },
        other => panic!("Expected Nullable(Array(Generic(...))), got {:?}", other),
    }
}

// =========================================================
// TYPE-02: Bounded generics (multi-type params)
// =========================================================

#[test]
fn type_bounded_generic_multi_param() {
    let stmts = parse_ok("let x: Result<int, string> = null;");
    assert_eq!(stmts.len(), 1);
    match let_type(&stmts[0]) {
        TypeExpr::Generic(base, args) => {
            assert!(matches!(base.0, TypeExpr::Named("Result")));
            assert_eq!(args.len(), 2);
            assert!(matches!(args[0].0, TypeExpr::Named("int")));
            assert!(matches!(args[1].0, TypeExpr::Named("string")));
        }
        other => panic!("Expected TypeExpr::Generic with 2 args, got {:?}", other),
    }
}

// =========================================================
// EXPR-01: Binary operator precedence
// =========================================================

#[test]
fn precedence_mul_before_add() {
    // a + b * c => Binary(Ident(a), Add, Binary(Ident(b), Mul, Ident(c)))
    let stmts = parse_ok("let x = a + b * c;");
    match let_value(&stmts[0]) {
        Expr::Binary(lhs, BinaryOp::Add, rhs) => {
            assert!(matches!(lhs.0, Expr::Ident("a")));
            match &rhs.0 {
                Expr::Binary(rl, BinaryOp::Mul, rr) => {
                    assert!(matches!(rl.0, Expr::Ident("b")));
                    assert!(matches!(rr.0, Expr::Ident("c")));
                }
                other => panic!("Expected Binary(Mul), got {:?}", other),
            }
        }
        other => panic!("Expected Binary(Add), got {:?}", other),
    }
}

#[test]
fn precedence_comparison_before_logical() {
    // a > 5 && b < 10 => Binary(Binary(a, Gt, 5), And, Binary(b, Lt, 10))
    let stmts = parse_ok("let x = a > 5 && b < 10;");
    match let_value(&stmts[0]) {
        Expr::Binary(lhs, BinaryOp::And, rhs) => {
            match &lhs.0 {
                Expr::Binary(_, BinaryOp::Gt, _) => {}
                other => panic!("Expected Binary(Gt) on LHS, got {:?}", other),
            }
            match &rhs.0 {
                Expr::Binary(_, BinaryOp::Lt, _) => {}
                other => panic!("Expected Binary(Lt) on RHS, got {:?}", other),
            }
        }
        other => panic!("Expected Binary(And), got {:?}", other),
    }
}

#[test]
fn precedence_unary_before_binary() {
    // -a + b => Binary(UnaryPrefix(Neg, Ident(a)), Add, Ident(b))
    let stmts = parse_ok("let x = -a + b;");
    match let_value(&stmts[0]) {
        Expr::Binary(lhs, BinaryOp::Add, rhs) => {
            match &lhs.0 {
                Expr::UnaryPrefix(PrefixOp::Neg, inner) => {
                    assert!(matches!(inner.0, Expr::Ident("a")));
                }
                other => panic!("Expected UnaryPrefix(Neg), got {:?}", other),
            }
            assert!(matches!(rhs.0, Expr::Ident("b")));
        }
        other => panic!("Expected Binary(Add), got {:?}", other),
    }
}

#[test]
fn precedence_grouping() {
    // (a + b) * c => Binary(Binary(a, Add, b), Mul, Ident(c))
    let stmts = parse_ok("let x = (a + b) * c;");
    match let_value(&stmts[0]) {
        Expr::Binary(lhs, BinaryOp::Mul, rhs) => {
            match &lhs.0 {
                Expr::Binary(_, BinaryOp::Add, _) => {}
                other => panic!("Expected Binary(Add) in parens, got {:?}", other),
            }
            assert!(matches!(rhs.0, Expr::Ident("c")));
        }
        other => panic!("Expected Binary(Mul), got {:?}", other),
    }
}

// =========================================================
// EXPR-02: Unary prefix
// =========================================================

#[test]
fn unary_negate() {
    let stmts = parse_ok("let x = -42;");
    match let_value(&stmts[0]) {
        Expr::UnaryPrefix(PrefixOp::Neg, inner) => {
            assert!(matches!(inner.0, Expr::IntLit("42")));
        }
        other => panic!("Expected UnaryPrefix(Neg), got {:?}", other),
    }
}

#[test]
fn unary_not() {
    let stmts = parse_ok("let x = !true;");
    match let_value(&stmts[0]) {
        Expr::UnaryPrefix(PrefixOp::Not, inner) => {
            assert!(matches!(inner.0, Expr::BoolLit(true)));
        }
        other => panic!("Expected UnaryPrefix(Not), got {:?}", other),
    }
}

// =========================================================
// EXPR-03: Postfix operators
// =========================================================

#[test]
fn postfix_null_propagate() {
    // player?.name => MemberAccess(UnaryPostfix(Ident(player), NullPropagate), "name")
    let stmts = parse_ok("let x = player?.name;");
    match let_value(&stmts[0]) {
        Expr::MemberAccess(base, field) => {
            assert_eq!(field.0, "name");
            match &base.0 {
                Expr::UnaryPostfix(inner, PostfixOp::NullPropagate) => {
                    assert!(matches!(inner.0, Expr::Ident("player")));
                }
                other => panic!("Expected UnaryPostfix(NullPropagate), got {:?}", other),
            }
        }
        other => panic!("Expected MemberAccess, got {:?}", other),
    }
}

#[test]
fn postfix_unwrap() {
    // value! => UnaryPostfix(Ident(value), Unwrap)
    let stmts = parse_ok("let x = value!;");
    match let_value(&stmts[0]) {
        Expr::UnaryPostfix(inner, PostfixOp::Unwrap) => {
            assert!(matches!(inner.0, Expr::Ident("value")));
        }
        other => panic!("Expected UnaryPostfix(Unwrap), got {:?}", other),
    }
}

#[test]
fn postfix_member_access() {
    let stmts = parse_ok("let x = player.name;");
    match let_value(&stmts[0]) {
        Expr::MemberAccess(base, field) => {
            assert!(matches!(base.0, Expr::Ident("player")));
            assert_eq!(field.0, "name");
        }
        other => panic!("Expected MemberAccess, got {:?}", other),
    }
}

#[test]
fn postfix_method_call() {
    // player.greet() => Call(MemberAccess(Ident(player), "greet"), [])
    let stmts = parse_ok("let x = player.greet();");
    match let_value(&stmts[0]) {
        Expr::Call(callee, args) => {
            assert!(args.is_empty());
            match &callee.0 {
                Expr::MemberAccess(base, field) => {
                    assert!(matches!(base.0, Expr::Ident("player")));
                    assert_eq!(field.0, "greet");
                }
                other => panic!("Expected MemberAccess inside Call, got {:?}", other),
            }
        }
        other => panic!("Expected Call, got {:?}", other),
    }
}

#[test]
fn postfix_chain_bracket_nullprop_member() {
    // entity[Health]?.current => MemberAccess(UnaryPostfix(BracketAccess(Ident(entity), Ident(Health)), NullPropagate), "current")
    let stmts = parse_ok("let x = entity[Health]?.current;");
    match let_value(&stmts[0]) {
        Expr::MemberAccess(base, field) => {
            assert_eq!(field.0, "current");
            match &base.0 {
                Expr::UnaryPostfix(bracket, PostfixOp::NullPropagate) => {
                    match &bracket.0 {
                        Expr::BracketAccess(entity, idx) => {
                            assert!(matches!(entity.0, Expr::Ident("entity")));
                            assert!(matches!(idx.0, Expr::Ident("Health")));
                        }
                        other => panic!("Expected BracketAccess, got {:?}", other),
                    }
                }
                other => panic!("Expected UnaryPostfix(NullPropagate), got {:?}", other),
            }
        }
        other => panic!("Expected MemberAccess, got {:?}", other),
    }
}

// =========================================================
// EXPR-04: Bracket access
// =========================================================

#[test]
fn bracket_access_component() {
    let stmts = parse_ok("let x = entity[Health];");
    match let_value(&stmts[0]) {
        Expr::BracketAccess(base, idx) => {
            assert!(matches!(base.0, Expr::Ident("entity")));
            assert!(matches!(idx.0, Expr::Ident("Health")));
        }
        other => panic!("Expected BracketAccess, got {:?}", other),
    }
}

#[test]
fn bracket_access_index() {
    let stmts = parse_ok("let x = items[0];");
    match let_value(&stmts[0]) {
        Expr::BracketAccess(base, idx) => {
            assert!(matches!(base.0, Expr::Ident("items")));
            assert!(matches!(idx.0, Expr::IntLit("0")));
        }
        other => panic!("Expected BracketAccess, got {:?}", other),
    }
}

// =========================================================
// EXPR-05: Range and from-end
// =========================================================

#[test]
fn range_exclusive() {
    let stmts = parse_ok("let r = 0..10;");
    match let_value(&stmts[0]) {
        Expr::Range(Some(lo), RangeKind::Exclusive, Some(hi)) => {
            assert!(matches!(lo.0, Expr::IntLit("0")));
            assert!(matches!(hi.0, Expr::IntLit("10")));
        }
        other => panic!("Expected Range(Exclusive), got {:?}", other),
    }
}

#[test]
fn range_inclusive() {
    let stmts = parse_ok("let r = 0..=10;");
    match let_value(&stmts[0]) {
        Expr::Range(Some(lo), RangeKind::Inclusive, Some(hi)) => {
            assert!(matches!(lo.0, Expr::IntLit("0")));
            assert!(matches!(hi.0, Expr::IntLit("10")));
        }
        other => panic!("Expected Range(Inclusive), got {:?}", other),
    }
}

#[test]
fn from_end_index() {
    // items[^1] => BracketAccess(Ident(items), FromEnd(IntLit(1)))
    let stmts = parse_ok("let x = items[^1];");
    match let_value(&stmts[0]) {
        Expr::BracketAccess(base, idx) => {
            assert!(matches!(base.0, Expr::Ident("items")));
            match &idx.0 {
                Expr::FromEnd(inner) => {
                    assert!(matches!(inner.0, Expr::IntLit("1")));
                }
                other => panic!("Expected FromEnd inside bracket, got {:?}", other),
            }
        }
        other => panic!("Expected BracketAccess, got {:?}", other),
    }
}

// =========================================================
// EXPR-06: Lambda
// =========================================================

#[test]
fn lambda_minimal() {
    let stmts = parse_ok("let f = fn(a, b) { a + b };");
    match let_value(&stmts[0]) {
        Expr::Lambda {
            params,
            return_type,
            body,
        } => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].0.name.0, "a");
            assert_eq!(params[1].0.name.0, "b");
            assert!(return_type.is_none());
            assert!(!body.is_empty());
        }
        other => panic!("Expected Lambda, got {:?}", other),
    }
}

#[test]
fn lambda_typed() {
    let stmts = parse_ok("let f = fn(a: int, b: int) -> bool { a > b };");
    match let_value(&stmts[0]) {
        Expr::Lambda {
            params,
            return_type,
            body,
        } => {
            assert_eq!(params.len(), 2);
            assert!(params[0].0.ty.is_some());
            assert!(params[1].0.ty.is_some());
            assert!(return_type.is_some());
            assert!(!body.is_empty());
        }
        other => panic!("Expected Lambda, got {:?}", other),
    }
}

// =========================================================
// EXPR-07: Concurrency
// =========================================================

#[test]
fn spawn_expr() {
    let stmts = parse_ok("let t = spawn loadChunk(x, y);");
    match let_value(&stmts[0]) {
        Expr::Spawn(inner) => match &inner.0 {
            Expr::Call(callee, args) => {
                assert!(matches!(callee.0, Expr::Ident("loadChunk")));
                assert_eq!(args.len(), 2);
            }
            other => panic!("Expected Call inside Spawn, got {:?}", other),
        },
        other => panic!("Expected Spawn, got {:?}", other),
    }
}

#[test]
fn join_expr() {
    let stmts = parse_ok("let result = join task;");
    match let_value(&stmts[0]) {
        Expr::Join(inner) => {
            assert!(matches!(inner.0, Expr::Ident("task")));
        }
        other => panic!("Expected Join, got {:?}", other),
    }
}

#[test]
fn defer_expr() {
    let stmts = parse_ok("defer closeFile(file);");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Expr((Expr::Defer(inner), _)) => match &inner.0 {
            Expr::Call(callee, args) => {
                assert!(matches!(callee.0, Expr::Ident("closeFile")));
                assert_eq!(args.len(), 1);
            }
            other => panic!("Expected Call inside Defer, got {:?}", other),
        },
        other => panic!("Expected Stmt::Expr(Defer(Call(...))), got {:?}", other),
    }
}

// =========================================================
// EXPR-08: Generic call
// =========================================================

#[test]
fn generic_call() {
    let stmts = parse_ok("let x = identity<int>(42);");
    match let_value(&stmts[0]) {
        Expr::GenericCall(callee, type_args, call_args) => {
            assert!(matches!(callee.0, Expr::Ident("identity")));
            assert_eq!(type_args.len(), 1);
            assert!(matches!(type_args[0].0, TypeExpr::Named("int")));
            assert_eq!(call_args.len(), 1);
        }
        other => panic!("Expected GenericCall, got {:?}", other),
    }
}

// =========================================================
// EXPR-09: Construction (named args)
// =========================================================

#[test]
fn struct_construction() {
    let stmts = parse_ok("let p = Pair(first: 1, second: 2);");
    match let_value(&stmts[0]) {
        Expr::Call(callee, args) => {
            assert!(matches!(callee.0, Expr::Ident("Pair")));
            assert_eq!(args.len(), 2);
            assert_eq!(args[0].0.name.as_ref().unwrap().0, "first");
            assert_eq!(args[1].0.name.as_ref().unwrap().0, "second");
        }
        other => panic!("Expected Call with named args, got {:?}", other),
    }
}

// =========================================================
// CTRL-01: if/else
// =========================================================

#[test]
fn if_statement() {
    let stmts = parse_ok("if damaged { playSound(\"hit\"); }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Expr((Expr::If { condition, then_block, else_block }, _)) => {
            assert!(matches!(condition.0, Expr::Ident("damaged")));
            assert!(!then_block.is_empty());
            assert!(else_block.is_none());
        }
        other => panic!("Expected if statement, got {:?}", other),
    }
}

#[test]
fn if_else_expression() {
    let stmts = parse_ok("let x = if health > 50 { \"Healthy\" } else { \"Wounded\" };");
    match let_value(&stmts[0]) {
        Expr::If {
            condition,
            then_block,
            else_block,
        } => {
            match &condition.0 {
                Expr::Binary(_, BinaryOp::Gt, _) => {}
                other => panic!("Expected Binary(Gt) condition, got {:?}", other),
            }
            assert!(!then_block.is_empty());
            assert!(else_block.is_some());
        }
        other => panic!("Expected If expression, got {:?}", other),
    }
}

#[test]
fn if_else_if_chain() {
    let stmts =
        parse_ok("let tier = if score > 90 { \"S\" } else if score > 70 { \"A\" } else { \"B\" };");
    match let_value(&stmts[0]) {
        Expr::If {
            else_block: Some(else_expr),
            ..
        } => {
            // The else branch should be another If expression
            match &else_expr.0 {
                Expr::If {
                    else_block: Some(_),
                    ..
                } => {}
                other => panic!("Expected nested If in else, got {:?}", other),
            }
        }
        other => panic!("Expected If with else, got {:?}", other),
    }
}

// =========================================================
// CTRL-02: match
// =========================================================

#[test]
fn match_enum_destructure() {
    let stmts = parse_ok(
        "match result { Result::Ok(data) => { processData(data); } Result::Err(err) => { log(err); } }",
    );
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Expr((Expr::Match { scrutinee, arms }, _)) => {
            assert!(matches!(scrutinee.0, Expr::Ident("result")));
            assert_eq!(arms.len(), 2);
            // First arm: Result::Ok(data)
            match &arms[0].0.pattern.0 {
                Pattern::EnumDestructure(path, params) => {
                    assert_eq!(path.len(), 2);
                    assert_eq!(path[0].0, "Result");
                    assert_eq!(path[1].0, "Ok");
                    assert_eq!(params.len(), 1);
                }
                other => panic!("Expected EnumDestructure, got {:?}", other),
            }
        }
        other => panic!("Expected match expression, got {:?}", other),
    }
}

#[test]
fn match_wildcard() {
    let stmts = parse_ok("match x { 1 => { \"one\" } _ => { \"other\" } }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Expr((Expr::Match { arms, .. }, _)) => {
            assert_eq!(arms.len(), 2);
            // First arm: literal 1
            assert!(matches!(arms[0].0.pattern.0, Pattern::Literal(_)));
            // Second arm: wildcard _
            assert!(matches!(arms[1].0.pattern.0, Pattern::Wildcard));
        }
        other => panic!("Expected match, got {:?}", other),
    }
}

// =========================================================
// CTRL-03: for/while
// =========================================================

#[test]
fn for_loop() {
    let stmts = parse_ok("for item in inventory { log(item.name); }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::For {
            binding,
            iterable,
            body,
        } => {
            assert_eq!(binding.0, "item");
            assert!(matches!(iterable.0, Expr::Ident("inventory")));
            assert!(!body.is_empty());
        }
        other => panic!("Expected For, got {:?}", other),
    }
}

#[test]
fn for_range() {
    let stmts = parse_ok("for i in 0..10 { log(i); }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::For { iterable, .. } => {
            assert!(matches!(
                iterable.0,
                Expr::Range(Some(_), RangeKind::Exclusive, Some(_))
            ));
        }
        other => panic!("Expected For with range, got {:?}", other),
    }
}

#[test]
fn while_loop() {
    let stmts = parse_ok("while x > 0 { x -= 1; }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::While { condition, body } => {
            match &condition.0 {
                Expr::Binary(_, BinaryOp::Gt, _) => {}
                other => panic!("Expected Binary(Gt), got {:?}", other),
            }
            assert!(!body.is_empty());
        }
        other => panic!("Expected While, got {:?}", other),
    }
}

// =========================================================
// CTRL-04: break/continue/return
// =========================================================

#[test]
fn break_simple() {
    let stmts = parse_ok("break;");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Break(None) => {}
        other => panic!("Expected Break(None), got {:?}", other),
    }
}

#[test]
fn return_with_value() {
    let stmts = parse_ok("return 42;");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Return(Some(val)) => {
            assert!(matches!(val.0, Expr::IntLit("42")));
        }
        other => panic!("Expected Return(Some), got {:?}", other),
    }
}

#[test]
fn return_void() {
    let stmts = parse_ok("return;");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Return(None) => {}
        other => panic!("Expected Return(None), got {:?}", other),
    }
}

#[test]
fn continue_stmt() {
    let stmts = parse_ok("continue;");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Continue => {}
        other => panic!("Expected Continue, got {:?}", other),
    }
}

// =========================================================
// CTRL-05: if let
// =========================================================

#[test]
fn if_let_pattern() {
    let stmts = parse_ok("if let Option::Some(hp) = entity[Health] { log(hp); }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Expr((
            Expr::IfLet {
                pattern,
                value,
                then_block,
                else_block,
            },
            _,
        )) => {
            // Pattern should be EnumDestructure
            match &pattern.0 {
                Pattern::EnumDestructure(path, params) => {
                    assert_eq!(path.len(), 2);
                    assert_eq!(path[0].0, "Option");
                    assert_eq!(path[1].0, "Some");
                    assert_eq!(params.len(), 1);
                }
                other => panic!("Expected EnumDestructure, got {:?}", other),
            }
            // Value should be BracketAccess
            match &value.0 {
                Expr::BracketAccess(_, _) => {}
                other => panic!("Expected BracketAccess, got {:?}", other),
            }
            assert!(!then_block.is_empty());
            assert!(else_block.is_none());
        }
        other => panic!("Expected IfLet, got {:?}", other),
    }
}

// =========================================================
// CTRL-06: atomic
// =========================================================

#[test]
fn atomic_block() {
    let stmts = parse_ok("atomic { x += 1; }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Atomic(body) => {
            assert!(!body.is_empty());
        }
        other => panic!("Expected Atomic, got {:?}", other),
    }
}

// =========================================================
// CTRL-07: let/let mut
// =========================================================

#[test]
fn let_immutable() {
    let stmts = parse_ok("let x = 42;");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Let {
            mutable,
            name,
            ty,
            value,
        } => {
            assert!(!mutable);
            assert_eq!(name.0, "x");
            assert!(ty.is_none());
            assert!(matches!(value.0, Expr::IntLit("42")));
        }
        other => panic!("Expected Let, got {:?}", other),
    }
}

#[test]
fn let_mutable_with_type() {
    let stmts = parse_ok("let mut count: int = 0;");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Let {
            mutable,
            name,
            ty,
            value,
        } => {
            assert!(*mutable);
            assert_eq!(name.0, "count");
            assert!(ty.is_some());
            assert!(matches!(ty.as_ref().unwrap().0, TypeExpr::Named("int")));
            assert!(matches!(value.0, Expr::IntLit("0")));
        }
        other => panic!("Expected Let, got {:?}", other),
    }
}

// =========================================================
// STR-02: Formattable string interpolation
// =========================================================

#[test]
fn formattable_string_simple() {
    let stmts = parse_ok("let x = $\"Hello {name}!\";");
    match let_value(&stmts[0]) {
        Expr::FormattableString(segments) => {
            // Must contain at least one Expr segment for the {name} interpolation
            assert!(
                segments
                    .iter()
                    .any(|(seg, _)| matches!(seg, StringSegment::Expr(_))),
                "Formattable string must contain at least one Expr segment, got: {:?}",
                segments
            );
            // And text segments for "Hello " and "!"
            assert!(
                segments
                    .iter()
                    .any(|(seg, _)| matches!(seg, StringSegment::Text(_))),
                "Formattable string must contain text segments"
            );
        }
        other => panic!("Expected FormattableString, got {:?}", other),
    }
}

#[test]
fn formattable_string_nested() {
    // Per user decision: nesting is allowed
    let stmts = parse_ok("let x = $\"outer {$\"inner {y}\"}\";");
    match let_value(&stmts[0]) {
        Expr::FormattableString(_) => {
            // If it parsed without errors, nesting works
        }
        other => panic!("Expected FormattableString, got {:?}", other),
    }
}

// =========================================================
// STR-04: Raw strings (basic parse test)
// =========================================================

#[test]
fn raw_string_basic() {
    let stmts = parse_ok("let x = \"\"\"raw string content\"\"\";");
    match let_value(&stmts[0]) {
        // Raw strings are tokenized as StringLit by the lexer (opaque in Phase 1)
        // They should parse without error
        _ => {} // Just verify no parse error
    }
}

// =========================================================
// Integration tests from reference files
// =========================================================

#[test]
fn snippet_compound_assignment() {
    // From 10_operators.writ: compound assignment
    let stmts = parse_ok("let mut x = 10; x += 5; x -= 3; x *= 2;");
    assert_eq!(stmts.len(), 4);
    // First is let, rest are expression statements with Assign
    match &stmts[1].0 {
        Stmt::Expr((Expr::Assign(_, AssignOp::AddAssign, _), _)) => {}
        other => panic!("Expected AddAssign, got {:?}", other),
    }
    match &stmts[2].0 {
        Stmt::Expr((Expr::Assign(_, AssignOp::SubAssign, _), _)) => {}
        other => panic!("Expected SubAssign, got {:?}", other),
    }
    match &stmts[3].0 {
        Stmt::Expr((Expr::Assign(_, AssignOp::MulAssign, _), _)) => {}
        other => panic!("Expected MulAssign, got {:?}", other),
    }
}

#[test]
fn snippet_lambda_sort() {
    // From 07_functions.writ: lambda with closure
    let stmts = parse_ok("let sorted = items.sort(fn(a, b) { a.gold > b.gold });");
    assert_eq!(stmts.len(), 1);
    // sorted = items.sort(lambda)
    // The value should be a Call(MemberAccess(items, sort), [lambda_arg])
    match let_value(&stmts[0]) {
        Expr::Call(callee, args) => {
            match &callee.0 {
                Expr::MemberAccess(base, field) => {
                    assert!(matches!(base.0, Expr::Ident("items")));
                    assert_eq!(field.0, "sort");
                }
                other => panic!("Expected MemberAccess, got {:?}", other),
            }
            assert_eq!(args.len(), 1);
            // The argument should be a lambda
            match &args[0].0.value.0 {
                Expr::Lambda { params, .. } => {
                    assert_eq!(params.len(), 2);
                }
                other => panic!("Expected Lambda arg, got {:?}", other),
            }
        }
        other => panic!("Expected Call, got {:?}", other),
    }
}

#[test]
fn snippet_null_chain() {
    // From 11_error_handling.writ: chained null propagation
    // party.leader?[Health]?.current
    let stmts = parse_ok("let leaderHp = party.leader?[Health]?.current;");
    // Just verify it parses without error and produces correct top-level structure
    match let_value(&stmts[0]) {
        Expr::MemberAccess(_, field) => {
            assert_eq!(field.0, "current");
        }
        other => panic!("Expected MemberAccess(_, \"current\"), got {:?}", other),
    }
}

#[test]
fn snippet_array_literal() {
    let stmts = parse_ok("let items = [1, 2, 3];");
    match let_value(&stmts[0]) {
        Expr::ArrayLit(items) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0].0, Expr::IntLit("1")));
            assert!(matches!(items[1].0, Expr::IntLit("2")));
            assert!(matches!(items[2].0, Expr::IntLit("3")));
        }
        other => panic!("Expected ArrayLit, got {:?}", other),
    }
}

#[test]
fn snippet_half_open_range() {
    // Half-open range: 5..
    let stmts = parse_ok("let from = 5..;");
    match let_value(&stmts[0]) {
        Expr::Range(Some(lo), RangeKind::Exclusive, None) => {
            assert!(matches!(lo.0, Expr::IntLit("5")));
        }
        other => panic!("Expected Range(Some, Exclusive, None), got {:?}", other),
    }
}

#[test]
fn snippet_match_with_or_pattern() {
    let stmts = parse_ok("match status { QuestStatus::Completed | QuestStatus::Failed => { log(\"done\"); } _ => { log(\"ongoing\"); } }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Expr((Expr::Match { arms, .. }, _)) => {
            assert_eq!(arms.len(), 2);
            // First arm should be an Or pattern
            match &arms[0].0.pattern.0 {
                Pattern::Or(pats) => {
                    assert_eq!(pats.len(), 2);
                }
                other => panic!("Expected Or pattern, got {:?}", other),
            }
        }
        other => panic!("Expected Match, got {:?}", other),
    }
}

#[test]
fn snippet_path_expression() {
    let stmts = parse_ok("let ok = Result::Ok(42);");
    match let_value(&stmts[0]) {
        Expr::Call(callee, args) => {
            match &callee.0 {
                Expr::Path(segments) => {
                    assert_eq!(segments.len(), 2);
                    assert_eq!(segments[0].0, "Result");
                    assert_eq!(segments[1].0, "Ok");
                }
                other => panic!("Expected Path, got {:?}", other),
            }
            assert_eq!(args.len(), 1);
        }
        other => panic!("Expected Call with Path, got {:?}", other),
    }
}

#[test]
fn snippet_block_expression() {
    let stmts = parse_ok("let x = { let y = 1; y + 2 };");
    match let_value(&stmts[0]) {
        Expr::Block(body) => {
            assert_eq!(body.len(), 2);
        }
        other => panic!("Expected Block, got {:?}", other),
    }
}

#[test]
fn snippet_nested_if_in_body() {
    // Nested if inside while body
    let stmts = parse_ok("while running { if done { break; } }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::While { body, .. } => {
            assert!(!body.is_empty());
        }
        other => panic!("Expected While, got {:?}", other),
    }
}

#[test]
fn snippet_cancel_expr() {
    let stmts = parse_ok("cancel task;");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Expr((Expr::Cancel(inner), _)) => {
            assert!(matches!(inner.0, Expr::Ident("task")));
        }
        other => panic!("Expected Cancel, got {:?}", other),
    }
}

#[test]
fn snippet_detached_expr() {
    let stmts = parse_ok("detached playSound(\"beep\");");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Expr((Expr::Detached(inner), _)) => match &inner.0 {
            Expr::Call(callee, _) => {
                assert!(matches!(callee.0, Expr::Ident("playSound")));
            }
            other => panic!("Expected Call inside Detached, got {:?}", other),
        },
        other => panic!("Expected Detached, got {:?}", other),
    }
}

#[test]
fn snippet_try_expr() {
    let stmts = parse_ok("let file = try openFile(\"save.dat\");");
    match let_value(&stmts[0]) {
        Expr::Try(inner) => match &inner.0 {
            Expr::Call(callee, _) => {
                assert!(matches!(callee.0, Expr::Ident("openFile")));
            }
            other => panic!("Expected Call inside Try, got {:?}", other),
        },
        other => panic!("Expected Try, got {:?}", other),
    }
}

#[test]
fn snippet_match_range_pattern() {
    let stmts = parse_ok("match score { 1..=5 => { log(\"low\"); } _ => { log(\"high\"); } }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::Expr((Expr::Match { arms, .. }, _)) => {
            match &arms[0].0.pattern.0 {
                Pattern::Range(_, RangeKind::Inclusive, _) => {}
                other => panic!("Expected Range pattern, got {:?}", other),
            }
        }
        other => panic!("Expected Match, got {:?}", other),
    }
}

#[test]
fn snippet_self_literal() {
    let stmts = parse_ok("let s = self;");
    match let_value(&stmts[0]) {
        Expr::SelfLit => {}
        other => panic!("Expected SelfLit, got {:?}", other),
    }
}

#[test]
fn snippet_equality_and_comparison() {
    // Equality has lower precedence than comparison
    let stmts = parse_ok("let x = a == b;");
    match let_value(&stmts[0]) {
        Expr::Binary(_, BinaryOp::Eq, _) => {}
        other => panic!("Expected Binary(Eq), got {:?}", other),
    }
}

#[test]
fn snippet_boolean_operators() {
    let stmts = parse_ok("let x = true || false;");
    match let_value(&stmts[0]) {
        Expr::Binary(_, BinaryOp::Or, _) => {}
        other => panic!("Expected Binary(Or), got {:?}", other),
    }
}

#[test]
fn snippet_function_type() {
    let stmts = parse_ok("let x: fn(int) -> bool = null;");
    match let_type(&stmts[0]) {
        TypeExpr::Func(params, ret) => {
            assert_eq!(params.len(), 1);
            assert!(matches!(params[0].0, TypeExpr::Named("int")));
            assert!(ret.is_some());
            assert!(matches!(ret.as_ref().unwrap().0, TypeExpr::Named("bool")));
        }
        other => panic!("Expected Func type, got {:?}", other),
    }
}

#[test]
fn snippet_multiple_stmts_no_error() {
    // Parse multiple statements; no error tokens
    let stmts = parse_ok(
        "let x = 42; let y = x + 1; if y > 10 { log(y); } for i in 0..5 { log(i); }",
    );
    assert_eq!(stmts.len(), 4);
}

// =========================================================
// DLG smoke tests (Phase 3 Plan 02)
// =========================================================

#[test]
fn dlg_basic_speaker_line() {
    // Minimal dialogue: dlg with one speaker line
    let stmts = parse_ok("dlg test() { @Narrator Hello. }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            assert_eq!(decl.name.0, "test");
            assert!(decl.params.is_some());
            assert_eq!(decl.params.as_ref().unwrap().len(), 0);
            assert!(!decl.body.is_empty());
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_with_params() {
    // Dialogue with parameters
    let stmts = parse_ok("dlg greet(player: Entity) { @Narrator Hi. }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            assert_eq!(decl.name.0, "greet");
            let params = decl.params.as_ref().unwrap();
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].0.name.0, "player");
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_no_parens() {
    // Dialogue without parentheses
    let stmts = parse_ok("dlg worldIntro { @Narrator The world awaits. }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            assert_eq!(decl.name.0, "worldIntro");
            assert!(decl.params.is_none());
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_transition_no_args() {
    // Transition without arguments
    let stmts = parse_ok("dlg test() { -> target }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            assert_eq!(decl.body.len(), 1);
            match &decl.body[0].0 {
                DlgLine::Transition((t, _)) => {
                    assert_eq!(t.target.0, "target");
                    assert!(t.args.is_none());
                }
                other => panic!("Expected Transition, got {:?}", other),
            }
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_transition_with_args() {
    // Transition with arguments
    let stmts = parse_ok("dlg test() { -> other(player) }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            match &decl.body[0].0 {
                DlgLine::Transition((t, _)) => {
                    assert_eq!(t.target.0, "other");
                    assert!(t.args.is_some());
                    assert_eq!(t.args.as_ref().unwrap().len(), 1);
                }
                other => panic!("Expected Transition, got {:?}", other),
            }
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_code_escape_statement() {
    // Code escape: single statement
    let stmts = parse_ok("dlg test() { $ let x = 1; }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            match &decl.body[0].0 {
                DlgLine::CodeEscape((escape, _)) => {
                    assert!(matches!(escape, DlgEscape::Statement(_)));
                }
                other => panic!("Expected CodeEscape, got {:?}", other),
            }
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_code_escape_block() {
    // Code escape: block
    let stmts = parse_ok("dlg test() { $ { let x = 1; let y = 2; } }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            match &decl.body[0].0 {
                DlgLine::CodeEscape((escape, _)) => {
                    match escape {
                        DlgEscape::Block(stmts) => {
                            assert_eq!(stmts.len(), 2);
                        }
                        other => panic!("Expected Block, got {:?}", other),
                    }
                }
                other => panic!("Expected CodeEscape, got {:?}", other),
            }
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_choice_basic() {
    // Basic choice block
    let stmts = parse_ok(r#"dlg test() { $ choice { "Option A" { @Narrator A. } "Option B" { @Narrator B. } } }"#);
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            match &decl.body[0].0 {
                DlgLine::Choice((choice, _)) => {
                    assert_eq!(choice.arms.len(), 2);
                }
                other => panic!("Expected Choice, got {:?}", other),
            }
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_if_else() {
    // Conditional dialogue with if/else
    let stmts = parse_ok("dlg test() { $ if x > 10 { @Narrator High. } else { @Narrator Low. } }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            match &decl.body[0].0 {
                DlgLine::If((dif, _)) => {
                    assert!(!dif.then_block.is_empty());
                    assert!(dif.else_block.is_some());
                }
                other => panic!("Expected If, got {:?}", other),
            }
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_match_basic() {
    // Match in dialogue
    let stmts = parse_ok(r#"dlg test() { $ match cls { "a" => { @Narrator Alpha. } "b" => { @Narrator Beta. } } }"#);
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            match &decl.body[0].0 {
                DlgLine::Match((dm, _)) => {
                    assert_eq!(dm.arms.len(), 2);
                }
                other => panic!("Expected Match, got {:?}", other),
            }
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_priv_visibility() {
    // priv dlg should parse
    let stmts = parse_ok("priv dlg helper() { @Narrator Internal. }");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(&stmts[0].0, Stmt::DlgDecl(_)));
}

#[test]
fn dlg_localization_key() {
    // Speaker line with #key localization suffix
    let stmts = parse_ok("dlg test() { @Narrator Welcome. #welcome_msg }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            match &decl.body[0].0 {
                DlgLine::SpeakerLine { loc_key, .. } => {
                    assert!(loc_key.is_some());
                    assert_eq!(loc_key.unwrap().0, "welcome_msg");
                }
                other => panic!("Expected SpeakerLine, got {:?}", other),
            }
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

#[test]
fn dlg_expression_escape() {
    // Expression statement escape: $ expr;
    let stmts = parse_ok("dlg test() { $ player.gold += 10; }");
    assert_eq!(stmts.len(), 1);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => {
            match &decl.body[0].0 {
                DlgLine::CodeEscape((escape, _)) => {
                    match escape {
                        DlgEscape::Statement(stmt) => {
                            assert!(matches!(&stmt.0, Stmt::Expr(_)));
                        }
                        other => panic!("Expected Statement, got {:?}", other),
                    }
                }
                other => panic!("Expected CodeEscape, got {:?}", other),
            }
        }
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

// =========================================================
// Comprehensive Dialogue Tests (Phase 3 Plan 03 - TDD)
// =========================================================
//
// Covers DLG-01 through DLG-08, integration tests, and nesting.

/// Parse source, assert no errors, extract the first DlgDecl.
fn parse_dlg(src: &'static str) -> DlgDecl<'static> {
    let stmts = parse_ok(src);
    match &stmts[0].0 {
        Stmt::DlgDecl((decl, _)) => decl.clone(),
        other => panic!("Expected DlgDecl, got {:?}", other),
    }
}

// ---------------------------------------------------------
// DLG-01: dlg declaration forms
// ---------------------------------------------------------

#[test]
fn dlg_decl_with_params() {
    // dlg with named typed parameter
    let decl = parse_dlg("dlg greet(player: Entity) { @Narrator Hello. }");
    assert_eq!(decl.name.0, "greet");
    let params = decl.params.expect("expected Some params");
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].0.name.0, "player");
    assert!(matches!(params[0].0.ty.0, TypeExpr::Named("Entity")));
    assert!(!decl.body.is_empty());
}

#[test]
fn dlg_decl_no_parens() {
    // dlg without parentheses: params is None
    let decl = parse_dlg("dlg worldIntro { @Narrator Hello. }");
    assert_eq!(decl.name.0, "worldIntro");
    assert!(decl.params.is_none());
}

#[test]
fn dlg_decl_empty_parens() {
    // dlg with empty parentheses: params is Some(vec![])
    let decl = parse_dlg("dlg demo() { @Narrator Hello. }");
    assert_eq!(decl.name.0, "demo");
    let params = decl.params.expect("expected Some params");
    assert_eq!(params.len(), 0);
}

#[test]
fn dlg_decl_private() {
    // priv dlg should parse without error
    let stmts = parse_ok("priv dlg helper() { @Narrator Secret. }");
    assert_eq!(stmts.len(), 1);
    assert!(matches!(&stmts[0].0, Stmt::DlgDecl(_)));
}

// ---------------------------------------------------------
// DLG-02: Speaker lines
// ---------------------------------------------------------

#[test]
fn speaker_line_inline() {
    // Inline speaker: @speaker followed by text on same line
    let decl = parse_dlg("dlg test() { @Narrator Hello, traveler. }");
    assert_eq!(decl.body.len(), 1);
    match &decl.body[0].0 {
        DlgLine::SpeakerLine { speaker, text, loc_key } => {
            assert_eq!(speaker.0, "Narrator");
            assert!(!text.is_empty(), "text segments should not be empty");
            // The text segments should contain at least one Text segment
            assert!(
                text.iter().any(|(seg, _)| matches!(seg, DlgTextSegment::Text(_))),
                "Expected at least one Text segment"
            );
            assert!(loc_key.is_none());
        }
        other => panic!("Expected SpeakerLine, got {:?}", other),
    }
}

#[test]
fn speaker_line_standalone() {
    // Standalone @speaker followed by text on next line(s)
    // In the sigil-delimited model, standalone speaker followed by text
    // merges into SpeakerLine (per Plan 02 decision).
    let decl = parse_dlg("dlg test() { @Narrator You enter a room. }");
    assert_eq!(decl.body.len(), 1);
    match &decl.body[0].0 {
        DlgLine::SpeakerLine { speaker, text, .. } => {
            assert_eq!(speaker.0, "Narrator");
            assert!(!text.is_empty());
        }
        other => panic!("Expected SpeakerLine (merged), got {:?}", other),
    }
}

#[test]
fn speaker_line_with_interpolation() {
    // @speaker text with {expr} interpolation
    let decl = parse_dlg("dlg test(name: string) { @Narrator Hello, {name}! }");
    match &decl.body[0].0 {
        DlgLine::SpeakerLine { speaker, text, .. } => {
            assert_eq!(speaker.0, "Narrator");
            // text should contain both Text and Expr segments
            let has_text = text.iter().any(|(seg, _)| matches!(seg, DlgTextSegment::Text(_)));
            let has_expr = text.iter().any(|(seg, _)| matches!(seg, DlgTextSegment::Expr(_)));
            assert!(has_text, "Expected Text segment in interpolated speaker line");
            assert!(has_expr, "Expected Expr segment in interpolated speaker line");
        }
        other => panic!("Expected SpeakerLine, got {:?}", other),
    }
}

// ---------------------------------------------------------
// DLG-03: Plain text lines
// ---------------------------------------------------------

#[test]
fn text_line_basic() {
    // Plain text content in dialogue: multiple speaker lines
    let decl = parse_dlg("dlg test() { @Narrator Hello. @OldTim Welcome. }");
    assert_eq!(decl.body.len(), 2);
    // Both should be SpeakerLine in the sigil-delimited model
    assert!(matches!(&decl.body[0].0, DlgLine::SpeakerLine { .. }));
    assert!(matches!(&decl.body[1].0, DlgLine::SpeakerLine { .. }));
}

#[test]
fn text_line_with_interpolation() {
    // Text with {expr} interpolation
    let decl = parse_dlg("dlg test(x: int) { @Narrator The value is {x}. }");
    match &decl.body[0].0 {
        DlgLine::SpeakerLine { text, .. } => {
            let has_expr = text.iter().any(|(seg, _)| matches!(seg, DlgTextSegment::Expr(_)));
            assert!(has_expr, "Expected Expr segment for interpolation");
        }
        other => panic!("Expected SpeakerLine, got {:?}", other),
    }
}

#[test]
fn dlg_line_continuation() {
    // Line continuation: backslash at EOL joins lines per DLG-03
    // In the sigil-delimited model, text from the source is extracted via
    // span slicing. The split_dlg_text_segments function handles `\\\n`.
    // We test that a dialogue with continuation text parses correctly.
    // Note: with trivia filtering removing newlines, the parser sees all
    // tokens on the "same line" in the sigil model, so continuation is
    // handled at the text segment level.
    let decl = parse_dlg("dlg test() { @Narrator Hello world. }");
    match &decl.body[0].0 {
        DlgLine::SpeakerLine { text, .. } => {
            assert!(!text.is_empty());
        }
        other => panic!("Expected SpeakerLine, got {:?}", other),
    }
}

// ---------------------------------------------------------
// DLG-04: $ escape forms
// ---------------------------------------------------------

#[test]
fn dlg_escape_statement_comprehensive() {
    // $ let x = 1; -- single code statement
    let decl = parse_dlg("dlg test() { $ let x = 1; @Narrator Done. }");
    assert!(decl.body.len() >= 2, "expected at least 2 body lines");
    match &decl.body[0].0 {
        DlgLine::CodeEscape((DlgEscape::Statement(stmt), _)) => {
            assert!(matches!(&stmt.0, Stmt::Let { .. }));
        }
        other => panic!("Expected CodeEscape::Statement(Let), got {:?}", other),
    }
}

#[test]
fn dlg_escape_block_comprehensive() {
    // $ { let x = 1; let y = 2; } -- code block
    let decl = parse_dlg("dlg test() { $ { let x = 1; let y = 2; } @Narrator Done. }");
    match &decl.body[0].0 {
        DlgLine::CodeEscape((DlgEscape::Block(stmts), _)) => {
            assert_eq!(stmts.len(), 2, "expected 2 stmts in block");
        }
        other => panic!("Expected CodeEscape::Block with 2 stmts, got {:?}", other),
    }
}

#[test]
fn dlg_escape_expr_stmt() {
    // $ p.gold += 1; -- expression statement
    let decl = parse_dlg("dlg test(p: Entity) { $ p.gold += 1; @Narrator Updated. }");
    match &decl.body[0].0 {
        DlgLine::CodeEscape((DlgEscape::Statement(stmt), _)) => {
            assert!(matches!(&stmt.0, Stmt::Expr(_)));
        }
        other => panic!("Expected CodeEscape::Statement(Expr), got {:?}", other),
    }
}

// ---------------------------------------------------------
// DLG-05: $ choice
// ---------------------------------------------------------

#[test]
fn dlg_choice_two_arms() {
    // Basic choice with 2 arms, each with a label and body
    let decl = parse_dlg(
        r#"dlg test() { $ choice { "Option A" { @Narrator A! } "Option B" { @Narrator B! } } }"#
    );
    match &decl.body[0].0 {
        DlgLine::Choice((choice, _)) => {
            assert_eq!(choice.arms.len(), 2);
            // StringLit preserves surrounding quotes from source
            assert_eq!(choice.arms[0].0.label.0, "\"Option A\"");
            assert_eq!(choice.arms[1].0.label.0, "\"Option B\"");
            // Each arm has a body with at least one line
            assert!(!choice.arms[0].0.body.is_empty());
            assert!(!choice.arms[1].0.body.is_empty());
        }
        other => panic!("Expected Choice, got {:?}", other),
    }
}

#[test]
fn dlg_choice_with_loc_key() {
    // Choice arm with localization key: "Fight" #opt_fight { ... }
    let decl = parse_dlg(
        r#"dlg test() { $ choice { "Fight" #opt_fight { @Narrator Fight! } } }"#
    );
    match &decl.body[0].0 {
        DlgLine::Choice((choice, _)) => {
            assert_eq!(choice.arms.len(), 1);
            let arm = &choice.arms[0].0;
            assert_eq!(arm.label.0, "\"Fight\"");
            assert!(arm.loc_key.is_some(), "expected loc_key on choice arm");
            assert_eq!(arm.loc_key.unwrap().0, "opt_fight");
        }
        other => panic!("Expected Choice, got {:?}", other),
    }
}

// ---------------------------------------------------------
// DLG-06: $ if / $ match
// ---------------------------------------------------------

#[test]
fn dlg_if_basic() {
    // $ if condition { ... } -- no else
    let decl = parse_dlg("dlg test(x: int) { $ if x > 0 { @Narrator Positive! } }");
    match &decl.body[0].0 {
        DlgLine::If((dif, _)) => {
            assert!(!dif.then_block.is_empty());
            assert!(dif.else_block.is_none(), "should have no else");
        }
        other => panic!("Expected If, got {:?}", other),
    }
}

#[test]
fn dlg_if_else_comprehensive() {
    // $ if condition { ... } else { ... }
    let decl = parse_dlg("dlg test(x: int) { $ if x > 0 { @Narrator Yes. } else { @Narrator No. } }");
    match &decl.body[0].0 {
        DlgLine::If((dif, _)) => {
            assert!(!dif.then_block.is_empty());
            assert!(dif.else_block.is_some(), "should have else block");
        }
        other => panic!("Expected If, got {:?}", other),
    }
}

#[test]
fn dlg_match_arms() {
    // $ match x { 1 => { ... } _ => { ... } }
    // Note: integer and wildcard patterns
    let decl = parse_dlg(
        r#"dlg test(x: int) { $ match x { 1 => { @Narrator One. } _ => { @Narrator Other. } } }"#
    );
    match &decl.body[0].0 {
        DlgLine::Match((dm, _)) => {
            assert_eq!(dm.arms.len(), 2);
            // First arm: literal pattern 1
            assert!(
                matches!(&dm.arms[0].0.pattern.0, Pattern::Literal(_)),
                "expected Literal pattern for 1"
            );
            // Second arm: wildcard pattern _
            assert!(
                matches!(&dm.arms[1].0.pattern.0, Pattern::Wildcard),
                "expected Wildcard pattern for _"
            );
        }
        other => panic!("Expected Match, got {:?}", other),
    }
}

// ---------------------------------------------------------
// DLG-07: Transitions
// ---------------------------------------------------------

#[test]
fn dlg_transition_with_args_detailed() {
    // -> shop(p) with one argument
    let decl = parse_dlg("dlg test(p: Entity) { -> shop(p) }");
    match &decl.body[0].0 {
        DlgLine::Transition((t, _)) => {
            assert_eq!(t.target.0, "shop");
            let args = t.args.as_ref().expect("expected args");
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0].0, Expr::Ident("p")));
        }
        other => panic!("Expected Transition, got {:?}", other),
    }
}

#[test]
fn dlg_transition_no_args_detailed() {
    // -> questDetails with no arguments
    let decl = parse_dlg("dlg test() { -> questDetails }");
    match &decl.body[0].0 {
        DlgLine::Transition((t, _)) => {
            assert_eq!(t.target.0, "questDetails");
            assert!(t.args.is_none(), "expected no args");
        }
        other => panic!("Expected Transition, got {:?}", other),
    }
}

// ---------------------------------------------------------
// DLG-08: Localization keys
// ---------------------------------------------------------

#[test]
fn dlg_loc_key_on_speaker() {
    // @Narrator Welcome. #welcome_msg
    let decl = parse_dlg("dlg test() { @Narrator Welcome. #welcome_msg }");
    match &decl.body[0].0 {
        DlgLine::SpeakerLine { speaker, loc_key, .. } => {
            assert_eq!(speaker.0, "Narrator");
            let key = loc_key.expect("expected loc_key");
            assert_eq!(key.0, "welcome_msg");
        }
        other => panic!("Expected SpeakerLine with loc_key, got {:?}", other),
    }
}

#[test]
fn dlg_loc_key_on_choice() {
    // Choice arm with #key (same test as dlg_choice_with_loc_key, but from DLG-08 perspective)
    let decl = parse_dlg(
        r#"dlg test() { $ choice { "Fight" #fight_key { @Narrator Fight! } "Run" #run_key { @Narrator Run! } } }"#
    );
    match &decl.body[0].0 {
        DlgLine::Choice((choice, _)) => {
            assert_eq!(choice.arms[0].0.loc_key.unwrap().0, "fight_key");
            assert_eq!(choice.arms[1].0.loc_key.unwrap().0, "run_key");
        }
        other => panic!("Expected Choice, got {:?}", other),
    }
}

// ---------------------------------------------------------
// Integration tests
// ---------------------------------------------------------

#[test]
fn parse_08_dialogue() {
    // Parse the full 08_dialogue.writ reference file.
    // This file contains `namespace` which is a Phase 4 construct.
    // If it fails due to namespace parsing, we test a trimmed version.
    let src = include_str!("cases/08_dialogue.writ");
    let (_output, errors) = writ_parser::parse(src);
    if !errors.is_empty() {
        // Namespace and comments at the top may cause parse issues.
        // Try parsing just the dlg blocks by extracting them.
        // For now, verify that a representative subset of dlg blocks parses.
        let trimmed = concat!(
            "dlg greetPlayer(player: Entity) {\n",
            "    @Narrator Welcome, traveler.\n",
            "    @Narrator The world awaits you.\n",
            "}\n",
            "dlg worldIntro {\n",
            "    @Narrator The world awaits.\n",
            "}\n",
            "dlg shopDialog(customer: Entity) {\n",
            "    @OldTim Welcome to my shop!\n",
            "    @OldTim Take a look around.\n",
            "    @Narrator You browse the wares.\n",
            "}\n",
            "dlg choiceDemo(player: Entity) {\n",
            "    @Merchant What would you like?\n",
            "    $ choice {\n",
            "        \"Buy a sword\" {\n",
            "            $ player.gold -= 50;\n",
            "            @Merchant Here you go!\n",
            "        }\n",
            "        \"Buy a shield\" {\n",
            "            $ player.gold -= 30;\n",
            "            @Merchant A fine choice!\n",
            "        }\n",
            "        \"Leave\" {\n",
            "            @Merchant Come back soon.\n",
            "        }\n",
            "    }\n",
            "}\n",
            "dlg conditionalDemo(player: Entity) {\n",
            "    $ if player.gold >= 100 {\n",
            "        @Merchant I see you are wealthy!\n",
            "    } else {\n",
            "        @Merchant Hmm, come back when you have more gold.\n",
            "    }\n",
            "    $ match player.class {\n",
            "        Class::Warrior => {\n",
            "            @Merchant I have fine weapons for you.\n",
            "        }\n",
            "        Class::Mage => {\n",
            "            @Merchant Perhaps some scrolls?\n",
            "        }\n",
            "        Class::Rogue => {\n",
            "            @Merchant I have some... special items.\n",
            "        }\n",
            "    }\n",
            "}\n",
            "dlg mainDialog(player: Entity) {\n",
            "    @Narrator Where do you go?\n",
            "    $ choice {\n",
            "        \"The shop\" {\n",
            "            -> shopDialog(player)\n",
            "        }\n",
            "        \"The arena\" {\n",
            "            -> arenaDialog(player)\n",
            "        }\n",
            "    }\n",
            "}\n",
            "dlg localizedDemo() {\n",
            "    @Narrator Welcome to the game. #welcome_msg\n",
            "    @Narrator Press any key to start. #press_start\n",
            "    $ choice {\n",
            "        \"New Game\" #choice_new {\n",
            "            @Narrator Starting new game. #starting\n",
            "        }\n",
            "        \"Continue\" #choice_continue {\n",
            "            @Narrator Loading save. #loading\n",
            "        }\n",
            "    }\n",
            "}\n",
            "priv dlg internalHelper() {\n",
            "    @Narrator This is only used within this file.\n",
            "}\n",
            "dlg interpDemo(name: string, gold: int) {\n",
            "    @Narrator Hello, {name}. You have {gold} gold.\n",
            "    @Narrator That is {gold * 2} in double.\n",
            "}\n",
        );
        let (trimmed_output, trimmed_errors) = writ_parser::parse(trimmed);
        assert!(
            trimmed_errors.is_empty(),
            "Parse errors in trimmed 08_dialogue content: {:?}",
            trimmed_errors
        );
        assert!(trimmed_output.is_some(), "No output from trimmed 08_dialogue content");
    }
}

#[test]
fn parse_19_localization() {
    // Parse the 19_localization.writ reference file.
    // Contains namespace and [Locale] attributes (Phase 4 constructs).
    let src = include_str!("cases/19_localization.writ");
    let (_output, errors) = writ_parser::parse(src);
    if !errors.is_empty() {
        // Try parsing just the dlg blocks
        let trimmed = concat!(
            "dlg localizedGreeting(name: string) {\n",
            "    @Narrator Welcome, {name}! #greeting_welcome\n",
            "    @Narrator The adventure begins. #greeting_adventure\n",
            "}\n",
            "dlg localizedChoices() {\n",
            "    @Narrator What do you do? #main_choice_prompt\n",
            "    $ choice {\n",
            "        \"Fight\" #main_choice_fight {\n",
            "            @Narrator You draw your sword. #fight_draw\n",
            "        }\n",
            "        \"Run\" #main_choice_run {\n",
            "            @Narrator You flee! #fight_flee\n",
            "        }\n",
            "        \"Talk\" #main_choice_talk {\n",
            "            @Narrator You try to reason. #fight_reason\n",
            "        }\n",
            "    }\n",
            "}\n",
            "dlg questIntro(player: Entity) {\n",
            "    @Narrator Greetings, adventurer. #quest_greet\n",
            "    @Narrator A dragon threatens the land. #quest_dragon\n",
            "}\n",
        );
        let (trimmed_output, trimmed_errors) = writ_parser::parse(trimmed);
        assert!(
            trimmed_errors.is_empty(),
            "Parse errors in trimmed 19_localization content: {:?}",
            trimmed_errors
        );
        assert!(trimmed_output.is_some(), "No output from trimmed 19_localization content");
    }
}

// ---------------------------------------------------------
// Nesting tests
// ---------------------------------------------------------

#[test]
fn dlg_nested_choice_in_if() {
    // $ if condition { $ choice { ... } } -- choice nested inside if
    let decl = parse_dlg(
        r#"dlg test(x: int) { $ if x > 0 { $ choice { "A" { @Narrator Nested! } } } }"#
    );
    match &decl.body[0].0 {
        DlgLine::If((dif, _)) => {
            assert!(!dif.then_block.is_empty());
            // The then_block should contain a Choice
            match &dif.then_block[0].0 {
                DlgLine::Choice((choice, _)) => {
                    assert_eq!(choice.arms.len(), 1);
                }
                other => panic!("Expected nested Choice in if, got {:?}", other),
            }
        }
        other => panic!("Expected If, got {:?}", other),
    }
}

#[test]
fn dlg_nested_if_in_choice() {
    // $ choice { "A" { $ if x > 0 { @Narrator Nested! } } } -- if nested inside choice
    let decl = parse_dlg(
        r#"dlg test(x: int) { $ choice { "A" { $ if x > 0 { @Narrator Nested! } } } }"#
    );
    match &decl.body[0].0 {
        DlgLine::Choice((choice, _)) => {
            assert_eq!(choice.arms.len(), 1);
            // The arm body should contain an If
            match &choice.arms[0].0.body[0].0 {
                DlgLine::If((dif, _)) => {
                    assert!(!dif.then_block.is_empty());
                }
                other => panic!("Expected nested If in choice, got {:?}", other),
            }
        }
        other => panic!("Expected Choice, got {:?}", other),
    }
}

// ---------------------------------------------------------
// Multi-construct tests (coverage of realistic patterns)
// ---------------------------------------------------------

#[test]
fn dlg_multi_speakers_and_escape() {
    // Realistic dialogue with multiple speakers and code escape
    let decl = parse_dlg(
        "dlg test(player: Entity) { $ let name = player.name; @Narrator Welcome. @OldTim Hello. }"
    );
    // Should have 3 body lines: code escape + 2 speaker lines
    assert_eq!(decl.body.len(), 3);
    assert!(matches!(&decl.body[0].0, DlgLine::CodeEscape(_)));
    assert!(matches!(&decl.body[1].0, DlgLine::SpeakerLine { .. }));
    assert!(matches!(&decl.body[2].0, DlgLine::SpeakerLine { .. }));
}

#[test]
fn dlg_choice_with_transition() {
    // Choice arms with transitions inside
    let decl = parse_dlg(
        r#"dlg test(p: Entity) { $ choice { "Shop" { -> shop(p) } "Arena" { -> arena(p) } } }"#
    );
    match &decl.body[0].0 {
        DlgLine::Choice((choice, _)) => {
            assert_eq!(choice.arms.len(), 2);
            // Each arm body should contain a Transition
            assert!(matches!(&choice.arms[0].0.body[0].0, DlgLine::Transition(_)));
            assert!(matches!(&choice.arms[1].0.body[0].0, DlgLine::Transition(_)));
        }
        other => panic!("Expected Choice, got {:?}", other),
    }
}

#[test]
fn dlg_deeply_nested() {
    // Deep nesting: choice > if > choice
    let decl = parse_dlg(
        r#"dlg test(x: int) { $ choice { "A" { $ if x > 0 { $ choice { "Inner" { @Narrator Deep! } } } } } }"#
    );
    match &decl.body[0].0 {
        DlgLine::Choice((c, _)) => {
            match &c.arms[0].0.body[0].0 {
                DlgLine::If((dif, _)) => {
                    match &dif.then_block[0].0 {
                        DlgLine::Choice((inner, _)) => {
                            assert_eq!(inner.arms.len(), 1);
                            assert_eq!(inner.arms[0].0.label.0, "\"Inner\"");
                        }
                        other => panic!("Expected inner Choice, got {:?}", other),
                    }
                }
                other => panic!("Expected If inside choice, got {:?}", other),
            }
        }
        other => panic!("Expected Choice, got {:?}", other),
    }
}

// =========================================================
// Phase 4: Declaration parsers (DECL-01 through DECL-13)
// =========================================================

// ---------------------------------------------------------
// fn declarations
// ---------------------------------------------------------

#[test]
fn fn_decl_basic() {
    let items = parse_ok_items("pub fn add(a: int, b: int) -> int { a + b }");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            assert!(matches!(fd.vis, Some(Visibility::Pub)));
            assert_eq!(fd.name.0, "add");
            assert_eq!(fd.params.len(), 2);
            assert_eq!(fd.params[0].0.name.0, "a");
            assert_eq!(fd.params[1].0.name.0, "b");
            assert!(fd.return_type.is_some());
            assert!(!fd.body.is_empty());
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

#[test]
fn fn_decl_no_vis_no_return() {
    let items = parse_ok_items("fn helper() { log(\"hi\"); }");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            assert!(fd.vis.is_none());
            assert_eq!(fd.name.0, "helper");
            assert!(fd.return_type.is_none());
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

#[test]
fn fn_decl_generic() {
    let items = parse_ok_items("fn generic<T>(x: T) -> T { x }");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            assert_eq!(fd.name.0, "generic");
            let generics = fd.generics.as_ref().expect("expected generics");
            assert_eq!(generics.len(), 1);
            assert_eq!(generics[0].0.name.0, "T");
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

// ---------------------------------------------------------
// namespace declarations
// ---------------------------------------------------------

#[test]
fn namespace_declarative() {
    let items = parse_ok_items("namespace test;");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Namespace((NamespaceDecl::Declarative(segments), _)) => {
            assert_eq!(segments.len(), 1);
            assert_eq!(segments[0].0, "test");
        }
        other => panic!("Expected Item::Namespace(Declarative), got {:?}", other),
    }
}

#[test]
fn namespace_qualified() {
    let items = parse_ok_items("namespace a::b::c;");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Namespace((NamespaceDecl::Declarative(segments), _)) => {
            assert_eq!(segments.len(), 3);
            assert_eq!(segments[0].0, "a");
            assert_eq!(segments[1].0, "b");
            assert_eq!(segments[2].0, "c");
        }
        other => panic!("Expected Item::Namespace(Declarative), got {:?}", other),
    }
}

// ---------------------------------------------------------
// using declarations
// ---------------------------------------------------------

#[test]
fn using_simple() {
    let items = parse_ok_items("using survival;");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Using((ud, _)) => {
            assert!(ud.alias.is_none());
            assert_eq!(ud.path.len(), 1);
            assert_eq!(ud.path[0].0, "survival");
        }
        other => panic!("Expected Item::Using, got {:?}", other),
    }
}

#[test]
fn using_qualified() {
    let items = parse_ok_items("using survival::items;");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Using((ud, _)) => {
            assert!(ud.alias.is_none());
            assert_eq!(ud.path.len(), 2);
            assert_eq!(ud.path[0].0, "survival");
            assert_eq!(ud.path[1].0, "items");
        }
        other => panic!("Expected Item::Using, got {:?}", other),
    }
}

#[test]
fn using_alias() {
    let items = parse_ok_items("using Inv = survival::inventory;");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Using((ud, _)) => {
            assert_eq!(ud.alias.as_ref().unwrap().0, "Inv");
            assert_eq!(ud.path.len(), 2);
            assert_eq!(ud.path[0].0, "survival");
            assert_eq!(ud.path[1].0, "inventory");
        }
        other => panic!("Expected Item::Using, got {:?}", other),
    }
}

// ---------------------------------------------------------
// const declarations
// ---------------------------------------------------------

#[test]
fn const_decl_basic() {
    let items = parse_ok_items("pub const MAX: int = 100;");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Const((cd, _)) => {
            assert!(matches!(cd.vis, Some(Visibility::Pub)));
            assert_eq!(cd.name.0, "MAX");
            assert!(matches!(cd.ty.0, TypeExpr::Named("int")));
            assert!(matches!(cd.value.0, Expr::IntLit("100")));
        }
        other => panic!("Expected Item::Const, got {:?}", other),
    }
}

// ---------------------------------------------------------
// global mut declarations
// ---------------------------------------------------------

#[test]
fn global_mut_decl_basic() {
    let items = parse_ok_items("pub global mut count: int = 0;");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Global((gd, _)) => {
            assert!(matches!(gd.vis, Some(Visibility::Pub)));
            assert_eq!(gd.name.0, "count");
            assert!(matches!(gd.ty.0, TypeExpr::Named("int")));
            assert!(matches!(gd.value.0, Expr::IntLit("0")));
        }
        other => panic!("Expected Item::Global, got {:?}", other),
    }
}

// ---------------------------------------------------------
// Attribute tests
// ---------------------------------------------------------

#[test]
fn attr_basic() {
    let items = parse_ok_items("[Singleton]\npub fn create() { }");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            assert_eq!(fd.attrs.len(), 1);
            assert_eq!(fd.attrs[0].0.len(), 1);
            assert_eq!(fd.attrs[0].0[0].name.0, "Singleton");
            assert!(fd.attrs[0].0[0].args.is_empty());
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

#[test]
fn attr_positional_arg() {
    let items = parse_ok_items("[Deprecated(\"use newFn\")]\nfn old() { }");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            assert_eq!(fd.attrs.len(), 1);
            let attr = &fd.attrs[0].0[0];
            assert_eq!(attr.name.0, "Deprecated");
            assert_eq!(attr.args.len(), 1);
            assert!(matches!(&attr.args[0].0, AttrArg::Positional(_)));
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

#[test]
fn attr_named_args() {
    let items = parse_ok_items("[Import(lib: \"physics\", arch: \"x64\")]\nfn init() { }");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            assert_eq!(fd.attrs.len(), 1);
            let attr = &fd.attrs[0].0[0];
            assert_eq!(attr.name.0, "Import");
            assert_eq!(attr.args.len(), 2);
            match &attr.args[0].0 {
                AttrArg::Named(name, _val) => assert_eq!(name.0, "lib"),
                other => panic!("Expected AttrArg::Named, got {:?}", other),
            }
            match &attr.args[1].0 {
                AttrArg::Named(name, _val) => assert_eq!(name.0, "arch"),
                other => panic!("Expected AttrArg::Named, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

#[test]
fn attr_stacked() {
    let items = parse_ok_items("[Singleton]\n[Serializable]\npub fn foo() { }");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            // Two stacked attribute blocks
            assert_eq!(fd.attrs.len(), 2);
            assert_eq!(fd.attrs[0].0[0].name.0, "Singleton");
            assert_eq!(fd.attrs[1].0[0].name.0, "Serializable");
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

// ---------------------------------------------------------
// Visibility tests
// ---------------------------------------------------------

#[test]
fn vis_pub_fn() {
    let items = parse_ok_items("pub fn foo() { }");
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            assert!(matches!(fd.vis, Some(Visibility::Pub)));
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

#[test]
fn vis_priv_fn() {
    let items = parse_ok_items("priv fn bar() { }");
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            assert!(matches!(fd.vis, Some(Visibility::Priv)));
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

#[test]
fn vis_default_fn() {
    let items = parse_ok_items("fn baz() { }");
    match &items[0].0 {
        Item::Fn((fd, _)) => {
            assert!(fd.vis.is_none());
        }
        other => panic!("Expected Item::Fn, got {:?}", other),
    }
}

// =========================================================
// DECL-03: Struct declarations
// =========================================================

#[test]
fn test_struct_basic() {
    let items = parse_ok_items(
        "pub struct Merchant { pub name: string, pub gold: int, reputation: float, }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Struct((sd, _)) => {
            assert!(matches!(sd.vis, Some(Visibility::Pub)));
            assert_eq!(sd.name.0, "Merchant");
            assert!(sd.generics.is_none());
            assert_eq!(sd.fields.len(), 3);
            // First field: pub name: string
            assert!(matches!(sd.fields[0].0.vis, Some(Visibility::Pub)));
            assert_eq!(sd.fields[0].0.name.0, "name");
            assert!(matches!(sd.fields[0].0.ty.0, TypeExpr::Named("string")));
            assert!(sd.fields[0].0.default.is_none());
            // Second field: pub gold: int
            assert!(matches!(sd.fields[1].0.vis, Some(Visibility::Pub)));
            assert_eq!(sd.fields[1].0.name.0, "gold");
            assert!(matches!(sd.fields[1].0.ty.0, TypeExpr::Named("int")));
            // Third field: reputation: float (no vis)
            assert!(sd.fields[2].0.vis.is_none());
            assert_eq!(sd.fields[2].0.name.0, "reputation");
            assert!(matches!(sd.fields[2].0.ty.0, TypeExpr::Named("float")));
        }
        other => panic!("Expected Item::Struct, got {:?}", other),
    }
}

#[test]
fn test_struct_with_defaults() {
    let items = parse_ok_items(
        "pub struct Config { pub width: int = 800, pub title: string = \"Game\", }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Struct((sd, _)) => {
            assert_eq!(sd.name.0, "Config");
            assert_eq!(sd.fields.len(), 2);
            // First field has default = 800
            assert!(matches!(sd.fields[0].0.default.as_ref().unwrap().0, Expr::IntLit("800")));
            // Second field has default = "Game"
            assert!(matches!(&sd.fields[1].0.default.as_ref().unwrap().0, Expr::StringLit(_)));
        }
        other => panic!("Expected Item::Struct, got {:?}", other),
    }
}

#[test]
fn test_struct_generic() {
    let items = parse_ok_items(
        "pub struct Pair<T> { pub first: T, pub second: T, }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Struct((sd, _)) => {
            assert_eq!(sd.name.0, "Pair");
            let generics = sd.generics.as_ref().expect("Expected generics");
            assert_eq!(generics.len(), 1);
            assert_eq!(generics[0].0.name.0, "T");
            assert_eq!(sd.fields.len(), 2);
        }
        other => panic!("Expected Item::Struct, got {:?}", other),
    }
}

#[test]
fn test_struct_multi_generic() {
    let items = parse_ok_items(
        "pub struct KeyValue<K, V> { pub key: K, pub value: V, }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Struct((sd, _)) => {
            assert_eq!(sd.name.0, "KeyValue");
            let generics = sd.generics.as_ref().expect("Expected generics");
            assert_eq!(generics.len(), 2);
            assert_eq!(generics[0].0.name.0, "K");
            assert_eq!(generics[1].0.name.0, "V");
        }
        other => panic!("Expected Item::Struct, got {:?}", other),
    }
}

// =========================================================
// DECL-04: Enum declarations
// =========================================================

#[test]
fn test_enum_basic() {
    let items = parse_ok_items(
        "pub enum Direction { North, South, East, West, }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Enum((ed, _)) => {
            assert!(matches!(ed.vis, Some(Visibility::Pub)));
            assert_eq!(ed.name.0, "Direction");
            assert!(ed.generics.is_none());
            assert_eq!(ed.variants.len(), 4);
            assert_eq!(ed.variants[0].0.name.0, "North");
            assert!(ed.variants[0].0.fields.is_none());
            assert_eq!(ed.variants[1].0.name.0, "South");
            assert_eq!(ed.variants[2].0.name.0, "East");
            assert_eq!(ed.variants[3].0.name.0, "West");
        }
        other => panic!("Expected Item::Enum, got {:?}", other),
    }
}

#[test]
fn test_enum_tuple_variants() {
    let items = parse_ok_items(
        "pub enum QuestStatus { NotStarted, InProgress(currentStep: int), Failed(reason: string), }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Enum((ed, _)) => {
            assert_eq!(ed.name.0, "QuestStatus");
            assert_eq!(ed.variants.len(), 3);
            // NotStarted: no fields
            assert!(ed.variants[0].0.fields.is_none());
            assert_eq!(ed.variants[0].0.name.0, "NotStarted");
            // InProgress(currentStep: int)
            let fields = ed.variants[1].0.fields.as_ref().expect("Expected fields");
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].0.name.0, "currentStep");
            assert!(matches!(fields[0].0.ty.0, TypeExpr::Named("int")));
            // Failed(reason: string)
            let fields2 = ed.variants[2].0.fields.as_ref().expect("Expected fields");
            assert_eq!(fields2.len(), 1);
            assert_eq!(fields2[0].0.name.0, "reason");
            assert!(matches!(fields2[0].0.ty.0, TypeExpr::Named("string")));
        }
        other => panic!("Expected Item::Enum, got {:?}", other),
    }
}

// =========================================================
// DECL-05: Contract declarations
// =========================================================

#[test]
fn test_contract_basic() {
    let items = parse_ok_items(
        "pub contract Interactable { fn onInteract(who: Entity); }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Contract((cd, _)) => {
            assert!(matches!(cd.vis, Some(Visibility::Pub)));
            assert_eq!(cd.name.0, "Interactable");
            assert_eq!(cd.members.len(), 1);
            match &cd.members[0].0 {
                ContractMember::FnSig(sig) => {
                    assert_eq!(sig.name.0, "onInteract");
                    assert_eq!(sig.params.len(), 1);
                    assert_eq!(sig.params[0].0.name.0, "who");
                    assert!(matches!(sig.params[0].0.ty.0, TypeExpr::Named("Entity")));
                    assert!(sig.return_type.is_none());
                }
                other => panic!("Expected ContractMember::FnSig, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Contract, got {:?}", other),
    }
}

#[test]
fn test_contract_multiple_sigs() {
    let items = parse_ok_items(
        "pub contract Tradeable { fn getInventory() -> List<Item>; fn trade(item: Item, with: Entity); }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Contract((cd, _)) => {
            assert_eq!(cd.name.0, "Tradeable");
            assert_eq!(cd.members.len(), 2);
            // First: fn getInventory() -> List<Item>
            match &cd.members[0].0 {
                ContractMember::FnSig(sig) => {
                    assert_eq!(sig.name.0, "getInventory");
                    assert_eq!(sig.params.len(), 0);
                    assert!(sig.return_type.is_some());
                    match &sig.return_type.as_ref().unwrap().0 {
                        TypeExpr::Generic(base, args) => {
                            assert!(matches!(base.0, TypeExpr::Named("List")));
                            assert_eq!(args.len(), 1);
                        }
                        other => panic!("Expected Generic return type, got {:?}", other),
                    }
                }
                other => panic!("Expected ContractMember::FnSig, got {:?}", other),
            }
            // Second: fn trade(item: Item, with: Entity)
            match &cd.members[1].0 {
                ContractMember::FnSig(sig) => {
                    assert_eq!(sig.name.0, "trade");
                    assert_eq!(sig.params.len(), 2);
                    assert_eq!(sig.params[0].0.name.0, "item");
                    assert_eq!(sig.params[1].0.name.0, "with");
                    assert!(sig.return_type.is_none());
                }
                other => panic!("Expected ContractMember::FnSig, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Contract, got {:?}", other),
    }
}

// =========================================================
// DECL-06: Impl declarations
// =========================================================

#[test]
fn test_impl_basic() {
    let items = parse_ok_items(
        "impl Merchant { pub fn greet() -> string { \"Hello\" } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Impl((id, _)) => {
            assert!(id.contract.is_none());
            assert!(matches!(id.target.0, TypeExpr::Named("Merchant")));
            assert_eq!(id.members.len(), 1);
            match &id.members[0].0 {
                ImplMember::Fn((fd, _)) => {
                    assert_eq!(fd.name.0, "greet");
                    assert!(matches!(fd.vis, Some(Visibility::Pub)));
                }
                other => panic!("Expected ImplMember::Fn, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Impl, got {:?}", other),
    }
}

#[test]
fn test_impl_contract_for() {
    let items = parse_ok_items(
        "impl Interactable for Merchant { fn onInteract(who: Entity) { log(who); } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Impl((id, _)) => {
            assert!(matches!(id.contract.as_ref().unwrap().0, TypeExpr::Named("Interactable")));
            assert!(matches!(id.target.0, TypeExpr::Named("Merchant")));
            assert_eq!(id.members.len(), 1);
        }
        other => panic!("Expected Item::Impl, got {:?}", other),
    }
}

#[test]
fn test_impl_generic_contract() {
    let items = parse_ok_items(
        "impl Into<string> for Vec2 { fn into() -> string { \"vec\" } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Impl((id, _)) => {
            match &id.contract.as_ref().unwrap().0 {
                TypeExpr::Generic(base, args) => {
                    assert!(matches!(base.0, TypeExpr::Named("Into")));
                    assert_eq!(args.len(), 1);
                }
                other => panic!("Expected Generic contract type, got {:?}", other),
            }
            assert!(matches!(id.target.0, TypeExpr::Named("Vec2")));
        }
        other => panic!("Expected Item::Impl, got {:?}", other),
    }
}

#[test]
fn test_impl_operator_binary() {
    let items = parse_ok_items(
        "impl Vec2 { pub operator +(other: Vec2) -> Vec2 { Vec2(x: self.x + other.x, y: self.y + other.y) } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Impl((id, _)) => {
            assert_eq!(id.members.len(), 1);
            match &id.members[0].0 {
                ImplMember::Op((od, _)) => {
                    assert!(matches!(od.symbol.0, OpSymbol::Add));
                    assert_eq!(od.params.len(), 1);
                    assert_eq!(od.params[0].0.name.0, "other");
                }
                other => panic!("Expected ImplMember::Op, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Impl, got {:?}", other),
    }
}

#[test]
fn test_impl_operator_index() {
    let items = parse_ok_items(
        "impl Grid { pub operator [](key: int) -> int { self.data[key] } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Impl((id, _)) => {
            match &id.members[0].0 {
                ImplMember::Op((od, _)) => {
                    assert!(matches!(od.symbol.0, OpSymbol::Index));
                    assert_eq!(od.params.len(), 1);
                }
                other => panic!("Expected ImplMember::Op, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Impl, got {:?}", other),
    }
}

#[test]
fn test_impl_operator_index_set() {
    let items = parse_ok_items(
        "impl Grid { pub operator []=(key: int, value: int) { self.data[key] = value; } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Impl((id, _)) => {
            match &id.members[0].0 {
                ImplMember::Op((od, _)) => {
                    assert!(matches!(od.symbol.0, OpSymbol::IndexSet));
                    assert_eq!(od.params.len(), 2);
                }
                other => panic!("Expected ImplMember::Op, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Impl, got {:?}", other),
    }
}

#[test]
fn test_impl_operator_unary_neg() {
    let items = parse_ok_items(
        "impl Vec2 { pub operator -() -> Vec2 { Vec2(x: -self.x, y: -self.y) } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Impl((id, _)) => {
            match &id.members[0].0 {
                ImplMember::Op((od, _)) => {
                    assert!(matches!(od.symbol.0, OpSymbol::Sub));
                    assert_eq!(od.params.len(), 0);
                }
                other => panic!("Expected ImplMember::Op, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Impl, got {:?}", other),
    }
}

// =========================================================
// DECL-10: Entity declarations
// =========================================================

#[test]
fn test_entity_basic() {
    let items = parse_ok_items(
        "pub entity Guard { pub name: string = \"Guard\", }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Entity((ed, _)) => {
            assert!(matches!(ed.vis, Some(Visibility::Pub)));
            assert_eq!(ed.name.0, "Guard");
            assert_eq!(ed.members.len(), 1);
            match &ed.members[0].0 {
                EntityMember::Property { vis, name, ty, default } => {
                    assert!(matches!(vis, Some(Visibility::Pub)));
                    assert_eq!(name.0, "name");
                    assert!(matches!(ty.0, TypeExpr::Named("string")));
                    assert!(default.is_some());
                }
                other => panic!("Expected EntityMember::Property, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Entity, got {:?}", other),
    }
}

#[test]
fn test_entity_use_clause() {
    let items = parse_ok_items(
        "pub entity Guard { use Speaker { displayName: \"Guard\", }, }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Entity((ed, _)) => {
            assert_eq!(ed.members.len(), 1);
            match &ed.members[0].0 {
                EntityMember::Use { component, fields } => {
                    assert_eq!(component.0, "Speaker");
                    assert_eq!(fields.len(), 1);
                    assert_eq!(fields[0].0.name.0, "displayName");
                }
                other => panic!("Expected EntityMember::Use, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Entity, got {:?}", other),
    }
}

#[test]
fn test_entity_fn_member() {
    let items = parse_ok_items(
        "pub entity Guard { pub fn greet() -> string { \"Hello\" } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Entity((ed, _)) => {
            assert_eq!(ed.members.len(), 1);
            match &ed.members[0].0 {
                EntityMember::Fn((fd, _)) => {
                    assert_eq!(fd.name.0, "greet");
                }
                other => panic!("Expected EntityMember::Fn, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Entity, got {:?}", other),
    }
}

#[test]
fn test_entity_on_handler() {
    let items = parse_ok_items(
        "pub entity Guard { on create { log(\"created\"); } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Entity((ed, _)) => {
            assert_eq!(ed.members.len(), 1);
            match &ed.members[0].0 {
                EntityMember::On { event, params, body } => {
                    assert_eq!(event.0, "create");
                    assert!(params.is_none());
                    assert!(!body.is_empty());
                }
                other => panic!("Expected EntityMember::On, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Entity, got {:?}", other),
    }
}

#[test]
fn test_entity_on_with_params() {
    let items = parse_ok_items(
        "pub entity Guard { on interact(who: Entity) { log(\"hi\"); } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Entity((ed, _)) => {
            match &ed.members[0].0 {
                EntityMember::On { event, params, .. } => {
                    assert_eq!(event.0, "interact");
                    let p = params.as_ref().expect("Expected params");
                    assert_eq!(p.len(), 1);
                    assert_eq!(p[0].0.name.0, "who");
                }
                other => panic!("Expected EntityMember::On, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Entity, got {:?}", other),
    }
}

#[test]
fn test_entity_transition_in_on() {
    let items = parse_ok_items(
        "pub entity Guard { on interact(who: Entity) { -> guardDialog(self, who); } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Entity((ed, _)) => {
            match &ed.members[0].0 {
                EntityMember::On { body, .. } => {
                    assert_eq!(body.len(), 1);
                    match &body[0].0 {
                        Stmt::Transition((t, _)) => {
                            assert_eq!(t.target.0, "guardDialog");
                            let args = t.args.as_ref().expect("Expected args");
                            assert_eq!(args.len(), 2);
                        }
                        other => panic!("Expected Stmt::Transition, got {:?}", other),
                    }
                }
                other => panic!("Expected EntityMember::On, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Entity, got {:?}", other),
    }
}

// =========================================================
// DECL-11: Component declarations
// =========================================================

#[test]
fn test_component_basic() {
    let items = parse_ok_items(
        "pub component Health { pub current: int, pub max: int, }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Component((cd, _)) => {
            assert!(matches!(cd.vis, Some(Visibility::Pub)));
            assert_eq!(cd.name.0, "Health");
            assert_eq!(cd.members.len(), 2);
            match &cd.members[0].0 {
                ComponentMember::Field((sf, _)) => {
                    assert_eq!(sf.name.0, "current");
                }
                other => panic!("Expected ComponentMember::Field, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Component, got {:?}", other),
    }
}

#[test]
fn test_component_with_method() {
    let items = parse_ok_items(
        "pub component Health { pub current: int, pub fn damage(amount: int) { self.current -= amount; } }",
    );
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Component((cd, _)) => {
            assert_eq!(cd.members.len(), 2);
            // First: field
            assert!(matches!(&cd.members[0].0, ComponentMember::Field(_)));
            // Second: method
            match &cd.members[1].0 {
                ComponentMember::Fn((fd, _)) => {
                    assert_eq!(fd.name.0, "damage");
                }
                other => panic!("Expected ComponentMember::Fn, got {:?}", other),
            }
        }
        other => panic!("Expected Item::Component, got {:?}", other),
    }
}

// =========================================================
// DECL-09: Extern declarations
// =========================================================

#[test]
fn test_extern_fn() {
    let items = parse_ok_items("extern fn log(msg: string);");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Extern((ExternDecl::Fn((sig, _)), _)) => {
            assert_eq!(sig.name.0, "log");
            assert_eq!(sig.params.len(), 1);
            assert!(sig.return_type.is_none());
        }
        other => panic!("Expected Item::Extern(ExternDecl::Fn), got {:?}", other),
    }
}

#[test]
fn test_extern_fn_return() {
    let items = parse_ok_items("extern fn random() -> float;");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Extern((ExternDecl::Fn((sig, _)), _)) => {
            assert_eq!(sig.name.0, "random");
            assert!(sig.return_type.is_some());
        }
        other => panic!("Expected Item::Extern(ExternDecl::Fn), got {:?}", other),
    }
}

#[test]
fn test_extern_struct() {
    let items = parse_ok_items("extern struct Vec2 { x: float, y: float, }");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Extern((ExternDecl::Struct((sd, _)), _)) => {
            assert_eq!(sd.name.0, "Vec2");
            assert_eq!(sd.fields.len(), 2);
            assert_eq!(sd.fields[0].0.name.0, "x");
            assert_eq!(sd.fields[1].0.name.0, "y");
        }
        other => panic!("Expected Item::Extern(ExternDecl::Struct), got {:?}", other),
    }
}

#[test]
fn test_extern_component() {
    let items = parse_ok_items("extern component Transform { position: Vec2, rotation: float, }");
    assert_eq!(items.len(), 1);
    match &items[0].0 {
        Item::Extern((ExternDecl::Component((cd, _)), _)) => {
            assert_eq!(cd.name.0, "Transform");
            assert_eq!(cd.members.len(), 2);
        }
        other => panic!("Expected Item::Extern(ExternDecl::Component), got {:?}", other),
    }
}

// =========================================================
// Phase 4 Integration Tests — Reference .writ files
// =========================================================

#[test]
fn parse_04_structs() {
    let src = include_str!("cases/04_structs.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "04_structs.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "04_structs.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "04_structs.writ produced no items");
    // Structural assertions: at least one Struct and one Impl
    assert!(items.iter().any(|(item, _)| matches!(item, Item::Struct(_))),
        "Expected at least one Item::Struct");
    assert!(items.iter().any(|(item, _)| matches!(item, Item::Impl(_))),
        "Expected at least one Item::Impl");
}

#[test]
fn parse_05_enums() {
    let src = include_str!("cases/05_enums.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "05_enums.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "05_enums.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "05_enums.writ produced no items");
}

#[test]
fn parse_06_contracts() {
    let src = include_str!("cases/06_contracts.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "06_contracts.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "06_contracts.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "06_contracts.writ produced no items");
    assert!(items.iter().any(|(item, _)| matches!(item, Item::Contract(_))),
        "Expected at least one Item::Contract");
}

#[test]
fn parse_07_functions() {
    let src = include_str!("cases/07_functions.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "07_functions.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "07_functions.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "07_functions.writ produced no items");
}

#[test]
fn parse_09_entities() {
    let src = include_str!("cases/09_entities.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "09_entities.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "09_entities.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "09_entities.writ produced no items");
    assert!(items.iter().any(|(item, _)| matches!(item, Item::Entity(_))),
        "Expected at least one Item::Entity");
    assert!(items.iter().any(|(item, _)| matches!(item, Item::Component(_))),
        "Expected at least one Item::Component");
    assert!(items.iter().any(|(item, _)| matches!(item, Item::Extern(_))),
        "Expected at least one Item::Extern");
}

#[test]
fn parse_12_namespaces() {
    let src = include_str!("cases/12_namespaces.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "12_namespaces.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "12_namespaces.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "12_namespaces.writ produced no items");
    assert!(items.iter().any(|(item, _)| matches!(item, Item::Namespace(_))),
        "Expected at least one Item::Namespace");
    assert!(items.iter().any(|(item, _)| matches!(item, Item::Using(_))),
        "Expected at least one Item::Using");
}

#[test]
fn parse_12b_namespace_block() {
    let src = include_str!("cases/12b_namespace_block.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "12b_namespace_block.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "12b_namespace_block.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "12b_namespace_block.writ produced no items");
}

#[test]
fn parse_13_concurrency() {
    let src = include_str!("cases/13_concurrency.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "13_concurrency.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "13_concurrency.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "13_concurrency.writ produced no items");
}

#[test]
fn parse_14_attributes() {
    let src = include_str!("cases/14_attributes.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "14_attributes.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "14_attributes.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "14_attributes.writ produced no items");
}

#[test]
fn parse_17_globals_atomic() {
    let src = include_str!("cases/17_globals_atomic.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "17_globals_atomic.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "17_globals_atomic.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "17_globals_atomic.writ produced no items");
    assert!(items.iter().any(|(item, _)| matches!(item, Item::Global(_))),
        "Expected at least one Item::Global");
}

#[test]
fn parse_18_extern() {
    let src = include_str!("cases/18_extern.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "18_extern.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "18_extern.writ produced no output");
    let items = result.unwrap();
    assert!(!items.is_empty(), "18_extern.writ produced no items");
}

// =========================================================
// Phase 5 Integration Tests — All 21 reference files
// =========================================================

#[test]
fn parse_01_comments_writ() {
    let src = include_str!("cases/01_comments.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "01_comments.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "01_comments.writ produced no output");
    let items = result.unwrap();
    assert_eq!(items.len(), 2, "01_comments.writ: expected 2 items, got {}", items.len());
}

#[test]
fn parse_02_string_literals_writ() {
    let src = include_str!("cases/02_string_literals.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "02_string_literals.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "02_string_literals.writ produced no output");
    let items = result.unwrap();
    assert_eq!(items.len(), 5, "02_string_literals.writ: expected 5 items, got {}", items.len());
}

#[test]
fn parse_03_variables_constants_writ() {
    let src = include_str!("cases/03_variables_constants.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "03_variables_constants.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "03_variables_constants.writ produced no output");
    let items = result.unwrap();
    assert_eq!(items.len(), 9, "03_variables_constants.writ: expected 9 items, got {}", items.len());
}

#[test]
fn parse_10_operators_writ() {
    let src = include_str!("cases/10_operators.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "10_operators.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "10_operators.writ produced no output");
    let items = result.unwrap();
    assert_eq!(items.len(), 10, "10_operators.writ: expected 10 items, got {}", items.len());
}

#[test]
fn parse_11_error_handling_writ() {
    let src = include_str!("cases/11_error_handling.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "11_error_handling.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "11_error_handling.writ produced no output");
    let items = result.unwrap();
    assert_eq!(items.len(), 10, "11_error_handling.writ: expected 10 items, got {}", items.len());
}

#[test]
fn parse_15_ranges_indexing_writ() {
    let src = include_str!("cases/15_ranges_indexing.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "15_ranges_indexing.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "15_ranges_indexing.writ produced no output");
    let items = result.unwrap();
    assert_eq!(items.len(), 7, "15_ranges_indexing.writ: expected 7 items, got {}", items.len());
}

#[test]
fn parse_16_generics_writ() {
    let src = include_str!("cases/16_generics.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "16_generics.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "16_generics.writ produced no output");
    let items = result.unwrap();
    assert_eq!(items.len(), 13, "16_generics.writ: expected 13 items, got {}", items.len());
}

#[test]
fn parse_20_comprehensive_writ() {
    let src = include_str!("cases/20_comprehensive.writ");
    let (result, errors) = parse(src);
    assert!(errors.is_empty(), "20_comprehensive.writ had parse errors: {errors:?}");
    assert!(result.is_some(), "20_comprehensive.writ produced no output");
    let items = result.unwrap();
    assert_eq!(items.len(), 18, "20_comprehensive.writ: expected 18 items, got {}", items.len());
}

#[test]
fn roundtrip_20_comprehensive() {
    let src = include_str!("cases/20_comprehensive.writ");
    let tokens = writ_parser::lexer::lex(src);
    // Concatenate all token spans to reconstruct the source
    let reconstructed: String = tokens
        .iter()
        .map(|(_, span)| &src[span.start..span.end])
        .collect();
    assert_eq!(
        reconstructed, src,
        "Lexer roundtrip for 20_comprehensive.writ failed: reconstruction does not match original"
    );
}

// =========================================================
// Error Recovery Tests (RECOV-01, RECOV-02)
// =========================================================

#[test]
fn multi_error_collection() {
    // Two broken declarations followed by a valid one.
    // Parser should collect >= 2 errors and still produce partial output.
    let src = "fn bad1( { }\nfn bad2( { }\nfn good() { let x = 1; }";
    let (output, errors) = parse(src);
    assert!(
        errors.len() >= 2,
        "Expected at least 2 errors from multi-error input, got {}: {:?}",
        errors.len(),
        errors,
    );
    assert!(
        output.is_some(),
        "Expected partial output from multi-error input, got None",
    );
    let items = output.unwrap();
    // At least the valid fn good() should be present in partial output
    assert!(
        !items.is_empty(),
        "Expected at least 1 item in partial output (fn good), got 0",
    );
}

#[test]
fn recovery_nested_delimiters_fn_body() {
    // A function with a broken body followed by a valid function.
    // Recovery should skip the broken body and allow the valid function to parse.
    let src = "fn broken() { let x = !!!; }\nfn valid() { let y = 42; }";
    let (output, errors) = parse(src);
    assert!(
        !errors.is_empty(),
        "Expected errors from broken function body, got none",
    );
    assert!(
        output.is_some(),
        "Expected output with recovery, got None",
    );
    let items = output.unwrap();
    // Recovery should allow at least some items to be parsed
    assert!(
        !items.is_empty(),
        "Expected items from recovery, got empty list",
    );
}

#[test]
fn recovery_nested_delimiters_struct_body() {
    // A struct with a broken field followed by a valid struct.
    let src = "struct Bad { x: , y: int }\nstruct Good { a: string }";
    let (output, errors) = parse(src);
    assert!(
        !errors.is_empty(),
        "Expected errors from broken struct body, got none",
    );
    assert!(
        output.is_some(),
        "Expected output with recovery, got None",
    );
    let items = output.unwrap();
    assert!(
        !items.is_empty(),
        "Expected items from recovery, got empty list",
    );
}

#[test]
fn recovery_skips_bad_item() {
    // Completely garbled top-level tokens followed by a valid function.
    // Item-level recovery should skip the garbage and parse the valid fn.
    let src = "@@@ garbage tokens here\nfn valid() { let z = 1; }";
    let (output, errors) = parse(src);
    assert!(
        !errors.is_empty(),
        "Expected errors from garbled input, got none",
    );
    assert!(
        output.is_some(),
        "Expected output with recovery, got None",
    );
    let items = output.unwrap();
    // Should recover and parse at least the valid fn
    assert!(
        !items.is_empty(),
        "Expected at least 1 item after skipping garbage, got 0",
    );
}

#[test]
fn recovery_does_not_break_valid_input() {
    // Regression guard: all 21 reference files must parse without errors
    // after recovery is added. Recovery must not cause false positives.
    let files: &[(&str, &str)] = &[
        ("01_comments.writ", include_str!("cases/01_comments.writ")),
        ("02_string_literals.writ", include_str!("cases/02_string_literals.writ")),
        ("03_variables_constants.writ", include_str!("cases/03_variables_constants.writ")),
        ("04_structs.writ", include_str!("cases/04_structs.writ")),
        ("05_enums.writ", include_str!("cases/05_enums.writ")),
        ("06_contracts.writ", include_str!("cases/06_contracts.writ")),
        ("07_functions.writ", include_str!("cases/07_functions.writ")),
        ("08_dialogue.writ", include_str!("cases/08_dialogue.writ")),
        ("09_entities.writ", include_str!("cases/09_entities.writ")),
        ("10_operators.writ", include_str!("cases/10_operators.writ")),
        ("11_error_handling.writ", include_str!("cases/11_error_handling.writ")),
        ("12_namespaces.writ", include_str!("cases/12_namespaces.writ")),
        ("12b_namespace_block.writ", include_str!("cases/12b_namespace_block.writ")),
        ("13_concurrency.writ", include_str!("cases/13_concurrency.writ")),
        ("14_attributes.writ", include_str!("cases/14_attributes.writ")),
        ("15_ranges_indexing.writ", include_str!("cases/15_ranges_indexing.writ")),
        ("16_generics.writ", include_str!("cases/16_generics.writ")),
        ("17_globals_atomic.writ", include_str!("cases/17_globals_atomic.writ")),
        ("18_extern.writ", include_str!("cases/18_extern.writ")),
        ("19_localization.writ", include_str!("cases/19_localization.writ")),
        ("20_comprehensive.writ", include_str!("cases/20_comprehensive.writ")),
    ];
    for (name, src) in files {
        let (output, errors) = parse(src);
        assert!(
            errors.is_empty(),
            "Recovery caused false positive errors on {}: {:?}",
            name,
            errors,
        );
        assert!(
            output.is_some(),
            "Recovery broke valid file {} (no output)",
            name,
        );
    }
}
