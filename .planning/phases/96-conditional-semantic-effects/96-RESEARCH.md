# Phase 96: [Conditional] Semantic Effects - Research

**Researched:** 2026-03-27
**Domain:** Writ compiler — attribute-driven conditional compilation (resolver + emitter + CLI)
**Confidence:** HIGH

## Summary

Phase 96 implements `[Conditional("name")]` as a compile-time emission filter: only the function
variant matching the active condition is emitted in the binary. A non-conditional fallback with
identical signature must always exist, and the resolver must verify this. Call-site arguments
must still type-check even when the call will be elided — elision is strictly an emit-time
decision, not a checker decision.

The design builds on two fully-established precedents: (1) Phase 95's `deprecated_items`
pattern in `TypeEnv` (a `FxHashMap<DefId, String>` populated by an attribute scan in
`env_build`) and (2) the existing `conditions: HashMap<String, bool>` field already present in
`WritConfig` (`config.rs`). The `conditions` map was scaffolded during earlier roadmap work and
already parses from `writ.toml`. Phase 96 must wire it through `run_pipeline` -> `typecheck` ->
`emit_bodies` as a `HashSet<String>` of active condition names.

The `--condition name` CLI flag (not `--condition name=bool`) is the correct choice: a flag
names an active condition; its absence means the condition is inactive. This matches C#'s
`#if DEBUG` model and the C++ `-D DEBUG` model — conditions are boolean presence, not key=value
pairs. The `writ.toml` `[conditions]` table with `bool` values already supports this via the
existing schema.

**Primary recommendation:** (1) Add `active_conditions: HashSet<String>` to `run_pipeline` and
`emit_bodies` signatures; (2) populate `conditional_fns: FxHashMap<DefId, String>` in
`TypeEnv::build` (parallel to `deprecated_items`); (3) verify fallback existence at resolver
time using a second pass over the DefMap; (4) filter `TypedDecl::Fn` in `collect_defs` based on
active conditions; (5) type-check call sites normally (no changes to check layer beyond the
fallback lookup map).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices are at Claude's discretion.

### Claude's Discretion
All implementation choices. Key design notes supplied:
- [Conditional] elision must happen at emit time only (EmitCtx, not CheckCtx) — args still
  type-check when call is elided
- Research gap: `--condition name` vs. `--condition name=bool` CLI syntax — decide during
  planning (resolved: `--condition name`, see Summary)
- Impl-block [Conditional] semantics need spec update before shipping

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| COND-01 | `[Conditional("name")]` function emits only the winning variant when condition is active via `writ.toml` or `--condition` flag | `active_conditions: HashSet<String>` threaded through pipeline; `collect_defs` skips `TypedDecl::Fn` whose condition is not in the active set |
| COND-02 | When no condition is active, the non-conditional fallback function is emitted | Fallback has no `[Conditional]` attribute; when no condition matches, all conditional variants are skipped and only the fallback emits |
| COND-03 | Resolver verifies fallback exists with matching signature; emitter errors on multiple conditions matching the same signature simultaneously | Two checks: (a) resolver pass after collect verifies each conditional fn has a fallback of identical arity/types; (b) emit-time check detects two active conditions that both match the same name+sig |
| COND-04 | Arguments at a `[Conditional]` call site still type-check even when the call is elided | No changes to check layer — type checking happens before emit; since call site resolves to the fallback signature (which has the same sig), type checking always succeeds |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rustc_hash::FxHashMap` / `FxHashSet` | (in-crate) | `conditional_fns` map in TypeEnv; `active_conditions` set in pipeline | All TypeEnv maps use FxHashMap; FxHashSet is same crate |
| `std::collections::HashSet` | std | `active_conditions` parameter type at pipeline boundary | Used by `WritConfig::conditions` already |
| `writ_diagnostics::Diagnostic::error` | (in-crate) | Emit E0009 for missing fallback, E0010 for ambiguous conditions | Established builder; same pattern as all resolution errors |
| `writ_compiler::config::WritConfig` | (in-crate) | Already has `conditions: HashMap<String, bool>` | Scaffolded; just needs to be read and filtered |
| `clap` | (in-crate) | `--condition` flag in `writ-cli/src/main.rs` | Already used for all CLI args |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `writ_compiler::ast::decl::AstAttribute` / `AstAttributeArg` | (in-crate) | Read `[Conditional("name")]` from function attrs | Used in `env_build` phase to scan attrs on `AstDecl::Fn` |
| `env_build::find_attrs_for_entry` | (in-crate) | Duplicate the `deprecated_items` scan pattern for conditional fns | Already established in `env_build.rs` for Phase 95 |
| `emit/collect/mod.rs::collect_defs` | (in-crate) | Skip conditional TypedDecl::Fn entries at emit time | The natural filter point — same file already dispatches by decl kind |

**Installation:** No new dependencies.

## Architecture Patterns

### Recommended Project Structure
No new files required. Changes are purely additive at well-defined seams:

```
writ-compiler/src/
├── config.rs                     # no change (conditions already parsed)
├── check/
│   ├── env.rs                    # add conditional_fns field; add fallback_for_conditional map
│   ├── env_build.rs              # populate conditional_fns; fallback verification pass
│   └── mod.rs                    # thread active_conditions through typecheck()
└── emit/
    └── collect/
        └── mod.rs                # filter TypedDecl::Fn in collect_defs by active_conditions
