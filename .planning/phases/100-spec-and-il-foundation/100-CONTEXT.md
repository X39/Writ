# Phase 100: Spec and IL Foundation - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure/spec phase — all design decisions pre-captured in STATE.md)

<domain>
## Phase Boundary

Write the language spec reflection section and update IL spec sections to define TypeOf opcode, reflection types in writ-runtime virtual module, and format_version 4. Satisfies SPEC-01 through SPEC-08. Purely documentation — no code changes.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure spec/documentation phase. All key design decisions already captured in STATE.md Accumulated Context:
- typeof(expr) is static compile-time, expr.get_type() is dynamic runtime
- Reflectable is contract 19 with get_type() -> Type, auto-implemented on all user-defined types
- 6 reflection types: Type, FieldInfo, MethodInfo, ParameterInfo, AttributeInfo, ContractInfo
- FieldInfo.set() crashes task on let-field violation
- MethodInfo.invoke() uses current task stack
- format_version bumps 3 → 4
- BOX/UNBOX coercions at reflection API boundaries (no TyKind::Any)
- Dynamic construction deferred to v12+
- Primitive typeof via intrinsics (IntGetType etc.)
- ReflectionIndex lazy init
- GC permanent roots for reflection singletons

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- Spec files in `language-spec/spec/` numbered `00_` through `69_`
- IL spec sections `30_` through `67_` cover VM, type system, instructions, execution model
- `47_2_18_writ_runtime_module_contents.md` — virtual module spec (where reflection types go)
- `67_4_2_opcode_assignment_table.md` — opcode table (where TypeOf goes)

### Established Patterns
- Language spec sections numbered §1-§28 (language), §2.1-§2.18 (IL), §3.0-§3.16 (instructions), §4.0-§4.2 (reference)
- Each section has a dedicated file with numeric prefix for ordering

### Integration Points
- New §1.X Reflection section (language spec, new file)
- Update §2.18 writ-runtime module contents (add reflection types + Reflectable contract 19)
- Update §4.2 opcode table (add TypeOf)
- Update §2.4 or relevant format section (format_version 4)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase. Follow existing spec style and structure.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
