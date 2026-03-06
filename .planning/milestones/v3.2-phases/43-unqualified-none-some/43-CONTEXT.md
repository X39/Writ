# Phase 43: Unqualified None/Some - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Inject `None` and `Some` as pre-injected sub-prelude symbols so Writ scripts can use them without the `Option::` prefix. Implement a general `using EnumName::*;` glob variant import mechanism alongside — `None`/`Some` are the first users of this mechanism but it applies to all enums. Qualified `Option::None`/`Option::Some` continues to work. No changes to IL emission (emitter already handles `"None"`/`"Some"` by name).

</domain>

<decisions>
## Implementation Decisions

### Pattern position
- Unqualified `None`/`Some` work in both **expression and pattern position** — `match x { None => ..., Some(v) => ... }` is valid without qualification
- Pattern handling in `check_expr.rs` already supports single-segment paths like `["None"]` — this is a natural extension of the existing `["Option", "Some"]` path pattern handling

### General enum variant import mechanism
- Phase 43 implements a general `using EnumName::*;` glob import — not an Option-specific hack
- `using Status::*;` brings all Status variants into scope unqualified
- `using Option::*;` is also valid (redundant since None/Some are pre-injected, but consistent)
- Selective import (`using Option::None;`) is NOT required for this phase — glob only
- Spec gets a new subsection in the existing imports/using section covering: qualified paths, using-glob, and sub-prelude builtins

### None/Some automatic injection
- `None` and `Some` are **pre-injected at sub-prelude priority** — no `using` required
- This makes them behave like `null` (which already lowers to `Option::None`) — a parallel spelling
- Sub-prelude priority means they are shadowed by any user definition without error

### Type inference for bare None
- `let x = None;` with no type annotation → **error**: "cannot infer type for `None` — add a type annotation: `let x: T? = None`"
- `let x = Some(42);` → infers `int?` from the argument type — annotation not required
- `foo(None)` where `foo` takes `bool?` → infers `bool?` from the parameter type (bidirectional inference from context)

### Shadowing rules
- Any user definition (local `let`, type, function param, module-level const) **silently shadows** the injected `None`/`Some` — no warning
- Any user definition also silently shadows using-glob imports (`using Status::*;` Active is shadowed by `let Active = 5;`)
- **Two using-glob conflicts** (e.g., `using Status::*; using Color::*;` both have `Green`) → **ambiguity error**: use qualified path to disambiguate
- This reuses the existing `LookupResult::Ambiguous` mechanism in the resolver

### Claude's Discretion
- Implementation approach for sub-prelude injection: new `LookupResult` variant (e.g., `OptionConstructor`) vs. rewriting to qualified path before resolution vs. handling in type checker — whichever fits the existing resolver architecture cleanest
- Whether `using Status::*;` is handled in the resolver scope chain (new `ScopeLayer::GlobEnum`) or expanded at the `using` declaration site
- Exact spec section number for the new imports subsection

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-compiler/src/resolve/scope.rs`: `LookupResult` enum and `ScopeChain` — add new variant or logic here for unqualified None/Some and using-glob
- `writ-compiler/src/emit/body/expr.rs:1061-1094`: Already matches `"None"`/`"Some"` by name to emit `LOAD_NULL`/`WRAP_SOME` — **no emitter changes needed**
- `writ-compiler/src/lower/expr.rs:65`: `null` → `Path { segments: ["Option", "None"] }` — unqualified `None` needs parallel path through the resolver/checker
- `writ-compiler/src/resolve/prelude.rs`: `PRELUDE_TYPE_NAMES` pattern — sub-prelude injection follows the same shape

### Established Patterns
- `LookupResult::Ambiguous(Vec<(DefId, String)>)` — already used for `using` conflicts; reuse for using-glob collisions
- `check_expr.rs` pattern handling: `path = ["Option", "Some"]` or `["Some"]` already handled in `AstPattern::EnumDestructure` — single-segment `["None"]` and `["Some"]` are natural extensions
- Emitter name-matching already handles `"None"`, `"Some"`, `"Ok"`, `"Err"` uniformly — the pattern is established

### Integration Points
- `resolve/scope.rs` → `check/check_expr.rs`: Resolver must expose unqualified `None`/`Some` with enough info for type checker to assign `Option<_>` type
- `resolver.rs` `using` handling → new `using Enum::*;` case: when a `using` declaration ends with `::*`, resolve the enum name and add all its variants to the active using-imports
- Spec: new subsection in the existing `using` imports section (`language-spec/spec/` splatted files around the `using` declaration coverage)

</code_context>

<specifics>
## Specific Ideas

- The user explicitly confirmed: `None`/`Some` should NOT be treated as special by the language beyond the automatic sub-prelude injection — the general `using Enum::*;` mechanism treats them identically to user-defined enums
- `using Option::*;` should be valid and consistent (even though redundant), so users don't hit a confusing error when they write it explicitly
- Spec update should clarify this applies in both expression and match pattern position
- "Sub-prelude" framing from REQUIREMENTS.md: injected below prelude priority, meaning user-defined names always win at any scope level

</specifics>

<deferred>
## Deferred Ideas

- Selective import: `using Option::None;` (import one variant) — not required, defer to future
- `using Enum::*;` scoped to a block rather than a file — deferred
- Warning on shadowing built-in None/Some — explicitly rejected; no warning per user decision

</deferred>

---

*Phase: 43-unqualified-none-some*
*Context gathered: 2026-03-06*
