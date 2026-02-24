use chumsky::span::SimpleSpan;
use thiserror::Error;

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
