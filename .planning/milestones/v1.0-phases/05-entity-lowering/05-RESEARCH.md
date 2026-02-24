# Phase 5: Entity Lowering - Research

**Researched:** 2026-02-26
**Domain:** CST-to-AST desugaring — `entity` declarations to `struct` + `impl` blocks + lifecycle hook contract impls
**Confidence:** HIGH — all findings sourced from the live codebase (CST types, AST types, prior passes, tests)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Lifecycle hook representation**
- Hooks lower to **contract impls**: `on create { ... }` → `impl OnCreate for Guard { fn on_create(self) { ... } }`
- Three contracts: `OnCreate`, `OnInteract`, `OnDestroy` — assumed pre-defined (extern/builtin), lowering only emits impl blocks
- Hook bodies receive **full expression lowering** (formattable strings, optional sugar, compound assignments, `->` transitions all desugar)
- `[Singleton]` propagates as an `AstAttribute` on the generated `AstStructDecl`

**Component initialization**
- `use Health { current: 80, max: 80 }` → struct field with struct literal initializer containing **only the specified overrides**
- `use Speaker {}` → empty struct literal `Speaker {}` — type checker fills all defaults from component definition
- Component fields use **user-unreachable names** (e.g., `$Health`, `$Sprite`) to prevent collisions with user-defined properties
- Entity property fields (name, patrolRoute) retain their original names and visibility unchanged

**ComponentAccess impl shape**
- One `impl ComponentAccess<T> for EntityName` per `use` clause
- Single method: `fn get(self) -> T { self.$ComponentName }`
- Entity's own methods go in a **separate inherent impl block** (`impl Guard { fn greet() { ... } }`)

**Emission order**
- Claude's discretion — pick a logical, deterministic order suitable for snapshot tests

**Error boundaries**
- **Lowering-time errors** (all accumulated, never halt processing):
  - Duplicate `use` clauses (same component used twice)
  - Duplicate property names
  - Unknown lifecycle event names (not create/interact/destroy)
  - Property-component name collisions
- **Deferred to type checker:**
  - Conflicting method names across components (§14.3) — lowering can't see component definitions
  - Missing/invalid field names in use clause overrides
  - Component existence validation

**Member partitioning**
- Explicit named `partition_entity_members()` function as a pre-step
- Returns (properties, use_clauses, methods, hooks)
- All duplicate/validation checks happen during partitioning

### Claude's Discretion

- Emission order — pick a logical, deterministic order suitable for snapshot tests

### Deferred Ideas (OUT OF SCOPE)

- Spec update needed: clarify `self` semantics — remove static modifier for non-instance methods, add `self` parameter to indicate instance methods (spec gap, not a Phase 5 task)
- Cross-component method conflict detection — belongs in the type checker phase, not lowering
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| R12 | Desugar `entity` declarations: `entity Name { ... }` → `struct Name`, entity methods → `impl Name`, lifecycle hooks → contract impl blocks, `[Singleton]` attribute recognized and propagated, member partitioning as explicit pre-step | Confirmed by CST `EntityDecl` / `EntityMember` types; AST `AstStructDecl` / `AstImplDecl` already defined; hook lowering pattern mirrors operator lowering (one `lower_operator_impls` → `Vec<AstDecl>`). `LoweringError::ConflictingComponentMethod` already declared in error.rs |
| R13 | Component field flattening: `use Health { current: 80, max: 80 }` → `$Health: Health` struct field with initializer `Health { current: 80, max: 80 }`; fields not overridden left absent from initializer (type checker fills defaults) | Confirmed by `EntityMember::Use { component, fields }` and `UseField { name, value }` CST types; `AstStructField.default: Option<AstExpr>` carries the struct literal initializer |
</phase_requirements>

---

## Summary

Phase 5 desugar `entity` declarations from the CST into multiple AST declarations. A single `entity Guard { ... }` expands into: one `AstDecl::Struct` (properties + component fields), one `AstDecl::Impl` for entity methods (if any), one `AstDecl::Impl` per `use` clause (`ComponentAccess<T>`), and one `AstDecl::Impl` per lifecycle hook (`OnCreate`, `OnInteract`, `OnDestroy`). The pattern exactly mirrors how `lower_operator_impls` already returns `Vec<AstDecl>` — entity lowering will use the same `decls.extend(...)` call site in `lower/mod.rs`.

