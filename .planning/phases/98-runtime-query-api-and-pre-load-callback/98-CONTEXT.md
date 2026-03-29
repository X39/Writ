# Phase 98: Runtime Query API and Pre-Load Callback - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — discuss skipped)

<domain>
## Phase Boundary

The host can inspect all attribute data on a loaded module before any code executes, query attributes by name or by type, and reject modules that do not meet attribute-based requirements.

Requirements: QAPI-01, QAPI-02, QAPI-03, QAPI-04, QAPI-05, QAPI-06

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

Key design notes from research:
- ModuleAttributeView (not &Domain) must be the pre-load callback argument from day one — retrofitting is a breaking change
- No attribute causes automatic instantiation or invocation — hosts must explicitly act on query results

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-module/src/attr.rs` — AttrValue enum, encode/decode (Phase 93)
- `writ-module/src/tables.rs` — AttributeDefRow table structure
- `writ-runtime/src/host.rs` — RuntimeHost trait
- `writ-runtime/src/runtime.rs` — Domain struct, add_module method
- `writ-runtime/src/lib.rs` — public API

### Integration Points
- RuntimeHost trait: new on_module_load callback
- Domain: query methods (query_attributes, query_attributes_on, query_attribute_value)
- Module loading path: fire callback before add_module

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None — infrastructure phase.

</deferred>