writ-cli/src/
├── main.rs                       # add --condition flag to Compile and Build subcommands
└── pipeline.rs                   # accept active_conditions: HashSet<String>; pass to emit_bodies
writ-diagnostics/src/
└── code.rs                       # add E0009 (missing fallback), E0010 (ambiguous conditions)
writ-golden/tests/golden/
├── conditional_active.writ       # golden: condition "debug" active
├── conditional_active.writil     # golden: only debug variant emitted
├── conditional_inactive.writ     # golden: no condition active
└── conditional_inactive.writil   # golden: only fallback emitted
```

### Pattern 1: active_conditions threading (HIGH confidence)

**What:** A `HashSet<String>` of active condition names flows from the CLI / writ.toml through
`run_pipeline` into `emit_bodies`. The check layer does NOT receive it — type checking runs
unconditionally on all functions (COND-04 guarantee).

**When to use:** This is the single source of truth for which conditions are active.

**Flow:**
```
writ.toml [conditions] + --condition CLI flag
     |
     v
active_conditions: HashSet<String>          (computed in cmd_compile / cmd_build)
     |
     v
run_pipeline(file_sources, module_name, emit_debug_info, active_conditions)
     |
     v
emit_bodies(typed_ast, interner, asts, emit_debug_info, sources, &active_conditions)
     |
     v
collect_defs(typed_ast, asts, interner, builder, diags, &active_conditions)
     |
     v
