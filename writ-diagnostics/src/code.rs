//! Diagnostic error and warning code constants.
//!
//! Error codes (E-series) indicate hard errors that prevent compilation.
//! Warning codes (W-series) indicate potential issues that do not block compilation.

// Error codes
pub const E0001: &str = "E0001"; // duplicate definition
pub const E0002: &str = "E0002"; // prelude shadow
pub const E0003: &str = "E0003"; // unresolved name
pub const E0004: &str = "E0004"; // ambiguous name
pub const E0005: &str = "E0005"; // visibility violation
pub const E0006: &str = "E0006"; // invalid attribute target
pub const E0007: &str = "E0007"; // invalid speaker

// Type error codes (E01xx series)
pub const E0100: &str = "E0100"; // type mismatch
pub const E0101: &str = "E0101"; // arity mismatch
pub const E0102: &str = "E0102"; // undefined variable
pub const E0103: &str = "E0103"; // unsatisfied contract bound
pub const E0104: &str = "E0104"; // not callable
pub const E0105: &str = "E0105"; // cannot infer type
pub const E0106: &str = "E0106"; // unknown field
pub const E0107: &str = "E0107"; // immutable binding mutation
pub const E0108: &str = "E0108"; // immutable binding reassignment
pub const E0109: &str = "E0109"; // missing return
pub const E0110: &str = "E0110"; // not a method receiver
pub const E0111: &str = "E0111"; // operator not implemented
pub const E0112: &str = "E0112"; // missing contract impl (for suggestions)
pub const E0113: &str = "E0113"; // ? on non-Option
pub const E0114: &str = "E0114"; // ? in wrong return context
pub const E0115: &str = "E0115"; // try on non-Result / wrong return context
pub const E0116: &str = "E0116"; // non-exhaustive match
pub const E0117: &str = "E0117"; // missing field in construction
pub const E0118: &str = "E0118"; // not iterable
pub const E0119: &str = "E0119"; // closure capture error

// Warning codes
pub const W0001: &str = "W0001"; // unused import
pub const W0002: &str = "W0002"; // import shadow
pub const W0003: &str = "W0003"; // generic shadow
pub const W0004: &str = "W0004"; // namespace path mismatch
