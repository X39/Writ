//! Source line to IL address mapping for the DAP server.
//!
//! `BreakpointTable` is built from a compiled `Module`'s `SourceSpan` data.
//! It maps source line numbers to (method_idx, pc) pairs and provides
//! snap-to-nearest logic for breakpoints set on lines with no instructions.

use std::collections::HashMap;
use writ_module::module::Module;

/// A verified breakpoint with its resolved IL address.
#[derive(Debug, Clone)]
pub struct ResolvedBreakpoint {
    pub id: u32,
    /// The actual source line (may differ from requested if snapped to nearest).
    pub line: u32,
    pub method_idx: usize,
    pub pc: u32,
}

/// Maps source file lines to IL instruction addresses using SourceSpan data.
pub struct BreakpointTable {
    /// All valid breakpoint lines from the module's SourceSpan data.
    /// Maps line -> Vec<(method_idx, pc)>. A line may appear in multiple methods.
    /// Only the first pc per (method, line) is stored (earliest instruction on that line).
    line_index: HashMap<u32, Vec<(usize, u32)>>,
    /// Active breakpoints: breakpoint_id -> ResolvedBreakpoint.
    active: HashMap<u32, ResolvedBreakpoint>,
    /// Reverse lookup: (method_idx, pc) -> breakpoint_id.
    pc_lookup: HashMap<(usize, u32), u32>,
    next_id: u32,
}

impl BreakpointTable {
    /// Build a `BreakpointTable` from a compiled module's SourceSpan data.
    ///
    /// Iterates all method bodies' source_spans and maps line -> (method_idx, first_pc).
    /// If a line appears multiple times in the same method, only the smallest pc is kept.
    pub fn new(module: &Module) -> Self {
        let mut line_index: HashMap<u32, Vec<(usize, u32)>> = HashMap::new();

        for (method_idx, body) in module.method_bodies.iter().enumerate() {
            // Track the earliest pc we've seen for each (method_idx, line) pair.
            let mut seen: HashMap<u32, u32> = HashMap::new();
            for span in &body.source_spans {
                let entry = seen.entry(span.line).or_insert(span.pc);
                // Keep the minimum pc for this line in this method.
                if span.pc < *entry {
                    *entry = span.pc;
                }
            }
            for (line, first_pc) in seen {
                line_index
                    .entry(line)
                    .or_default()
                    .push((method_idx, first_pc));
            }
        }

        BreakpointTable {
            line_index,
            active: HashMap::new(),
            pc_lookup: HashMap::new(),
            next_id: 1,
        }
    }

    /// Set breakpoints for a list of requested source lines.
    ///
    /// For each requested line:
    /// - If the line has instructions, create a breakpoint there.
    /// - Otherwise, snap to the nearest valid line >= requested line.
    ///   If no valid line exists >= requested, snap to the nearest line < requested.
    ///   If the module has no source spans at all, the breakpoint is skipped.
    ///
    /// Replaces any previously set breakpoints. Returns the resolved breakpoints
    /// (needed by DAP to send the SetBreakpointsResponse).
    pub fn set_breakpoints(&mut self, lines: &[u32]) -> Vec<ResolvedBreakpoint> {
        // Clear all existing breakpoints.
        self.active.clear();
        self.pc_lookup.clear();

        let mut resolved = Vec::new();

        for &requested_line in lines {
            let actual_line = match self.snap_to_nearest(requested_line) {
                Some(l) => l,
                None => continue, // no valid lines in module at all
            };

            let targets = match self.line_index.get(&actual_line) {
                Some(v) => v.clone(),
                None => continue,
            };

            // Use the first target (lowest method_idx, then first pc).
            let (method_idx, pc) = targets[0];

            let id = self.next_id;
            self.next_id += 1;

            let bp = ResolvedBreakpoint {
                id,
                line: actual_line,
                method_idx,
                pc,
            };
            self.pc_lookup.insert((method_idx, pc), id);
            self.active.insert(id, bp.clone());
            resolved.push(bp);
        }

        resolved
    }

    /// Check if a breakpoint is set at the given (method_idx, pc).
    ///
    /// Returns the breakpoint id if hit, `None` otherwise.
    pub fn lookup(&self, method_idx: usize, pc: u32) -> Option<u32> {
        self.pc_lookup.get(&(method_idx, pc)).copied()
    }

    /// Remove all active breakpoints.
    pub fn clear_all(&mut self) {
        self.active.clear();
        self.pc_lookup.clear();
    }

    /// All valid source lines in this module (sorted).
    pub fn valid_lines(&self) -> Vec<u32> {
        let mut lines: Vec<u32> = self.line_index.keys().copied().collect();
        lines.sort_unstable();
        lines
    }