// For each TypedDecl::Fn { def_id }:
//   look up conditional_fns[def_id] -> Option<&str>
//   if Some(cond_name) && !active_conditions.contains(cond_name) -> skip
//   if Some(cond_name) &&  active_conditions.contains(cond_name) -> emit (suppress fallback)
//   if None -> emit only if no active condition matches this fn's name+sig
```

**Key insight:** `collect_defs` also needs to suppress the fallback when a conditional variant
IS being emitted. The fallback is identified as: a function with the same name and identical
signature but no `[Conditional]` attribute. When a conditional variant wins, its fallback must
be excluded from the binary.

### Pattern 2: conditional_fns in TypeEnv (HIGH confidence)

**What:** `TypeEnv` gains two new fields:
1. `conditional_fns: FxHashMap<DefId, String>` — maps `DefId` of a `[Conditional("name")]` fn
   to its condition string. Built in `env_build`, exact parallel to `deprecated_items`.
2. `fallback_for_conditional: FxHashMap<DefId, DefId>` — maps a conditional fn's `DefId` to
   its fallback fn's `DefId`. Built during fallback verification pass.

**Why TypeEnv and not the resolver output:** TypeEnv is the right home for attribute-derived
metadata (per Phase 93-94 decisions). The planner noted that DefEntry stays minimal.

**Example:**
```rust
// In env.rs — add two fields to TypeEnv
pub struct TypeEnv {
    // ... existing fields ...
    /// [Conditional("name")] functions, keyed by DefId, value is condition name string.
    pub conditional_fns: FxHashMap<DefId, String>,
    /// Maps conditional fn DefId -> fallback fn DefId (same name, same sig, no [Conditional]).
    pub fallback_for_conditional: FxHashMap<DefId, DefId>,
}
```

```rust
// In env_build.rs — extract_conditional_name helper (parallel to extract_deprecated_msg)
pub(super) fn extract_conditional_name(attrs: &[AstAttribute]) -> Option<String> {
    for attr in attrs {
        if attr.name == "Conditional" {
            for arg in &attr.args {
                if let AstAttributeArg::Positional(AstExpr::StringLit { value, .. }) = arg {
                    return Some(value.clone());
                }
            }
        }
    }
    None
}
```

```rust
// In env.rs TypeEnv::build — second pass: populate conditional_fns
for decl in &resolved.decls {
    let def_id = env_build::decl_def_id(decl);
    let entry = resolved.def_map.get_entry(def_id);
    let attrs = env_build::find_attrs_for_entry(asts, entry);
    if let Some(cond_name) = env_build::extract_conditional_name(&attrs) {
        env.conditional_fns.insert(def_id, cond_name);
    }
}
```

### Pattern 3: Fallback Verification Pass (HIGH confidence)

**What:** After building `conditional_fns`, a third pass verifies every conditional fn has a
matching fallback. Two errors:
- E0009: A `[Conditional("name")]` fn has no non-conditional fn with the same name and
  compatible signature.
- E0010: Multiple `[Conditional]` fns with different condition names but the SAME name+sig
  simultaneously have their conditions active (detected at emit time, not resolver time).

**Where:** The resolver verification (E0009) runs in `TypeEnv::build` after all sigs are
registered. The emit-time check (E0010) runs in `collect_defs`.

**Fallback lookup algorithm:**
```rust
// After building conditional_fns, verify fallbacks exist:
for (&cond_def_id, cond_name) in &env.conditional_fns {
    let cond_entry = resolved.def_map.get_entry(cond_def_id);
    let cond_sig = env.fn_sigs.get(&cond_def_id);

    // Find a non-conditional fn with same name in same namespace
    let fallback = resolved.def_map
        .by_fqn
        .values()
        .find(|&&other_id| {
            if other_id == cond_def_id { return false; }
            let other_entry = resolved.def_map.get_entry(other_id);
            if other_entry.name != cond_entry.name { return false; }
            if other_entry.namespace != cond_entry.namespace { return false; }
            // Not itself conditional
            if env.conditional_fns.contains_key(&other_id) { return false; }
            // Same signature
            if let (Some(a), Some(b)) = (cond_sig, env.fn_sigs.get(&other_id)) {
                return sigs_compatible(a, b);
            }
            false
        });

    match fallback {
        Some(&fb_id) => { env.fallback_for_conditional.insert(cond_def_id, fb_id); }
        None => {
            diags.push(/* E0009: no matching fallback for [Conditional("cond_name")] fn */);
        }
    }
}
```

**Signature compatibility for fallback matching:** Same param count, same return type, same
param types. Use `FnSig::params` len + type equality. Generics should also match (same number
of generic params).

**Important:** `fn_overloads` must also be searched, not just `by_fqn`, because the fallback
and the conditional variant are two different functions with the same name — exactly an overload
set. The DefMap already tracks these in `fn_overloads: FxHashMap<String, Vec<DefId>>`.

**Corrected algorithm using fn_overloads:**
```rust
// Look in fn_overloads for the same FQN
let fqn = if cond_entry.namespace.is_empty() {
    cond_entry.name.clone()
} else {
    format!("{}::{}", cond_entry.namespace, cond_entry.name)
};

let overload_set = resolved.def_map.fn_overloads.get(&fqn)
    .map(|v| v.as_slice())
    .unwrap_or_else(|| {
        // Single fn (no overloads registered) — by_fqn only has one
        std::slice::from_ref(resolved.def_map.by_fqn.get(&fqn).unwrap())
    });
```

### Pattern 4: Emit-time filtering in collect_defs (HIGH confidence)

**What:** `collect_defs` receives `&HashSet<String>` as a new parameter. For each
`TypedDecl::Fn { def_id }`, it checks `type_env.conditional_fns` to decide whether to emit.

**Decision matrix:**
| DefId in conditional_fns? | Condition active? | Fallback exists? | Action |
|---------------------------|-------------------|------------------|--------|
| Yes (cond_name) | Yes | Yes | emit this fn; skip fallback |
| Yes (cond_name) | No | Yes | skip this fn; emit fallback |
| No | N/A | N/A (is the fallback) | emit unless a conditional variant is being emitted |

**Problem:** `collect_defs` iterates `typed_ast.decls` in order. It needs to know in advance
which fallback DefIds to skip. The solution is a two-pass approach:

Pass 1 (pre-scan): compute `skipped_def_ids: HashSet<DefId>` before iterating:
```rust
let mut skipped_def_ids: HashSet<DefId> = HashSet::default();