All inputs are present in the live codebase: `EntityDecl` and `EntityMember` (including `Property`, `Use`, `Fn`, `On`) are defined in `writ-parser/src/cst.rs`. Every output type needed (`AstStructDecl`, `AstStructField`, `AstImplDecl`, `AstFnDecl`, `AstAttribute`) is defined in `writ-compiler/src/ast/decl.rs`. Lowering-time errors specific to entity lowering are partially pre-declared (`LoweringError::ConflictingComponentMethod` already exists); additional variants for duplicate use, duplicate property, unknown event, and property-component collision need to be added.

The implementation follows one new file pattern established in Phases 3 and 4: create `writ-compiler/src/lower/entity.rs`, expose `pub fn lower_entity(...)  -> Vec<AstDecl>`, wire it in `lower/mod.rs` and in the `lower_namespace` block arm, then write snapshot tests covering all R12/R13 success criteria.

**Primary recommendation:** Create `lower/entity.rs` with `partition_entity_members()` as the first function, then `lower_entity()` that calls it and drives struct + impl emission. Mirror the exact code style of `operator.rs`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `chumsky::span::SimpleSpan` | already in dep | All spans on AST nodes | Project-wide invariant; every AST node requires a `SimpleSpan` |
| `writ_parser::cst::*` | workspace crate | CST input types (`EntityDecl`, `EntityMember`, `UseField`, `Param`, `FnDecl`, `Stmt`) | Only source of entity CST data |
| `crate::ast::decl::*` | workspace crate | Output types (`AstStructDecl`, `AstImplDecl`, `AstFnDecl`, `AstStructField`, `AstAttribute`) | Sole AST representation |
| `crate::lower::context::LoweringContext` | project | Error accumulation, shared state | All passes receive `&mut LoweringContext` |
| `crate::lower::error::LoweringError` | project | Error reporting | `emit_error()` never halts; all errors accumulated |
| `insta` | already in dev-dep | Snapshot testing | Project-wide test pattern; `assert_debug_snapshot!` used for all lowering tests |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `crate::lower::expr::lower_expr` | project | Lower hook body expressions | Lifecycle hook bodies contain arbitrary Writ expressions |
| `crate::lower::stmt::lower_stmt` | project | Lower hook body statements | Hook `on create { stmts }` lowers each stmt via lower_stmt |
| `crate::lower::optional::lower_type` | project | Lower type expressions | Property types and UseClause component types need TypeExpr → AstType |
| `super::lower_fn`, `super::lower_param`, `super::lower_vis`, `super::lower_attrs` | project | Lower entity method FnDecl and helpers | Entity methods are FnDecl — reuse existing helper |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Manual `partition_entity_members()` fn | Iterating members inline | Manual partition is more readable and testable in isolation; locked by user decision |
| `$ComponentName` field naming | `_component_name` style | `$` is illegal in Writ identifiers so user code cannot shadow it; locked by user decision |
| Separate file `entity.rs` | Inline in `mod.rs` | All structural passes (operator.rs, dialogue.rs) are separate files; consistency requires entity.rs |

**Installation:** No new dependencies required. All necessary crates are already in the workspace.

---

## Architecture Patterns

### Recommended Project Structure

```
writ-compiler/src/lower/
├── mod.rs              # lower() entry point — wire lower_entity here (Item::Entity arm)
├── entity.rs           # NEW: lower_entity() → Vec<AstDecl>; partition_entity_members()
├── operator.rs         # existing: lower_operator_impls() → Vec<AstDecl> (reference pattern)
├── dialogue.rs         # existing: lower_dialogue() → AstFnDecl (reference pattern)
├── error.rs            # ADD: new LoweringError variants for entity-specific errors
└── context.rs          # unchanged
```

### Pattern 1: Multi-Declaration Emitter (mirrors operator.rs)

