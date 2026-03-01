//! Type error definitions and conversion to diagnostics.

use chumsky::span::SimpleSpan;
use writ_diagnostics::{code, Diagnostic, FileId};

/// Type errors produced during type checking.
#[derive(Debug, Clone)]
pub enum TypeError {
    TypeMismatch {
        expected: String,
        found: String,
        expected_span: SimpleSpan,
        found_span: SimpleSpan,
        file: FileId,
        help: Option<String>,
    },
    ArityMismatch {
        fn_name: String,
        expected: usize,
        found: usize,
        call_span: SimpleSpan,
        def_span: SimpleSpan,
        file: FileId,
    },
    UndefinedVariable {
        name: String,
        span: SimpleSpan,
        file: FileId,
    },
    UnsatisfiedBound {
        ty_name: String,
        bound_name: String,
        call_span: SimpleSpan,
        file: FileId,
    },
    NotCallable {
        ty_name: String,
        span: SimpleSpan,
        file: FileId,
    },
    CannotInferType {
        name: String,
        span: SimpleSpan,
        file: FileId,
    },
    UnknownField {
        ty_name: String,
        field_name: String,
        span: SimpleSpan,
        file: FileId,
    },
    ImmutableMutation {
        binding_name: String,
        binding_span: SimpleSpan,
        mutation_span: SimpleSpan,
        mutation_kind: String,
        file: FileId,
    },
    ImmutableReassignment {
        binding_name: String,
        binding_span: SimpleSpan,
        assignment_span: SimpleSpan,
        file: FileId,
    },
    MissingReturn {
        fn_name: String,
        expected_ty: String,
        fn_span: SimpleSpan,
        file: FileId,
    },
    OperatorNotImplemented {
        op: String,
        ty_name: String,
        span: SimpleSpan,
        file: FileId,
    },
    MissingContractImpl {
        ty_name: String,
        contract_name: String,
        span: SimpleSpan,
        file: FileId,
        suggestion: String,
    },
    QuestionOnNonOption {
        found_ty: String,
        span: SimpleSpan,
        file: FileId,
    },
    QuestionWrongContext {
        expected: String,
        actual: String,
        span: SimpleSpan,
        file: FileId,
    },
    TryOnNonResult {
        found_ty: String,
        span: SimpleSpan,
        file: FileId,
    },
    NonExhaustiveMatch {
        missing_variants: Vec<String>,
        match_span: SimpleSpan,
        file: FileId,
    },
    MissingConstructionField {
        type_name: String,
        field_name: String,
        span: SimpleSpan,
        file: FileId,
    },
    NotIterable {
        ty_name: String,
        span: SimpleSpan,
        file: FileId,
    },
}