for (cond_def_id, cond_name) in &type_env.conditional_fns {
    let is_active = active_conditions.contains(cond_name.as_str());
    if is_active {
        // Active: skip the fallback
        if let Some(&fb_id) = type_env.fallback_for_conditional.get(cond_def_id) {
            skipped_def_ids.insert(fb_id);
        }
    } else {
        // Inactive: skip this conditional variant
        skipped_def_ids.insert(*cond_def_id);
    }
}
```

Pass 2 (existing loop): skip any `TypedDecl::Fn { def_id }` where
`skipped_def_ids.contains(def_id)`.

**E0010 — ambiguous active conditions:** Also during pre-scan, if two conditional fn DefIds
with different `cond_name` values map to the same fallback fn, AND both conditions are active,
emit E0010. Check: for each fallback_id, collect all active conditional variants that point to
it. If count > 1, error.

**Important:** `collect_defs` currently has signature:
```rust
pub fn collect_defs(
    typed_ast: &TypedAst,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    diags: &mut Vec<Diagnostic>,
)
```
The TypeEnv is NOT currently passed to `collect_defs` — it lives in `TypedAst` only partially.
**Resolution:** Either (a) pass `&TypeEnv` explicitly as a new parameter, or (b) include
`conditional_fns` and `fallback_for_conditional` on `TypedAst` itself.

Looking at `check/mod.rs`, `typecheck()` returns `(TypedAst, TyInterner, TypeEnv, Vec<Diagnostic>)`.
`pipeline.rs` discards `_type_env`. The cleanest approach for Phase 96 is to pass
`conditional_fns` and `fallback_for_conditional` embedded in `TypedAst`, or pass TypeEnv to
`collect_defs`. The simplest is to add a `conditional_fns` field to `TypedAst` in `check/ir.rs`,
populated at the end of `typecheck()`, so the collect pass has access without a new function
parameter.

### Pattern 5: CLI --condition flag (HIGH confidence)

**What:** Add `--condition <name>` (repeatable) to both `Compile` and `Build` subcommands.

**clap syntax:**
```rust
/// Activate a named compilation condition (repeatable: --condition debug --condition profile)
#[arg(long, action = clap::ArgAction::Append)]
condition: Vec<String>,
```

The `Vec<String>` is then converted to a `HashSet<String>` before calling `run_pipeline`. The
`WritConfig::conditions` map (from `writ.toml`) also contributes: any key with `true` value is
added to the active set.

**Merging writ.toml and CLI conditions:**
```rust
let mut active: HashSet<String> = config.conditions
    .iter()
    .filter(|(_, &v)| v)
    .map(|(k, _)| k.clone())
    .collect();