**What:** A single CST item maps to multiple AST declarations. The function returns `Vec<AstDecl>` and the call site uses `decls.extend(...)`.

**When to use:** Any CST item that expands into more than one AST declaration.

**Example (from operator.rs — the exact model to follow):**

```rust
// In lower/mod.rs:
Item::Impl((i, i_span)) => {
    decls.extend(lower_operator_impls(i, i_span, &mut ctx));
}

// The pattern to replicate for entity:
Item::Entity((e, e_span)) => {
    decls.extend(lower_entity(e, e_span, &mut ctx));
}
```

```rust
// In lower/entity.rs:
pub fn lower_entity(
    entity: EntityDecl<'_>,
    entity_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> Vec<AstDecl> {
    let entity_name = entity.name.0.to_string();
    let entity_name_span = entity.name.1;

    // Step 1: Lower attributes (check for [Singleton])
    let attrs = lower_attrs(entity.attrs, ctx);

    // Step 2: Partition members
    let (properties, use_clauses, methods, hooks) =
        partition_entity_members(entity.members, &entity_name, ctx);

    let mut result: Vec<AstDecl> = Vec::new();

    // Step 3: Emit struct (properties + component fields)
    result.push(AstDecl::Struct(build_struct(...)));

    // Step 4: Emit inherent impl for entity methods (if any)
    if !methods.is_empty() {
        result.push(AstDecl::Impl(build_methods_impl(...)));
    }

    // Step 5: Emit ComponentAccess<T> impl per use clause
    for use_clause in &use_clauses {
        result.push(AstDecl::Impl(build_component_access_impl(...)));
    }

    // Step 6: Emit lifecycle hook contract impls
    for hook in &hooks {
        result.push(AstDecl::Impl(build_hook_impl(...)));
    }

    result
}
```

### Pattern 2: Member Partitioning Pre-Step

**What:** Scan all members once, sort into four buckets (properties, use clauses, methods, hooks), perform all validation/error checks during this scan, return clean data to the emitters.

**When to use:** Any time a heterogeneous member list needs to be dispatched to multiple emission paths with cross-member validation.

**Example:**

```rust
/// Partitions entity members into four categories.
/// All duplicate/collision detection happens here.
/// Returns (properties, use_clauses, methods, hooks).
fn partition_entity_members<'src>(
    members: Vec<Spanned<EntityMember<'src>>>,
    entity_name: &str,
    ctx: &mut LoweringContext,
) -> (
    Vec<PartitionedProperty<'src>>,
    Vec<PartitionedUse<'src>>,
    Vec<PartitionedMethod<'src>>,
    Vec<PartitionedHook<'src>>,
) {
    let mut properties = Vec::new();
    let mut use_clauses = Vec::new();
    let mut methods = Vec::new();
    let mut hooks = Vec::new();
    let mut seen_props: Vec<String> = Vec::new();
    let mut seen_components: Vec<String> = Vec::new();
    let valid_events = ["create", "interact", "destroy"];

    for (member, member_span) in members {
        match member {
            EntityMember::Property { vis, name, ty, default } => {
                let name_str = name.0.to_string();
                if seen_props.contains(&name_str) {
                    ctx.emit_error(LoweringError::Generic {
                        message: format!("duplicate property `{name_str}` on entity `{entity_name}`"),
                        span: member_span,
                    });
                } else {
                    seen_props.push(name_str.clone());
                    properties.push(...);
                }
            }
            EntityMember::Use { component, fields } => {
                let comp_name = component.0.to_string();
                if seen_components.contains(&comp_name) {
                    ctx.emit_error(LoweringError::Generic {
                        message: format!("duplicate `use {comp_name}` in entity `{entity_name}`"),
                        span: component.1,
                    });
                } else {
                    // Also check property-component name collision
                    if seen_props.contains(&comp_name) {
                        ctx.emit_error(LoweringError::Generic {
                            message: format!("property `{comp_name}` conflicts with `use {comp_name}` in entity `{entity_name}`"),
                            span: member_span,
                        });
                    }
                    seen_components.push(comp_name);
                    use_clauses.push(...);
                }
            }
            EntityMember::Fn((fn_decl, fn_span)) => {
                methods.push(...);
            }
            EntityMember::On { event, params, body } => {
                let event_name = event.0;
                if !valid_events.contains(&event_name) {
                    ctx.emit_error(LoweringError::Generic {
                        message: format!("unknown lifecycle event `{event_name}` — valid events: create, interact, destroy"),
                        span: event.1,
                    });
                } else {
                    hooks.push(...);
                }
            }
        }
    }
    (properties, use_clauses, methods, hooks)
}
```

