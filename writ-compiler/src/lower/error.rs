use chumsky::span::SimpleSpan;
use thiserror::Error;
use writ_diagnostics::{Diagnostic, FileId};

#[derive(Debug, Error, Clone, PartialEq)]
pub enum LoweringError {
    #[error("unknown speaker `{name}`")]
    UnknownSpeaker {
        name: String,
        span: SimpleSpan,
    },

    #[error("dialogue transition `->` must be the last statement in its block")]
    NonTerminalTransition {
        span: SimpleSpan,
    },

    #[error("duplicate localization key `{key}`")]
    DuplicateLocKey {
        key: String,
        first_span: SimpleSpan,
        second_span: SimpleSpan,
    },

    #[error("conflicting component method `{method}` (from `{first_component}` and `{second_component}`)")]
    ConflictingComponentMethod {
        method: String,
        first_component: String,
        second_component: String,
        span: SimpleSpan,
    },

    #[error("duplicate `use {component}` in entity `{entity}`")]
    DuplicateUseClause {
        component: String,
        entity: String,
        span: SimpleSpan,
    },

    #[error("duplicate property `{property}` in entity `{entity}`")]
    DuplicateProperty {
        property: String,
        entity: String,
        span: SimpleSpan,
    },

    #[error("unknown lifecycle event `{event}` — valid events: create, interact, destroy")]
    UnknownLifecycleEvent {
        event: String,
        span: SimpleSpan,
    },

    #[error("property `{name}` conflicts with component `use {name}` in entity `{entity}`")]
    PropertyComponentCollision {
        name: String,
        entity: String,
        span: SimpleSpan,
    },

    #[error("{message}")]
    Generic {
        message: String,
        span: SimpleSpan,
    },
}

impl LoweringError {
    /// Convert this lowering error into a `Diagnostic` for ariadne rendering.
    pub fn to_diagnostic(&self, file_id: FileId) -> Diagnostic {
        match self {
            LoweringError::UnknownSpeaker { name, span } => Diagnostic::error(
                "L0001",
                format!("unknown speaker `{name}`"),
            )
            .with_primary(file_id, *span, format!("speaker `{name}` is not defined"))
            .with_help("speakers must be declared as [Singleton] entities with a Speaker component")
            .build(),

            LoweringError::NonTerminalTransition { span } => Diagnostic::error(
                "L0002",
                "dialogue transition must be last statement",
            )
            .with_primary(file_id, *span, "this `->` transition is not the final statement in its block")
            .build(),

            LoweringError::DuplicateLocKey { key, first_span, second_span } => Diagnostic::error(
                "L0003",
                format!("duplicate localization key `{key}`"),
            )
            .with_primary(file_id, *second_span, "duplicate key here")
            .with_secondary(file_id, *first_span, "first defined here")
            .build(),

            LoweringError::ConflictingComponentMethod { method, first_component, second_component, span } => Diagnostic::error(
                "L0004",
                format!("conflicting component method `{method}`"),
            )
            .with_primary(file_id, *span, format!("method `{method}` exists in both `{first_component}` and `{second_component}`"))
            .build(),

            LoweringError::DuplicateUseClause { component, entity, span } => Diagnostic::error(
                "L0005",
                format!("duplicate `use {component}` in entity `{entity}`"),
            )
            .with_primary(file_id, *span, format!("`{component}` is already used"))
            .build(),

            LoweringError::DuplicateProperty { property, entity, span } => Diagnostic::error(
                "L0006",
                format!("duplicate property `{property}` in entity `{entity}`"),
            )
            .with_primary(file_id, *span, format!("`{property}` is already defined"))
            .build(),

            LoweringError::UnknownLifecycleEvent { event, span } => Diagnostic::error(
                "L0007",
                format!("unknown lifecycle event `{event}`"),
            )
            .with_primary(file_id, *span, format!("`{event}` is not a valid lifecycle event"))
            .with_help("valid events: create, interact, destroy")
            .build(),

            LoweringError::PropertyComponentCollision { name, entity, span } => Diagnostic::error(
                "L0008",
                format!("property `{name}` conflicts with component `use {name}` in entity `{entity}`"),
            )
            .with_primary(file_id, *span, format!("`{name}` is both a property and a component"))
            .build(),

            LoweringError::Generic { message, span } => Diagnostic::error(
                "L0099",
                message.clone(),
            )
            .with_primary(file_id, *span, message.clone())
            .build(),
        }
    }
}