// CLI --condition flags override / augment
active.extend(cli_conditions.into_iter());
```

**For `writ compile` (single-file mode):** No `writ.toml` is loaded. Only `--condition` CLI
flags apply. Active set is just the CLI-provided names.

### Anti-Patterns to Avoid

- **Elision in the type checker:** COND-04 requires that args type-check even when elided.
  The checker must never see "this call is conditional and inactive, skip it." The checker
  always sees both the conditional and fallback variants; the emitter decides which to emit.

- **Storing condition state in DefEntry:** DefEntry stays minimal (no `condition: Option<String>`
  field). Attribute-derived metadata lives in TypeEnv. Same rule as Phase 95 deprecated_items.

- **Using DefMap.by_fqn for overload search:** When a conditional fn and its fallback share
  the same name, they form an overload set and live in `fn_overloads`, not as two separate
  entries in `by_fqn`. The fallback lookup must use `fn_overloads`.

- **Eliding at the resolve stage:** Resolver should not suppress conditional fns. All
  functions — conditional and fallback — must be in the DefMap for the checker to type-check
  call sites against the fallback signature (COND-04). Elision is emit-only.

- **Suppressing fallback in the type env:** `fn_sigs` must contain entries for both the
  conditional fn and its fallback. The call-site lookup in `resolve_overloaded_call` uses
  `fn_sigs` — if the conditional fn is absent from `fn_sigs`, the checker would fall back to
  the fallback sig anyway (correct), but the `conditional_fns` map must still be populated.

- **Missing the `fn_overloads` registration:** The collector (`collector.rs`) calls
  `DefMap::insert`. For functions, if a FQN already exists and both are `DefKind::Fn`, they go
  into `fn_overloads`. The conditional fn and fallback share the same FQN, so they WILL both be
  in `fn_overloads`. Verify this assumption by checking `DefMap::insert` logic (confirmed:
  lines 76-80 of `def_map.rs` show the overload registration path).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Attribute scanning | Inline match in collect_defs | `env_build::find_attrs_for_entry` + `extract_conditional_name` | Phase 95 precedent; O(1) lookup vs. O(n) inline scan |
| CLI repeatable flags | Manual Vec parsing | `clap #[arg(action = ArgAction::Append)]` | Established clap pattern; used for similar flags in other tools |
| Condition merging | Separate codepaths for toml vs. CLI | Single `HashSet<String>` union | Single source of truth; avoids priority bugs |
| Signature compatibility | Custom sig comparison | Reuse existing FnSig fields (params len + types) | FnSig already has all needed data |
| Multiple-active-condition detection | Complex data structure | Simple: per-fallback-id count of active conditionals | O(n) over conditional_fns; cheap and straightforward |

**Key insight:** The entire conditional mechanism is emit-time filtering over existing
infrastructure. No new IL instructions, no new AST nodes, no new type system features.

## Common Pitfalls

### Pitfall 1: Forgetting to suppress the fallback when a conditional wins
**What goes wrong:** Both the `[Conditional("debug")]` variant and the fallback are emitted
when `debug` is active — the binary has two functions with the same name and identical
signature, causing a token collision.
**Why it happens:** `collect_defs` skips the inactive conditional variant but forgets to also
skip the fallback when the active variant is emitted.
**How to avoid:** Pre-scan: for each active condition, add the fallback's DefId to
`skipped_def_ids`. See Pattern 4 above.
**Warning signs:** Golden test for "condition active" has two MethodDef rows for the same
function name.

### Pitfall 2: fn_overloads vs. by_fqn for fallback lookup
**What goes wrong:** Fallback verification passes (E0009 not emitted) but the fallback DefId
is never actually found, causing emit to produce an empty binary or panic.
**Why it happens:** `by_fqn` only stores the FIRST overload. The fallback may be the second
entry in an overload set, so `by_fqn.get(fqn)` returns the conditional variant, not the
fallback.
**How to avoid:** Always use `fn_overloads` for multi-variant lookups. Fall back to the single
`by_fqn` entry only when `fn_overloads` has no entry for that FQN.
**Warning signs:** E0009 fires for a function that clearly has a fallback defined in source.

### Pitfall 3: TypeEnv not available in collect_defs
**What goes wrong:** `collect_defs` can't access `conditional_fns` or `fallback_for_conditional`
because TypeEnv is not currently passed to it.
**Why it happens:** The collect pass only receives `TypedAst`, not `TypeEnv`.
**How to avoid:** Add `conditional_fns: FxHashMap<DefId, String>` and
`fallback_for_conditional: FxHashMap<DefId, DefId>` to `TypedAst` in `check/ir.rs`. Populate
them in `check/mod.rs::typecheck()` after `TypeEnv::build`. This avoids a new function
parameter and keeps the data co-located with the typed AST.
**Warning signs:** Compilation error: `collect_defs` can't find `conditional_fns`.

### Pitfall 4: Pipeline discards TypeEnv before emit
**What goes wrong:** `run_pipeline` does `let (typed_ast, interner, _type_env, type_diags)` —
TypeEnv is dropped before `emit_bodies` is called. If conditional data lives only in TypeEnv,
it's lost.
**Why it happens:** TypeEnv is returned from `typecheck()` and then discarded in `pipeline.rs`.
**How to avoid:** The resolution in Pitfall 3 (embed in TypedAst) sidesteps this entirely.
Alternatively, return `conditional_fns` as a separate parameter. Embedding in TypedAst is
cleaner.
**Warning signs:** active_conditions reach emit_bodies but conditional_fns is always empty.