### Pattern 3: ComponentAccess Impl Shape

**What:** Each `use Component` clause produces one `impl ComponentAccess<ComponentName> for EntityName` block with a single `fn get(self) -> ComponentName { self.$ComponentName }` method.

**Example:**

```rust
// For: use Health { current: 80, max: 80 }
// Emits:
AstDecl::Impl(AstImplDecl {
    contract: Some(AstType::Generic {
        name: "ComponentAccess".to_string(),
        args: vec![AstType::Named { name: "Health".to_string(), span }],
        span,
    }),
    target: AstType::Named { name: entity_name.to_string(), span: entity_name_span },
    members: vec![AstImplMember::Fn(AstFnDecl {
        name: "get".to_string(),
        params: vec![],  // `self` is implicit in Writ
        return_type: Some(AstType::Named { name: "Health".to_string(), span }),
        body: vec![AstStmt::Return {
            value: Some(AstExpr::MemberAccess {
                object: Box::new(AstExpr::SelfLit { span }),
                field: "$Health".to_string(),
                field_span: span,
                span,
            }),
            span,
        }],
        span,
        ..
    })],
    span,
})
```

### Pattern 4: Lifecycle Hook → Contract Impl

**What:** Each `on create { body }` lowers to `impl OnCreate for Guard { fn on_create(self) { lowered_body } }`. Parameters from `on interact(who: Entity)` become function params.

**Contract and method name mapping:**

| Event | Contract Name | Method Name |
|-------|--------------|-------------|
| `create` | `OnCreate` | `on_create` |
| `interact` | `OnInteract` | `on_interact` |
| `destroy` | `OnDestroy` | `on_destroy` |

**Example:**

```rust
// For: on interact(who: Entity) { -> guardDialog(self, who) }
AstDecl::Impl(AstImplDecl {
    contract: Some(AstType::Named { name: "OnInteract".to_string(), span }),
    target: AstType::Named { name: entity_name.to_string(), span: entity_name_span },
    members: vec![AstImplMember::Fn(AstFnDecl {
        name: "on_interact".to_string(),
        params: vec![ /* lowered from (who: Entity) */ ],
        return_type: None,
        body: /* lowered stmts from hook body */,
        ..
    })],
    span,
})
```

### Pattern 5: Struct Field for Component (R13)

**What:** `use Health { current: 80, max: 80 }` → one `AstStructField` named `$Health` of type `Health` with a default initializer that is a struct literal containing only the specified overrides.

**Key insight:** The initializer is a `StructLit` (or equivalent call expression) containing only the user-specified field overrides. Fields absent from the `use` clause are NOT included — the type checker resolves component defaults.

**Example:**

```rust
// use Health { current: 80, max: 80 }
AstStructField {
    vis: None,
    name: "$Health".to_string(),
    name_span: component_span,
    ty: AstType::Named { name: "Health".to_string(), span: component_span },
    default: Some(AstExpr::StructLit {
        ty: AstType::Named { name: "Health".to_string(), span: component_span },
        fields: vec![
            ("current", AstExpr::IntLit { value: 80, span }),
            ("max", AstExpr::IntLit { value: 80, span }),
        ],
        span: component_span,
    }),
    span: component_span,
}

// use Speaker {} (empty override)
AstStructField {
    vis: None,
    name: "$Speaker".to_string(),
    ty: AstType::Named { name: "Speaker".to_string(), span },
    default: Some(AstExpr::StructLit {
        ty: AstType::Named { name: "Speaker".to_string(), span },
        fields: vec![],  // type checker fills all defaults
        span,
    }),
    span,
}
```