impl From<TypeError> for Diagnostic {
    fn from(err: TypeError) -> Diagnostic {
        match err {
            TypeError::TypeMismatch {
                expected,
                found,
                expected_span,
                found_span,
                file,
                help,
            } => {
                let mut builder = Diagnostic::error(
                    code::E0100,
                    format!("type mismatch: expected `{}`, found `{}`", expected, found),
                )
                .with_primary(file, found_span, format!("found `{}` here", found))
                .with_secondary(file, expected_span, format!("expected `{}`", expected));

                if let Some(h) = help {
                    builder = builder.with_help(h);
                }
                builder.build()
            }
            TypeError::ArityMismatch {
                fn_name,
                expected,
                found,
                call_span,
                def_span,
                file,
            } => Diagnostic::error(
                code::E0101,
                format!(
                    "function `{}` expects {} argument(s), but {} were provided",
                    fn_name, expected, found
                ),
            )
            .with_primary(file, call_span, format!("{} argument(s) provided", found))
            .with_secondary(
                file,
                def_span,
                format!("`{}` defined with {} parameter(s)", fn_name, expected),
            )
            .build(),
            TypeError::UndefinedVariable { name, span, file } => Diagnostic::error(
                code::E0102,
                format!("undefined variable `{}`", name),
            )
            .with_primary(file, span, "not found in this scope")
            .build(),
            TypeError::UnsatisfiedBound {
                ty_name,
                bound_name,
                call_span,
                file,
            } => Diagnostic::error(
                code::E0103,
                format!(
                    "the contract bound `{}` is not satisfied for type `{}`",
                    bound_name, ty_name
                ),
            )
            .with_primary(file, call_span, "unsatisfied bound here")
            .with_help(format!(
                "consider adding `impl {} for {} {{ ... }}`",
                bound_name, ty_name
            ))
            .build(),
            TypeError::NotCallable { ty_name, span, file } => Diagnostic::error(
                code::E0104,
                format!("type `{}` is not callable", ty_name),
            )
            .with_primary(file, span, "not a function")
            .build(),
            TypeError::CannotInferType { name, span, file } => Diagnostic::error(
                code::E0105,
                format!("cannot infer type for `{}`", name),
            )
            .with_primary(file, span, "type annotation needed")
            .build(),
            TypeError::UnknownField {
                ty_name,
                field_name,
                span,
                file,
            } => Diagnostic::error(
                code::E0106,
                format!("type `{}` has no field `{}`", ty_name, field_name),
            )
            .with_primary(file, span, "unknown field")
            .build(),
            TypeError::ImmutableMutation {
                binding_name,
                binding_span,
                mutation_span,
                mutation_kind,
                file,
            } => Diagnostic::error(
                code::E0107,
                format!(
                    "cannot {} on immutable binding `{}`",
                    mutation_kind, binding_name
                ),
            )
            .with_primary(file, mutation_span, format!("{} here", mutation_kind))
            .with_secondary(
                file,
                binding_span,
                format!("`{}` declared as immutable", binding_name),
            )
            .with_help(format!("consider changing to `let mut {}`", binding_name))
            .build(),
            TypeError::ImmutableReassignment {
                binding_name,
                binding_span,
                assignment_span,
                file,
            } => Diagnostic::error(
                code::E0108,
                format!("cannot assign to immutable binding `{}`", binding_name),
            )
            .with_primary(file, assignment_span, "assignment here")
            .with_secondary(
                file,
                binding_span,
                format!("`{}` declared as immutable", binding_name),
            )
            .with_help(format!("consider changing to `let mut {}`", binding_name))
            .build(),
            TypeError::MissingReturn {
                fn_name,
                expected_ty,
                fn_span,
                file,
            } => Diagnostic::error(
                code::E0109,
                format!(
                    "function `{}` must return `{}` on all code paths",
                    fn_name, expected_ty
                ),
            )
            .with_primary(file, fn_span, "missing return value")
            .build(),
            TypeError::OperatorNotImplemented {
                op,
                ty_name,
                span,
                file,
            } => Diagnostic::error(
                code::E0111,
                format!("cannot apply operator `{}` to type `{}`", op, ty_name),
            )
            .with_primary(file, span, "operator not supported")
            .build(),
            TypeError::MissingContractImpl {
                ty_name,
                contract_name,
                span,
                file,
                suggestion,
            } => Diagnostic::error(
                code::E0112,
                format!(
                    "type `{}` does not implement contract `{}`",
                    ty_name, contract_name
                ),
            )
            .with_primary(file, span, "missing implementation")
            .with_help(suggestion)
            .build(),
            TypeError::QuestionOnNonOption {
                found_ty,
                span,
                file,
            } => Diagnostic::error(
                code::E0113,
                format!(
                    "`?` operator requires `Option<T>`, found `{}`",
                    found_ty
                ),
            )
            .with_primary(file, span, "not an Option type")
            .build(),
            TypeError::QuestionWrongContext {
                expected,
                actual,
                span,
                file,
            } => Diagnostic::error(
                code::E0114,
                format!(
                    "`?` operator requires enclosing function to return `{}`, but it returns `{}`",
                    expected, actual
                ),
            )
            .with_primary(file, span, "? used here")
            .build(),
            TypeError::TryOnNonResult {
                found_ty,
                span,
                file,
            } => Diagnostic::error(
                code::E0115,
                format!(
                    "`try` requires `Result<T, E>`, found `{}`",
                    found_ty
                ),
            )
            .with_primary(file, span, "not a Result type")
            .build(),
            TypeError::NonExhaustiveMatch {
                missing_variants,
                match_span,
                file,
            } => Diagnostic::error(
                code::E0116,
                format!(
                    "non-exhaustive match: missing variant(s) {}",
                    missing_variants
                        .iter()
                        .map(|v| format!("`{}`", v))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
            .with_primary(file, match_span, "non-exhaustive match")
            .build(),
            TypeError::MissingConstructionField {
                type_name,
                field_name,
                span,
                file,
            } => Diagnostic::error(
                code::E0117,
                format!(
                    "missing field `{}` in construction of `{}`",
                    field_name, type_name
                ),
            )
            .with_primary(file, span, format!("missing `{}`", field_name))
            .build(),
            TypeError::NotIterable {
                ty_name,
                span,
                file,
            } => Diagnostic::error(
                code::E0118,
                format!("type `{}` is not iterable", ty_name),
            )
            .with_primary(file, span, "not iterable")
            .build(),
        }
    }
}