### Pitfall 5: --condition in writ compile vs. writ build
**What goes wrong:** `writ compile` applies `--condition` but does NOT load `writ.toml`, so
`conditions` from config are never merged.
**Why it happens:** `cmd_compile` uses single-file mode without a project root.
**How to avoid:** For `writ compile`, active conditions = only CLI `--condition` flags. For
`writ build`, active conditions = union of writ.toml `[conditions]` where value is `true` PLUS
CLI `--condition` flags. Document this clearly in the plan.
**Warning signs:** Golden test for "active via writ.toml" passes for `writ build` but fails for
`writ compile`.

### Pitfall 6: Impl-block [Conditional] semantics
**What goes wrong:** A `[Conditional]` attribute on an `impl` block causes undefined behavior
or crashes.
**Why it happens:** The resolver and emitter only handle `[Conditional]` on `fn` declarations.
**How to avoid:** Scope Phase 96 to `fn` declarations only. If `[Conditional]` appears on
anything other than a `fn`, the attribute is silently passed through (existing behavior —
attributes on other decls go into the AttributeDef table only). The spec update for impl-block
`[Conditional]` is explicitly deferred per STATE.md.
**Warning signs:** Test with `[Conditional("x")] struct Foo {}` causes a panic or spurious
E0009.

### Pitfall 7: COND-03 error code assignments
**What goes wrong:** E0009 and E0010 are reserved or conflict with existing codes.
**Why it happens:** `code.rs` currently defines E0001-E0008 in the E00xx range and E0100-E0124
in the type error range. E0009 and E0010 are available in the E00xx resolution error range.
**How to avoid:** Assign E0009 = "missing conditional fallback" and E0010 = "ambiguous active
conditions". Confirmed by reading `code.rs` — E0009 and E0010 are currently unused.
**Warning signs:** compile error: duplicate const name in `code.rs`.

## Code Examples

Verified patterns from in-codebase sources:

### Existing conditions field in WritConfig (config.rs:22)
```rust
// Source: writ-compiler/src/config.rs
/// Conditional compilation flags.
#[serde(default)]
pub conditions: HashMap<String, bool>,
```
This is already there — no config changes needed. Just read and filter.

### active_conditions HashSet construction from writ.toml
```rust
// In cmd_build (or run_pipeline caller):
let active_conditions: std::collections::HashSet<String> = config.conditions
    .iter()
    .filter_map(|(k, &v)| if v { Some(k.clone()) } else { None })
    .chain(cli_condition_flags.into_iter())
    .collect();
```

### clap repeatable flag for --condition
```rust
// In main.rs Commands::Compile variant:
/// Activate a named compilation condition (may be repeated)
#[arg(long, action = clap::ArgAction::Append)]
condition: Vec<String>,
```

### Adding fields to TypeEnv (env.rs)
```rust
// Source pattern: writ-compiler/src/check/env.rs:55-70
pub struct TypeEnv {
    // ... existing fields (fn_sigs, struct_fields, ..., deprecated_items) ...
    /// [Conditional("name")] functions mapped to their condition string.
    pub conditional_fns: FxHashMap<DefId, String>,
    /// Maps conditional fn DefId -> fallback fn DefId.
    pub fallback_for_conditional: FxHashMap<DefId, DefId>,
}
```

### Embedding conditional data in TypedAst (check/ir.rs pattern)
```rust
// In TypedAst (check/ir.rs):
pub struct TypedAst {
    pub decls: Vec<TypedDecl>,
    pub def_map: DefMap,
    pub struct_field_types: FxHashMap<DefId, Vec<(String, Ty)>>,
    // NEW:
    /// Condition-name map for [Conditional] functions. Empty when no conditions are used.
    pub conditional_fns: FxHashMap<DefId, String>,
    /// Conditional fn -> fallback fn mapping. Empty when no conditions are used.
    pub fallback_for_conditional: FxHashMap<DefId, DefId>,
}
```