**IMPORTANT: Verify `AstExpr::StructLit` existence.** The `AstExpr` enum must have a `StructLit` variant. This needs to be confirmed before implementation — check `writ-compiler/src/ast/expr.rs`.

### Pattern 6: Singleton Attribute Propagation

**What:** If the entity has `[Singleton]` in its attribute list, that attribute is passed through verbatim to the generated `AstStructDecl.attrs`. No transformation needed — it's a copy-through from the entity's `lower_attrs()` result.

**Example:**

```rust
// [Singleton] entity Narrator { ... }
AstDecl::Struct(AstStructDecl {
    attrs: vec![AstAttribute {
        name: "Singleton".to_string(),
        name_span: attr_span,
        args: vec![],
        span: attr_span,
    }],
    name: "Narrator".to_string(),
    // ...
})
```

### Pattern 7: Emission Order (Claude's Discretion)

Recommended deterministic order for snapshot readability:

1. `AstDecl::Struct` (the struct definition with all fields)
2. `AstDecl::Impl` (inherent impl with entity methods, if non-empty)
3. `AstDecl::Impl` × N (one `ComponentAccess<T>` impl per `use` clause, in source order)
4. `AstDecl::Impl` × M (one lifecycle hook impl per `on` handler, in source order)

This order mirrors how a human would write the equivalent code: struct first, methods second, component access third, lifecycle fourth. It is deterministic (source order preserved) and produces stable snapshots.

### Anti-Patterns to Avoid

- **Halting on first error:** Follow the project pattern — emit error, skip problematic member, continue lowering remaining members. Never `panic!` or early-return from `lower_entity`.
- **Emitting entity-level span on all nodes:** Each emitted declaration should carry the span of the source construct that generated it (e.g., the `Use` span for `ComponentAccess` impls), not a single entity-wide span everywhere.
- **Using `SimpleSpan::new(0, 0)` for synthetic nodes:** All generated nodes must carry the span of their CST origin (`entity_span` or `member_span`). Zero spans on synthetic nodes violate R14.
- **Emitting an empty inherent impl:** Only emit the `impl Guard { methods }` block if there are actual methods. Mirrors the `operator.rs` pattern of suppressing empty base impls.
- **Putting entity methods in a contract impl:** Entity methods go in an inherent `impl Guard { ... }` (no `contract:` field), not in a contract impl block.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Type lowering (`TypeExpr` → `AstType`) | Custom type converter | `crate::lower::optional::lower_type` | Already handles nullable, generic, array, void; used by all prior passes |
| Statement lowering | Inline `match` in entity.rs | `crate::lower::stmt::lower_stmt` | Hook bodies are arbitrary Writ statements; lower_stmt handles the full grammar |
| Expression lowering | Inline expression fold | `crate::lower::expr::lower_expr` | Hook body expressions need all expression helpers (fmt_string, compound assign, etc.) |
| Function lowering | Copy-paste from mod.rs | `super::lower_fn` (already pub(crate)) | Entity methods are FnDecl — same path as regular functions |
| Attribute lowering | Inline attr parsing | `super::lower_attrs` | `[Singleton]` and other attributes need the same lowering path |
| Parameter lowering | Inline param conversion | `super::lower_param` | Hook `on interact(who: Entity)` params are CST Param — same as function params |

**Key insight:** Entity lowering reuses ALL existing lower_* helpers. The new code is purely structural — partition, validate, call helpers, assemble AstDecl values.

---

## Common Pitfalls

### Pitfall 1: `AstExpr::StructLit` May Not Exist

**What goes wrong:** The component field initializer needs to be a struct literal (`Health { current: 80, max: 80 }`). If `AstExpr` doesn't have a `StructLit` variant, there's no way to represent this.

**Why it happens:** The AST was defined in Phase 1 focusing on what existed at that time. Entity lowering is the first pass that needs to emit struct literals as default values.

**How to avoid:** Before implementation, check `writ-compiler/src/ast/expr.rs` for a `StructLit` variant. If absent, add it as part of Wave 0 / task 0 of Plan 01. The variant shape based on AstExpr patterns would be:

