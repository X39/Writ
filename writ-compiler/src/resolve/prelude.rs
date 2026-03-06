//! Writ prelude definitions.
//!
//! The prelude contains primitive types, built-in types, and standard contracts
//! that are always available without importing. User code cannot shadow these names.

/// Primitive type names (value types with direct IL support).
pub const PRELUDE_PRIMITIVE_NAMES: &[&str] = &["int", "float", "bool", "string", "void"];

/// Built-in type names from `writ-runtime`.
pub const PRELUDE_TYPE_NAMES: &[&str] = &["Option", "Result", "Range", "Array", "Entity"];

/// Standard contract names from `writ-runtime`.
pub const PRELUDE_CONTRACT_NAMES: &[&str] = &[
    "Add", "Sub", "Mul", "Div", "Mod", "Neg", "Not", "Eq", "Ord", "Index", "IndexSet", "BitAnd",
    "BitOr", "Iterable", "Iterator", "Into", "Error",
];

/// Sub-prelude variant constructors -- injected below all user-defined names.
/// Any user definition at any scope level shadows these without error.
pub const SUB_PRELUDE_VARIANT_NAMES: &[&str] = &["None", "Some"];

/// Log namespace: the 5 log-level inbuilt function names injected as synthetic ExternFn DefIds.
/// Each is registered in DefMap under the FQN `"log::{level}"` (e.g. `"log::info"`).
pub const LOG_NAMESPACE_LEVELS: &[&str] = &["trace", "debug", "info", "warn", "error"];

/// Dialogue builtin function names injected as synthetic ExternFn DefIds.
/// These are root-level (no namespace prefix) — registered as FQN `"say"`, `"choice"`, etc.
pub const DIALOGUE_BUILTINS: &[&str] = &["say", "say_localized", "choice", "ChoiceOption"];

/// Check if a name is any prelude name (primitive, type, or contract).
pub fn is_prelude_name(name: &str) -> bool {
    PRELUDE_PRIMITIVE_NAMES.contains(&name)
        || PRELUDE_TYPE_NAMES.contains(&name)
        || PRELUDE_CONTRACT_NAMES.contains(&name)
}

/// Check if a name is a primitive type name.
pub fn is_primitive_name(name: &str) -> bool {
    PRELUDE_PRIMITIVE_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_are_prelude() {
        assert!(is_prelude_name("int"));
        assert!(is_prelude_name("string"));
        assert!(is_prelude_name("bool"));
        assert!(is_primitive_name("int"));
    }

    #[test]
    fn types_are_prelude() {
        assert!(is_prelude_name("Option"));
        assert!(is_prelude_name("Result"));
        assert!(is_prelude_name("Entity"));
        assert!(!is_primitive_name("Option"));
    }

    #[test]
    fn contracts_are_prelude() {
        assert!(is_prelude_name("Add"));
        assert!(is_prelude_name("Eq"));
        assert!(is_prelude_name("Iterator"));
        assert!(is_prelude_name("Error"));
    }

    #[test]
    fn non_prelude_names() {
        assert!(!is_prelude_name("Foo"));
        assert!(!is_prelude_name("MyStruct"));
        assert!(!is_prelude_name("custom"));
    }
}