### collect_defs filtering (emit/collect/mod.rs pattern)
```rust
// In collect_defs, before the TypedDecl::Fn arm:
// Pre-scan: determine which DefIds to skip
let skipped = compute_skipped_def_ids(&typed_ast.conditional_fns,
                                       &typed_ast.fallback_for_conditional,
                                       active_conditions,
                                       diags);

// In the loop:
TypedDecl::Fn { def_id, .. } => {
    if skipped.contains(def_id) {
        continue; // elide this variant
    }
    collect_fn(*def_id, def_map, asts, interner, builder, &mut methoddef_handles, diags);
}
```

### Error code additions (diagnostics/src/code.rs)
```rust
// After E0008:
pub const E0009: &str = "E0009"; // missing [Conditional] fallback
pub const E0010: &str = "E0010"; // ambiguous active conditions
```

### find_attrs_for_entry and extract_conditional_name (env_build.rs)
```rust
// Source pattern: writ-compiler/src/check/env_build.rs (Phase 95 extract_deprecated_msg)
pub(super) fn extract_conditional_name(attrs: &[AstAttribute]) -> Option<String> {
    for attr in attrs {
        if attr.name == "Conditional" {
            for arg in &attr.args {
                if let AstAttributeArg::Positional(AstExpr::StringLit { value, .. }) = arg {
                    return Some(value.clone());
                }
            }
        }
    }
    None
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No conditional compilation | N/A (never existed) | Phase 96 | First conditional compilation feature |
| `conditions: HashMap<String, bool>` in WritConfig was unread | Parsed but discarded | Already in config.rs | Phase 96 wires it to the pipeline |

**Deprecated/outdated:**
- None — this is a new feature.

## Open Questions

1. **Should `writ.toml` [conditions] with value `false` suppress CLI `--condition`?**
   - What we know: The `conditions` map uses `bool` values. A `false` entry means "inactive".
   - What's unclear: If writ.toml has `debug = false` but CLI passes `--condition debug`,
     should `debug` be active or not?
   - Recommendation: CLI `--condition` always wins (additive union). The `false` value in
     writ.toml is purely documentation. CLI flags are not suppressible via toml. This matches
     the `-D` model in C/C++.

2. **What error is emitted when `[Conditional]` appears with no argument?**
   - What we know: `extract_conditional_name` returns `None` if no string arg is found.
   - What's unclear: Should `[Conditional]` with no arg be an error or silently ignored?
   - Recommendation: Emit E0006 ("invalid attribute target" / "invalid attribute argument") —
     reuse the existing validation error code. Or add a new error code E0009.1. Simpler: make
     the absence of a string arg emit an `E0009`-style error with message "Conditional requires
     a string argument". The planner should decide the specific code.

3. **Does fn_sigs contain BOTH the conditional variant and the fallback?**
   - What we know: Both are `DefKind::Fn` and both go through the normal `find_fn_decl` +
     `build_fn_sig` path in `TypeEnv::build`.
   - What we infer: Yes — both DefIds are in fn_sigs. The overload resolution in
     `resolve_overloaded_call` will find both as candidates. The checker picks the one whose
     signature matches the call args. Since both have the SAME signature, the overload
     resolution will see two identical candidates and could emit E0124 (ambiguous overload).
   - Resolution needed: The checker needs to understand that two overloads with identical
     signatures where one is `[Conditional]` are NOT ambiguous — the conditional is filtered.
     **This is a critical design issue.** The resolution: make `resolve_overloaded_call` prefer
     the non-conditional (fallback) variant when both match. Since the check layer does not
     receive `active_conditions`, it always checks against the fallback. This requires that
     the conditional fn's DefId be filtered OUT of `find_fn_candidates` in the checker.

   **Revised checker strategy for COND-04:** `find_fn_candidates` in `check/check_expr/mod.rs`
   must filter out DefIds that are in `conditional_fns`. The checker always resolves calls to the
   fallback, regardless of active conditions. The emitter then decides which physical function
   to emit. This means:
   - Checker: `find_fn_candidates` excludes conditional fn DefIds; always resolves to fallback.
   - Emitter: when condition is active, emits the conditional fn (not the fallback); when
     inactive, emits the fallback.
   - TypedExpr::Call always references the fallback's DefId at checker time.
   - The emitter sees the fallback's DefId in the call instruction and maps it to whichever
     physical MethodDef token was emitted (conditional if active, fallback otherwise).

   This avoids the E0124 ambiguity and satisfies COND-04 elegantly.

## Environment Availability

Step 2.6: SKIPPED — pure in-codebase Rust changes, no external tool dependencies.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`) |
| Config file | `Cargo.toml` (workspace) |
| Quick run command | `cargo test -p writ-golden conditional 2>&1` |
| Full suite command | `cargo test --workspace 2>&1` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| COND-01 | `--condition debug` causes only debug variant in binary | golden | `cargo test -p writ-golden conditional_active` | ❌ Wave 0 |
| COND-02 | No active condition: only fallback emitted | golden | `cargo test -p writ-golden conditional_inactive` | ❌ Wave 0 |
| COND-03 | Missing fallback produces E0009 | unit | `cargo test -p writ-compiler conditional_missing_fallback` | ❌ Wave 0 |
| COND-03 | Multiple active conditions on same sig produce E0010 | unit | `cargo test -p writ-compiler conditional_ambiguous` | ❌ Wave 0 |
| COND-04 | Call args type-check when condition inactive | unit | `cargo test -p writ-compiler conditional_typecheck_passthrough` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler --lib 2>&1`
- **Per wave merge:** `cargo test --workspace 2>&1`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-golden/tests/golden/conditional_active.writ` — COND-01 golden source
- [ ] `writ-golden/tests/golden/conditional_active.writil` — COND-01 expected IL (bless)
- [ ] `writ-golden/tests/golden/conditional_inactive.writ` — COND-02 golden source
- [ ] `writ-golden/tests/golden/conditional_inactive.writil` — COND-02 expected IL (bless)
- [ ] Unit tests in `writ-compiler/tests/` for E0009, E0010, COND-04 type-check passthrough
- [ ] Update `golden_tests.rs` to accept `active_conditions` parameter for compile_and_disassemble
  (or add a new `compile_and_disassemble_with_conditions` variant)