```rust
StructLit {
    ty: AstType,
    fields: Vec<(String, SimpleSpan, AstExpr)>,  // (field_name, name_span, value)
    span: SimpleSpan,
}
```

**Warning signs:** If plan tasks assume `AstExpr::StructLit` without first verifying it exists, the build will fail on the first test.

### Pitfall 2: `$` Prefix in Field Names is Valid in AST But Not Parseable as Writ

**What goes wrong:** Component fields are named `$Health`. The AST carries these as plain `String` fields, which is fine. But if any test tries to parse `self.$Health` as Writ source code, the parser will reject `$` as a valid identifier start (since `$` begins a formattable string in Writ).

**Why it happens:** The `$` naming convention makes the field user-unreachable FROM WRIT CODE, but the AST stores it as a plain string. Testing via `lower_src(...)` only parses Writ source — the generated AST field name never appears in source.

**How to avoid:** Write snapshot tests that lower an entity declaration and compare the whole AST structure. Never write a test that tries to parse source code containing `$ComponentName` as a field access. The `get(self)` method in `ComponentAccess` impl references `self.$Health` as an `AstExpr::MemberAccess` node — build this programmatically, not by parsing.

### Pitfall 3: Namespace Block Also Contains `Item::Entity`

**What goes wrong:** The `lower_namespace` function in `mod.rs` has its own `match item { ... }` arm list that currently has `Item::Entity(_) => todo!("Phase 5: entity lowering in namespace block")`. If `lower_entity` is only wired at the top-level `lower()` function, namespace-nested entities will still panic.

**Why it happens:** Discovered by reading `mod.rs` lines 111 and 331. Both `todo!` arms exist.

**How to avoid:** Wire `lower_entity` in BOTH places: the top-level `lower()` loop (line 111) AND the `lower_namespace` block arm (line 331). Use `decls.extend(lower_entity(e, e_span, ctx))` in both.

### Pitfall 4: Hook Body Expressions Need Full Expression Lowering

**What goes wrong:** A hook body like `on create { log($"Guard spawned: {self.name}"); }` contains a formattable string. If hook body statements are lowered with only a shallow stmt-level pass and not the full `lower_stmt` which calls `lower_expr`, the `$"..."` won't be desugared.

**Why it happens:** `lower_stmt` already chains to `lower_expr` for all expression positions. But if someone hand-writes a simpler pass for hook bodies, they'd skip expr lowering.

**How to avoid:** Always use `lower_stmt(s, ctx)` for each statement in a hook body. Never inline-handle stmts in entity.rs.

### Pitfall 5: Conflicting Method Name Error Already Declared But Deferred

**What goes wrong:** `LoweringError::ConflictingComponentMethod` is already declared in `error.rs` (with `method`, `first_component`, `second_component`, `span` fields). The CONTEXT.md says this error is DEFERRED to the type checker. Emitting it from lowering would be incorrect because lowering can't see component definitions.

**Why it happens:** The error was pre-declared in anticipation of being needed. But the decision log says cross-component method conflicts require component definition visibility that lowering doesn't have.

**How to avoid:** Do NOT emit `ConflictingComponentMethod` from entity lowering. Leave it for the type checker phase. The error variant remains in `error.rs` as a future type-checker error.

### Pitfall 6: Empty `use Component {}` vs `use Component { ... }`

**What goes wrong:** An empty `use Speaker {}` must still emit a struct field (`$Speaker: Speaker`) AND a `ComponentAccess<Speaker>` impl. If the implementation checks `if fields.is_empty() { skip }`, it will incorrectly omit the component.

**Why it happens:** Conflating "no field overrides" with "no component attachment."

**How to avoid:** The empty struct literal (`Speaker {}`) is always emitted as the default value for the field. An empty `fields` vec in `UseField` just means the struct literal has no entries — the field itself is always emitted.

---

## Code Examples

Verified patterns from the live codebase:

### Full Emission Pattern (mirrors operator.rs)

