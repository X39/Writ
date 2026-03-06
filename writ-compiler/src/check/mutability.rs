//! Mutability enforcement for the Writ type checker.
//!
//! Central module for mutation checks. Provides:
//! - Root-binding propagation: walks expression trees to find the root binding
//! - Immutable reassignment detection (E0108)
//! - Immutable field mutation detection (E0107)
//! - `mut self` method call checking on immutable bindings (E0107)
//!
//! All mutation checks route through `check_mutation` which delegates
//! to the appropriate checker based on the mutation kind.