## Sources

### Primary (HIGH confidence)
- `writ-compiler/src/config.rs` — `WritConfig::conditions: HashMap<String, bool>` confirmed scaffolded
- `writ-compiler/src/check/env.rs` — TypeEnv field patterns; `deprecated_items` parallel pattern
- `writ-compiler/src/check/env_build.rs` — `extract_deprecated_msg` pattern; `find_attrs_for_entry`
- `writ-compiler/src/resolve/def_map.rs:76-80` — overload set registration in `fn_overloads`
- `writ-compiler/src/emit/collect/mod.rs` — `collect_defs` loop structure; TypedDecl::Fn arm
- `writ-compiler/src/emit/mod.rs` — `emit_bodies` signature; collect_defs call chain
- `writ-compiler/src/check/check_expr/call.rs` — `resolve_overloaded_call`; `find_fn_candidates`
- `writ-compiler/src/check/check_expr/mod.rs` — CheckCtx structure; no active_conditions field
- `writ-compiler/src/check/check_decl.rs` — `check_fn_decl`; current_file pattern
- `writ-diagnostics/src/code.rs` — E0009/E0010 are unassigned; safe to use
- `writ-cli/src/main.rs` — clap Commands enum; Compile and Build variants
- `writ-cli/src/pipeline.rs` — `run_pipeline` signature; TypeEnv discarded after typecheck
- `writ-cli/src/commands/compile.rs` — `cmd_compile` structure
- `writ-golden/tests/golden_tests.rs` — `compile_and_disassemble` pipeline; 16MB stack thread

### Secondary (MEDIUM confidence)
- None needed — all findings verified directly from source.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries are in-crate; `conditions` field already exists
- Architecture: HIGH — all injection points verified by direct code reading; TypeEnv pattern
  confirmed by Phase 95 precedent; overload mechanism confirmed in def_map.rs
- Pitfalls: HIGH — identified by tracing actual code paths; Open Question 3 is a critical
  design issue that the planner MUST resolve (checker must exclude conditional fn DefIds from
  candidate resolution to avoid E0124 ambiguity)
- CLI integration: HIGH — clap patterns confirmed from main.rs; run_pipeline signature read

**Research date:** 2026-03-27
**Valid until:** 60 days (stable Rust codebase, no external dependency drift)

## Project Constraints (from CLAUDE.md)

CLAUDE.md does not exist in the working directory — no project-level directives to enforce
beyond the conventions already documented in this research (TypeEnv pattern, FxHashMap usage,
diagnostic builder API, golden test workflow).