```rust
// Source: D:/dev/git/Writ/writ-compiler/src/lower/operator.rs lines 22-98
// lower_operator_impls returns Vec<AstDecl> — entity lowering returns the same

pub fn lower_entity(
    entity: EntityDecl<'_>,
    entity_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> Vec<AstDecl> {
    let mut result: Vec<AstDecl> = Vec::new();
    // ... build and push declarations ...
    result
}

// Call site in lower/mod.rs (replace todo!):
Item::Entity((e, e_span)) => {
    decls.extend(lower_entity(e, e_span, &mut ctx));
}
```

### AstImplDecl with Contract (mirrors operator.rs lines 149-154)

```rust
// Source: D:/dev/git/Writ/writ-compiler/src/lower/operator.rs
AstDecl::Impl(AstImplDecl {
    contract: Some(AstType::Generic {
        name: "ComponentAccess".to_string(),
        args: vec![AstType::Named { name: component_name.clone(), span: comp_span }],
        span: comp_span,
    }),
    target: AstType::Named { name: entity_name.clone(), span: entity_name_span },
    members: vec![AstImplMember::Fn(get_fn)],
    span: use_span,
})
```

### AstImplDecl without Contract (inherent impl for entity methods)

```rust
// Source: D:/dev/git/Writ/writ-compiler/src/lower/operator.rs lines 75-82
// The "no contract" case (contract: None) already exists in the codebase
AstDecl::Impl(AstImplDecl {
    contract: None,
    target: AstType::Named { name: entity_name.clone(), span: entity_name_span },
    members: fn_members,
    span: entity_span,
})
```

### AstStructDecl (from lower/mod.rs lower_struct)

```rust
// Source: D:/dev/git/Writ/writ-compiler/src/lower/mod.rs lines 351-370
AstStructDecl {
    attrs: lower_attrs(entity.attrs, ctx),  // [Singleton] passes through
    vis: lower_vis(entity.vis),
    name: entity_name.clone(),
    name_span: entity.name.1,
    generics: vec![],  // entity declarations have no generics
    fields: /* property fields + component fields */,
    span: entity_span,
}
```

### AstStructField with Default (component field)

```rust
// Source: D:/dev/git/Writ/writ-compiler/src/ast/decl.rs AstStructField
AstStructField {
    vis: None,
    name: format!("${}", component_name),  // "$Health"
    name_span: component_span,
    ty: AstType::Named { name: component_name.clone(), span: component_span },
    default: Some(/* StructLit or Call expr — see Pitfall 1 */),
    span: component_span,
}
```

### Snapshot Test Pattern

```rust
// Source: D:/dev/git/Writ/writ-compiler/tests/lowering_tests.rs
// All tests use lower_src() for success paths, lower_src_with_errors() for error paths

#[test]
fn entity_component_field_flattening() {
    let ast = lower_src(r#"
        entity Guard {
            name: string,
            use Health { current: 80, max: 80 },
        }
    "#);
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn entity_duplicate_use_clause_error() {
    let (ast, errors) = lower_src_with_errors(r#"
        entity Guard {
            use Health { current: 80 },
            use Health { max: 100 },
        }
    "#);
    insta::assert_debug_snapshot!((ast, errors));
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Item::Entity(_) => todo!(...)` | `decls.extend(lower_entity(...))` | Phase 5 | Removes last `todo!` arm from the item dispatch loop |
| Conflicting method detection in lowering | Deferred to type checker | CONTEXT.md decision | Lowering cannot see component definitions — correctness requires deferral |

**Deprecated/outdated:**
- N/A — this is a new pass with no prior implementation.

---

## Open Questions

1. **Does `AstExpr::StructLit` exist?**
   - What we know: `AstStructField.default: Option<AstExpr>` accepts any expression as a default value. Component overrides need to be emitted as struct-literal expressions.
   - What's unclear: Whether `AstExpr` already has a `StructLit` variant or whether it needs to be added. The expr.rs file was not read during this research session.
   - Recommendation: Read `writ-compiler/src/ast/expr.rs` as the FIRST task of Plan 01. If `StructLit` is absent, add it before writing entity.rs. If it's present, proceed directly.