    /// Snap a requested line to the nearest valid line.
    ///
    /// Priority: exact match > nearest line >= requested > nearest line < requested.
    fn snap_to_nearest(&self, requested: u32) -> Option<u32> {
        if self.line_index.contains_key(&requested) {
            return Some(requested);
        }

        let mut best_above: Option<u32> = None;
        let mut best_below: Option<u32> = None;

        for &line in self.line_index.keys() {
            if line >= requested {
                best_above = Some(match best_above {
                    Some(prev) => prev.min(line),
                    None => line,
                });
            } else {
                best_below = Some(match best_below {
                    Some(prev) => prev.max(line),
                    None => line,
                });
            }
        }

        best_above.or(best_below)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use writ_module::module::{MethodBody, Module, SourceSpan};

    /// Build a minimal Module with the given (method_idx, line, pc) span entries.
    fn make_module(spans: &[(usize, u32, u32)]) -> Module {
        // Determine the number of method bodies needed.
        let max_method = spans.iter().map(|(m, _, _)| *m).max().map(|m| m + 1).unwrap_or(0);
        let mut method_bodies: Vec<MethodBody> = (0..max_method)
            .map(|_| MethodBody {
                register_types: vec![],
                code: vec![],
                debug_locals: vec![],
                source_spans: vec![],
            })
            .collect();

        for &(method_idx, line, pc) in spans {
            method_bodies[method_idx].source_spans.push(SourceSpan {
                pc,
                line,
                column: 0,
            });
        }

        let mut module = Module::new();
        module.method_bodies = method_bodies;
        module
    }

    #[test]
    fn test_breakpoint_hit() {
        // Method 0, pc=5, line=10
        let module = make_module(&[(0, 10, 5)]);
        let mut table = BreakpointTable::new(&module);
        table.set_breakpoints(&[10]);

        // Should hit at (method_idx=0, pc=5)
        let result = table.lookup(0, 5);
        assert!(result.is_some(), "expected breakpoint hit");
    }

    #[test]
    fn test_no_breakpoint_miss() {
        let module = make_module(&[(0, 10, 5)]);
        let mut table = BreakpointTable::new(&module);
        table.set_breakpoints(&[10]);

        // Different pc — should not hit
        let result = table.lookup(0, 6);
        assert!(result.is_none(), "expected no hit at wrong pc");

        // Different method — should not hit
        let result = table.lookup(1, 5);
        assert!(result.is_none(), "expected no hit at wrong method");
    }

    #[test]
    fn test_breakpoint_snap_to_nearest() {
        // Valid lines: 5, 10, 20
        let module = make_module(&[(0, 5, 0), (0, 10, 2), (0, 20, 4)]);
        let mut table = BreakpointTable::new(&module);

        // Request line 7 (between 5 and 10) — should snap to 10
        let resolved = table.set_breakpoints(&[7]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].line, 10, "should snap to nearest line >= 7");

        // Request line 25 (above all valid lines) — should snap to 20
        let resolved = table.set_breakpoints(&[25]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].line, 20, "should snap to nearest line below 25");
    }

    #[test]
    fn test_breakpoint_exact_line() {
        let module = make_module(&[(0, 10, 5)]);
        let mut table = BreakpointTable::new(&module);
        let resolved = table.set_breakpoints(&[10]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].line, 10, "exact line should be used");
        assert_eq!(resolved[0].pc, 5, "pc should match the span");
    }

    #[test]
    fn test_clear_all() {
        let module = make_module(&[(0, 10, 5)]);
        let mut table = BreakpointTable::new(&module);
        table.set_breakpoints(&[10]);
        table.clear_all();

        let result = table.lookup(0, 5);
        assert!(result.is_none(), "breakpoint should be cleared");
    }

    #[test]
    fn test_multiple_methods_same_line() {
        // Same line 10 in both method 0 and method 1
        let module = make_module(&[(0, 10, 5), (1, 10, 3)]);
        let mut table = BreakpointTable::new(&module);
        let resolved = table.set_breakpoints(&[10]);
        assert_eq!(resolved.len(), 1, "one breakpoint per requested line");
        // The first target (by iteration order — non-deterministic by HashMap) is used.
        // Just check that one of the two possible locations is hit.
        let bp = &resolved[0];
        assert_eq!(bp.line, 10);
        let hit = table.lookup(bp.method_idx, bp.pc);
        assert!(hit.is_some(), "lookup should hit the resolved (method, pc)");
    }

    #[test]
    fn test_first_pc_per_line_used() {
        // Two spans at line 10: pc=8 and pc=2 — pc=2 should be preferred.
        let module = make_module(&[(0, 10, 8), (0, 10, 2)]);
        let mut table = BreakpointTable::new(&module);
        let resolved = table.set_breakpoints(&[10]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].pc, 2, "smallest pc for the line should be used");
    }

    #[test]
    fn test_empty_module_no_crash() {
        let module = make_module(&[]);
        let mut table = BreakpointTable::new(&module);
        let resolved = table.set_breakpoints(&[10]);
        assert!(resolved.is_empty(), "no breakpoints should be resolved for empty module");
    }

    #[test]
    fn test_set_breakpoints_clears_previous() {
        let module = make_module(&[(0, 5, 0), (0, 10, 2)]);
        let mut table = BreakpointTable::new(&module);

        table.set_breakpoints(&[5]);
        assert!(table.lookup(0, 0).is_some(), "line 5 should be set");

        // Reset to only line 10
        table.set_breakpoints(&[10]);
        assert!(table.lookup(0, 0).is_none(), "line 5 should be cleared");
        assert!(table.lookup(0, 2).is_some(), "line 10 should be set");
    }
}