2. **Hook body `self` parameter handling**
   - What we know: CONTEXT.md notes "Self/this semantics: the spec currently lacks explicit `self` parameter definition for instance methods — lowering should emit `self` matching spec examples." The `AstFnDecl` has a `params: Vec<AstParam>` field.
   - What's unclear: Whether `self` should appear in `params` as a named param, or whether it's implicit. Looking at `operator.rs`, `AstFnDecl` bodies use `AstExpr::SelfLit { span }` which suggests `self` is implicit (not in params).
   - Recommendation: Do NOT add `self` to the params list for hook methods. Use `AstExpr::SelfLit` when referencing self in hook bodies. This matches how derived operators reference `self_expr()` in operator.rs.

3. **Property field visibility**
   - What we know: CONTEXT.md says "Entity property fields (name, patrolRoute) retain their original names and visibility unchanged."
   - What's unclear: Whether the entity-level `vis` should override property-level `vis`, or whether each property carries its own visibility (which it does — `EntityMember::Property { vis, ... }` has its own visibility field).
   - Recommendation: Use the property's own `vis` field (from `EntityMember::Property { vis, ... }`), not the entity-level `vis`. The entity-level `vis` applies to the struct declaration itself.

---

## Validation Architecture

> `workflow.nyquist_validation` is not set in `.planning/config.json` — this section is omitted per research instructions.

The project's quality gate uses `insta` snapshot tests. The R15 requirement specifies: entity lowering tests must cover component flattening, lifecycle hooks, `[Singleton]`. Snapshot tests are added to `writ-compiler/tests/lowering_tests.rs` using the established `lower_src` / `lower_src_with_errors` helpers. Run with: `cargo test -p writ-compiler` and accept new snapshots with `INSTA_UPDATE=always cargo test -p writ-compiler`.

---

## Sources

### Primary (HIGH confidence)

- `D:/dev/git/Writ/writ-parser/src/cst.rs` (lines 350–395) — `EntityDecl`, `EntityMember`, `UseField` CST types. Direct inspection; no inference.
- `D:/dev/git/Writ/writ-compiler/src/ast/decl.rs` — Full `AstDecl`, `AstStructDecl`, `AstImplDecl`, `AstFnDecl`, `AstAttribute` definitions.
- `D:/dev/git/Writ/writ-compiler/src/lower/operator.rs` — Canonical multi-declaration emitter pattern (`Vec<AstDecl>` return, `decls.extend(...)` call site).
- `D:/dev/git/Writ/writ-compiler/src/lower/mod.rs` — Both `todo!` arms for `Item::Entity` (lines 111 and 331); all existing lower_* helpers and their signatures.
- `D:/dev/git/Writ/writ-compiler/src/lower/error.rs` — Pre-declared `LoweringError::ConflictingComponentMethod`; other existing error variants.
- `D:/dev/git/Writ/writ-compiler/src/lower/context.rs` — `LoweringContext` API; `emit_error()` semantics.
- `D:/dev/git/Writ/language-spec/spec/15_14_entities.md` — Entity declaration syntax, component access (§14.3), singleton semantics (§14.4).
- `D:/dev/git/Writ/language-spec/spec/29_28_lowering_reference.md` — §28.3 entity lowering conceptual description.
- `.planning/phases/05-entity-lowering/05-CONTEXT.md` — All locked user decisions.
- `.planning/STATE.md` — Project decisions log; confirmed `todo!` arms in mod.rs.

### Secondary (MEDIUM confidence)

- `D:/dev/git/Writ/writ-compiler/tests/lowering_tests.rs` — Test patterns (`lower_src`, `lower_src_with_errors`, `insta::assert_debug_snapshot!`). Directly observed; exact patterns confirmed.

### Tertiary (LOW confidence)

- None — all findings sourced from live code or spec, not web search.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates already in workspace; no new dependencies needed
- Architecture patterns: HIGH — directly modeled on `operator.rs` which is the established pattern for multi-declaration emitters
- Pitfalls: HIGH — all pitfalls sourced from actual code inspection (both `todo!` arms in mod.rs, pre-declared error variants, the AstExpr.StructLit open question)

**Research date:** 2026-02-26
**Valid until:** Until `writ-compiler/src/ast/expr.rs` is read (resolves Open Question 1 about `AstExpr::StructLit`)
