# Phase 22: Name Resolution - Research

**Researched:** 2026-03-02
**Domain:** Compiler name resolution, symbol table design, Rust diagnostic libraries
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Error Diagnostic Style**
- Rust-style rich diagnostics: colored spans, source context, multi-span annotations, numbered error codes
- Error codes use E-series numbering (E0001, E0002, etc.) for errors; warning/error code series distinction is Claude's discretion
- ANSI color output from the start, with a `--no-color` flag for CI/piping (red=errors, yellow=warnings, blue=notes, bold=emphasis)
- Every error includes a `help:` note suggesting a fix — always, not just when obvious
- Ambiguous name errors show ALL candidate spans with file:line location and namespace
- Multi-file errors use inline multi-file rendering (all spans in one diagnostic, labeled by file)
- Report all errors — no max error cap, never stop early
- Emit both errors AND warnings (e.g., unused imports, unreachable `using`)
- Warnings suppressible via `[allow(unused_import)]` attributes AND CLI flags (`--no-warn XYZ0000`)
- Writ-specific terminology in error messages (entity, contract, component, dialogue), with flexibility to use "type" for entity where clearer
- Shared `writ-diagnostics` crate: used by parser, compiler, and runtime for unified diagnostic types and rendering
- Diagnostic rendering library: Claude's discretion (evaluate codespan-reporting, ariadne, miette)

**writ-runtime Type Availability**
- Implicit prelude: all writ-runtime types (Option, Result, Range, Array, Entity) AND all 17 contracts are always in scope without `using`
- Prelude types are NOT shadowable — defining a type with a prelude name is a compile error with a specific "reserved prelude type" error message
- Unqualified-only access: no `writ_runtime::` namespace prefix. Prelude types have no namespace
- `null` is a hard keyword that always resolves to `Option::None` — independent of prelude mechanism
- Primitives (int, float, string, bool, void) and prelude types use the same built-in scope mechanism in the resolver
- Reserved prelude names are hard-coded in the compiler source (const array)
- Entity is an implicit base type: every `entity Foo { }` implicitly has Entity as its base type; `Entity` parameters accept any entity
- Track original syntax through lowering: errors for `Option<T>` (desugared from `T?`) show both forms: "`Option<string>` (written as `string?`)"
- Name resolution verifies `Array<T>` exists and `T[]` desugars correctly; method resolution (`.push()`, `.pop()`) deferred to type checking
- Generic bounds checking (e.g., `E: Error` on Result) and component-type validation in `use` clauses: Claude's discretion on phase boundary

**Fuzzy Suggestions ("Did you mean?")**
- Suggestion count, scope (current vs. unimported namespaces), case sensitivity, and context-awareness: all Claude's discretion
- Must implement RES-12: unresolved names produce "did you mean `survival::HealthPotion`?" style suggestions

**Shadowing Policy**
- Local-to-local shadowing (`let x = 1; { let x = 2; }`) is allowed silently — no warning (core language feature)
- Import shadowing: local declaration shadowing a name from `using` produces a warning
- Generic type parameter shadowing outer type names: allowed with warning
- Top-level declaration shadowing a `using`-imported name: Claude's discretion

**Multi-file Compilation Model**
- Full `writ.toml` parsing: read `[compiler].sources` directories, recursively discover `.writ` files
- File processing order: non-deterministic (two-pass collection handles it; correctness doesn't depend on order)
- Namespace/path mismatch: emit a warning when file path doesn't match namespace declaration (spec says convention, not requirement)
- Duplicate definitions: two declarations with the same name in the same namespace from different files is a compile error

### Claude's Discretion
- Diagnostic rendering library choice (codespan-reporting vs ariadne vs miette)
- Warning/error code series format (separate E/W series vs unified)
- Prelude injection mechanism (virtual `using` vs direct root scope injection)
- Top-level declaration shadowing `using`-imported names (warning vs silent)
- Fuzzy suggestion implementation details (edit distance, count, scope, case sensitivity, context-awareness)
- Generic bounds checking phase boundary (resolution vs type checking)
- Component-type validation in entity `use` clauses (resolution vs type checking)
- Output IR structure (annotated AST vs new IR type with symbol IDs)
- Impl block association details (orphan rules, cross-namespace impls)

### Deferred Ideas (OUT OF SCOPE)
- Document reserved prelude names in the language spec — spec update, not compiler work
- `.editorconfig` support for diagnostic configuration — future CLI/tooling phase
- Language server warning for namespace/path mismatch — language server phase
- `writ-std` standard library types — future, not part of core spec (G3 in IL TODO)
</user_constraints>

---

## Summary

Phase 22 implements a classic two-pass name resolver over the `Ast` produced by Phase 21 (lowering). Pass 1 collects all top-level declarations into a `DefMap` keyed by fully qualified name; pass 2 walks every declaration body resolving every `AstType`, `AstExpr::Ident`/`Path`, and `AstExpr::New` reference against the collected symbols. The phase produces a `NameResolvedAst` (new IR type, not in-place mutation) and a `writ-diagnostics` crate consumed by all subsequent phases.

The technical complexity concentrates in three areas: (1) the namespace model — declarative vs. block namespaces, `using` scoping rules, cross-file same-namespace visibility, and `::` qualified paths including root-anchored `::name`; (2) the prelude injection of writ-runtime types and all 17 contracts into every file's built-in scope; and (3) Rust-style rich diagnostics with multi-span, multi-file rendering. These are all well-understood patterns with solid prior art in rust-analyzer's DefMap and ariadne for rendering.

**Primary recommendation:** Use `ariadne 0.6` for diagnostic rendering (already in writ-parser dev-deps; no new crate to evaluate), `toml 0.9` + `serde` for `writ.toml` parsing, and `strsim 0.11` for "did you mean?" edit-distance suggestions. Model the symbol table as a `HashMap<String, DefEntry>` keyed by fully-qualified name (`ns::Name`), with a separate file-local table per source file for private declarations.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| RES-01 | Compiler collects all top-level declarations (fn, struct, entity, enum, contract, impl, component, extern, const, global) across all files into a symbol table keyed by namespace-qualified name | Two-pass architecture: Pass 1 walks `Ast.items` building `DefMap`. Namespace context tracked via namespace stack mirroring `LoweringContext.namespace_stack` pattern. All `AstDecl` variants catalogued. |
| RES-02 | Compiler resolves `using` declarations (plain and alias forms) with correct scoping and conflict detection | `AstUsingDecl` has `alias: Option<String>` and `path: Vec<String>`. Resolution maps alias (or terminal segment) to the namespace's public DefMap. Conflict detection at usage site, not `using` site (spec §23.7). |
| RES-03 | Compiler resolves qualified paths (`ns::Name`, `Enum::Variant`, `::root::Name`) | `AstExpr::Path { segments }` and `AstType::Named/Generic`. Root-anchor: leading `::` in `segments[0]` (or rooted flag from CST — check lowering preserves this). Resolution: segment[0] matches namespace or type name; if type, subsequent segments are variant access. |
| RES-04 | Compiler enforces visibility rules (private, pub, type-private) including same-namespace cross-file visibility | Visibility stored in `AstVisibility` on each decl. DefEntry carries `vis`, `file_id`, `namespace`. Rules: cross-file same-namespace: pub only; cross-namespace: pub only; type-member: pub or same type. |
| RES-05 | Compiler resolves every `AstType` to a TypeRef blob or primitive tag, including writ-runtime virtual module types | `AstType` variants: Named, Generic, Array, Func, Void. Primitives: direct tag lookup. Named/Generic: lookup in DefMap + prelude. Array desugars to `Array<T>`. Func produces delegate signature blob. |
| RES-06 | Compiler associates impl blocks with their target type and contract definitions | `AstImplDecl` has `contract: Option<AstType>` and `target: AstType`. Resolution: resolve both types to DefEntries; associate impl methods with target TypeDef's method list. Cross-namespace impl rules TBD (Claude's discretion). |
| RES-07 | Compiler scopes generic type parameters to their declaration and body, with correct shadowing | `AstGenericParam` list on fn/struct/entity/enum/contract/impl decls. Resolver maintains a generic param scope stack. On entry to a generic decl, push params as Named types; pop on exit. Warning if shadows outer type name. |
| RES-08 | Compiler resolves `self`/`mut self` in method bodies to the enclosing type | `AstFnParam::SelfParam { mutable }` in method bodies. Resolver context tracks "current self type" when entering an impl method or entity method body. `AstExpr::SelfLit` resolves to the current self type. |
| RES-09 | Compiler validates `@Speaker` names resolve to `[Singleton]` entities with Speaker component | Speaker names from lowered dialogue fns (stored as identifier in fn body). Lookup speaker name in DefMap; verify: kind=Entity, has [Singleton] attribute, has Speaker component slot. |
| RES-10 | Compiler validates `[Singleton]` and `[Conditional]` attributes target allowed declaration kinds | `AstAttribute` on each decl. Scan attrs in Pass 2; [Singleton] only on Entity; [Conditional] only on Fn (spec §16.4). Unknown attributes: warning (not error) unless reserved. |
| RES-11 | Compiler detects ambiguous names from multiple `using` imports with both candidate spans | When resolving an unqualified name: collect all matching candidates across active `using` imports. If count > 1, emit ambiguity error with ALL candidate spans (file + span of each `using` decl + declaration site). |
| RES-12 | Compiler suggests similar names on resolution failure ("did you mean `survival::HealthPotion`?") | On unresolved name: use strsim (Jaro-Winkler or Levenshtein) against all visible names. Show top-N suggestions in `help:` note. Include unimported candidates as "did you mean X? (add `using survival;`)". |
</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ariadne | 0.6 | Diagnostic rendering with colored multi-span, multi-file output | Already in writ-parser dev-deps; 0.6 is current stable; supports multi-file Cache trait; rich label API |
| toml | 0.9 | Parse `writ.toml` project config | Official TOML crate; serde-compatible; simple `from_str` API; no overhead for the small config format |
| serde | 1.x | Derive Deserialize for writ.toml structs | Already used across Rust ecosystem; pairs naturally with toml crate |
| strsim | 0.11 | "Did you mean?" fuzzy name suggestions | Used by rustc itself for suggestions; Jaro-Winkler + Levenshtein; zero-cost when no error |
| walkdir | 2.x | Recursive `.writ` file discovery | Standard choice for recursive directory traversal; simple iterator API |
| thiserror | 2.0 | Error enum derives for `ResolutionError` | Already in writ-compiler; consistent with existing `LoweringError` pattern |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| rustc-hash | 2.1.1 | `FxHashMap`/`FxHashSet` for DefMap | STATE.md already lists this as approved for writ-compiler; faster than std HashMap for string keys |
| id-arena | 2.3.0 | Arena-allocated DefId for stable references | STATE.md already lists this as approved; avoids string interning complexity; DefId is copy |
| insta | 1.x | Snapshot tests for resolved IR | Already in writ-compiler dev-deps |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| ariadne 0.6 | miette | miette requires `fancy` feature flag and adds heavier proc-macro dependencies; ariadne is lighter and already in the project |
| ariadne 0.6 | codespan-reporting | codespan is simpler but less visually rich; ariadne matches the "Rust-style" quality bar |
| strsim | hand-rolled Levenshtein | strsim is well-tested, covers edge cases (Unicode), and adds no build overhead |
| walkdir | std::fs::read_dir recursion | walkdir handles symlinks and depth correctly; std requires manual recursion with error handling |
| toml 0.9 | toml_edit | toml_edit preserves formatting for round-trip edits; overkill for read-only config parsing |

**Installation:**
```bash
# In writ-diagnostics/Cargo.toml
ariadne = "0.6"
thiserror = "2.0"

# In writ-compiler/Cargo.toml (additions)
toml = "0.9"
serde = { version = "1", features = ["derive"] }
walkdir = "2"
strsim = "0.11"
rustc-hash = "2.1.1"
id-arena = "2.3.0"
writ-diagnostics = { path = "../writ-diagnostics" }

# In root Cargo.toml workspace members
# Add: "writ-diagnostics"
```

---

## Architecture Patterns

### Recommended Module Structure

```
writ-diagnostics/src/
├── lib.rs            # pub re-exports: Diagnostic, DiagnosticKind, emit()
├── diagnostic.rs     # Diagnostic struct: kind, code, span, labels, notes
├── render.rs         # ariadne integration: render Diagnostic → terminal
└── code.rs           # Error/warning code constants (E0001, W0001, etc.)

writ-compiler/src/
├── ast/              # existing (unchanged)
├── lower/            # existing (unchanged)
├── resolve/
│   ├── mod.rs        # pub fn resolve(ast: Vec<Ast>, config: &WritConfig) -> (NameResolvedAst, Vec<Diagnostic>)
│   ├── def_map.rs    # DefMap, DefEntry, DefId, NamespaceMap — Pass 1 output
│   ├── collector.rs  # Pass 1: walk Ast items, populate DefMap
│   ├── resolver.rs   # Pass 2: resolve all references, produce NameResolvedAst
│   ├── scope.rs      # ScopeChain: prelude + namespace imports + locals stack
│   ├── prelude.rs    # PRELUDE_TYPES const array, prelude lookup function
│   ├── error.rs      # ResolutionError variants → Diagnostic conversions
│   └── ir.rs         # NameResolvedAst, ResolvedDecl, ResolvedType, etc.
└── config.rs         # WritConfig: parsed writ.toml (toml + serde)
```

### Pattern 1: Two-Pass Symbol Collection

**What:** Pass 1 walks all `Ast.items` across all files and inserts every top-level declaration into `DefMap<String, DefEntry>` keyed by fully-qualified name. Pass 2 resolves references.

**When to use:** Always. Forward references between top-level declarations (e.g., struct A has field of type B where B is declared later) require all symbols to exist before any body is resolved.

**Example:**
```rust
// Source: rust-analyzer DefMap pattern (adapted for Writ)

pub struct DefMap {
    /// Key: "namespace::TypeName" or "TypeName" for root
    entries: FxHashMap<String, DefEntry>,
    /// Per-file private declarations (key: file_id)
    file_private: FxHashMap<FileId, FxHashMap<String, DefEntry>>,
    /// Namespace → list of pub def keys (for using resolution)
    namespace_pub: FxHashMap<String, Vec<String>>,
}

pub struct DefEntry {
    pub id: DefId,
    pub kind: DefKind,
    pub vis: DefVis,
    pub file_id: FileId,
    pub namespace: String,        // "" = root
    pub name_span: SimpleSpan,    // for error reporting
    pub generics: Vec<String>,    // generic param names
}

pub enum DefKind {
    Fn, Struct, Entity, Enum, Contract, Impl, Component, Extern, Const, Global,
}

pub enum DefVis {
    Pub,
    Private,   // file-local
}
```

### Pattern 2: ScopeChain for Pass 2 Resolution

**What:** A stack of scope layers. Lookup walks from innermost to outermost. Layers: prelude (bottom), root namespace, active `using` imports, current namespace, generic params, function params, local lets.

**When to use:** Pass 2 body resolution. Each scope push/pop corresponds to entering/leaving a block or declaration.

**Example:**
```rust
pub struct ScopeChain<'def> {
    def_map: &'def DefMap,
    /// Stack of layers, innermost last
    layers: Vec<ScopeLayer>,
    /// The current self type (Some when inside impl/entity method)
    self_type: Option<DefId>,
    /// Current namespace for unqualified resolution
    current_ns: String,
    /// Active using imports (alias → namespace)
    active_using: Vec<UsingEntry>,
}

pub enum ScopeLayer {
    GenericParams(Vec<(String, DefId)>),
    FnParams(Vec<(String, ResolvedType)>),
    LocalBlock(Vec<(String, ResolvedType, SimpleSpan)>),
}

impl ScopeChain<'_> {
    pub fn resolve_type(&self, name: &str) -> Result<ResolvedType, ResolutionError> {
        // 1. Check prelude (primitives + writ-runtime types)
        // 2. Check generic params (innermost first)
        // 3. Check current namespace (pub + private-if-same-file)
        // 4. Check using imports (detect ambiguity)
        // 5. Fail with fuzzy suggestions
    }
}
```

### Pattern 3: Prelude as a Static Layer

**What:** The prelude (primitives + writ-runtime types + 17 contracts) is a static `const` list that forms the bottom-most scope layer in every file. It is checked before user-defined names, but user-defined names that shadow prelude names trigger a hard error (not the usual shadowing warning).

**When to use:** Always inject. No user code should need to opt in.

**Example:**
```rust
// In prelude.rs
pub const PRELUDE_PRIMITIVE_NAMES: &[&str] = &[
    "int", "float", "bool", "string", "void",
];

pub const PRELUDE_TYPE_NAMES: &[&str] = &[
    "Option", "Result", "Range", "Array", "Entity",
];

pub const PRELUDE_CONTRACT_NAMES: &[&str] = &[
    "Add", "Sub", "Mul", "Div", "Mod", "Neg", "Not",
    "Eq", "Ord",
    "Index", "IndexSet",
    "BitAnd", "BitOr",
    "Iterable", "Iterator",
    "Into", "Error",
];

pub fn is_prelude_name(name: &str) -> bool {
    PRELUDE_PRIMITIVE_NAMES.contains(&name)
        || PRELUDE_TYPE_NAMES.contains(&name)
        || PRELUDE_CONTRACT_NAMES.contains(&name)
}

pub fn resolve_prelude(name: &str) -> Option<ResolvedType> {
    match name {
        "int"    => Some(ResolvedType::Primitive(PrimitiveTag::Int)),
        "float"  => Some(ResolvedType::Primitive(PrimitiveTag::Float)),
        "bool"   => Some(ResolvedType::Primitive(PrimitiveTag::Bool)),
        "string" => Some(ResolvedType::Primitive(PrimitiveTag::String)),
        "void"   => Some(ResolvedType::Primitive(PrimitiveTag::Void)),
        "Option" | "Result" | "Range" | "Array" | "Entity" => {
            // Look up in the virtual writ-runtime DefEntry
            Some(ResolvedType::Named(WRIT_RUNTIME_DEFS[name]))
        }
        _ => None,
    }
}
```

### Pattern 4: ariadne Diagnostic Rendering

**What:** `writ-diagnostics` owns the `Diagnostic` type. At render time, it builds an ariadne `Report` per diagnostic with labeled spans, notes, and help text. Multi-file diagnostics are handled by the ariadne `Cache` trait — pass a `FnCache` that maps FileId to source text.

**When to use:** All errors and warnings from all phases flow through this.

**Example:**
```rust
// Source: ariadne 0.6 docs
use ariadne::{Report, ReportKind, Label, Source, Color, Fmt};

fn render_diagnostic(diag: &Diagnostic, sources: &SourceMap) {
    let primary_span = diag.primary_span;
    let mut builder = Report::build(
        if diag.is_error { ReportKind::Error } else { ReportKind::Warning },
        primary_span.file_id,
        primary_span.start,
    )
    .with_code(diag.code)  // "E0042"
    .with_message(&diag.message);

    // Primary label (red for error, yellow for warning)
    builder = builder.with_label(
        Label::new((primary_span.file_id, primary_span.range()))
            .with_message(&diag.primary_label)
            .with_color(if diag.is_error { Color::Red } else { Color::Yellow })
    );

    // Secondary labels (all candidate spans for ambiguity errors)
    for label in &diag.secondary_labels {
        builder = builder.with_label(
            Label::new((label.span.file_id, label.span.range()))
                .with_message(&label.message)
                .with_color(Color::Blue)
        );
    }

    // help: note (always present)
    builder = builder.with_note(&diag.help_note);

    let report = builder.finish();
    // FnCache maps FileId → &str
    report.print(sources.as_ariadne_cache()).unwrap();
}
```

### Pattern 5: writ.toml Parsing

**What:** Parse `writ.toml` into a typed `WritConfig` struct at the start of the resolution phase. Use `toml 0.9` + `serde`.

**Example:**
```rust
// In writ-compiler/src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WritConfig {
    pub project: ProjectConfig,
    pub locale: LocaleConfig,
    #[serde(default)]
    pub compiler: CompilerConfig,
    #[serde(default)]
    pub conditions: FxHashMap<String, bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CompilerConfig {
    #[serde(default = "default_sources")]
    pub sources: Vec<String>,  // directories relative to writ.toml
    pub output: Option<String>,
}

fn default_sources() -> Vec<String> { vec!["src/".to_string()] }

pub fn load_config(toml_path: &Path) -> Result<WritConfig, ConfigError> {
    let text = std::fs::read_to_string(toml_path)?;
    toml::from_str(&text).map_err(ConfigError::Parse)
}
```

### Anti-Patterns to Avoid

- **Single-pass resolution:** Top-level names must all be collected before any body is resolved. If you try to resolve `struct A { b: B }` when `B` has not been collected yet, you will produce false "undefined type" errors for valid forward references.
- **Mutating the AST in place:** `Ast` is consumed by the resolver; the output is a new `NameResolvedAst` type. Do not add optional fields to `AstDecl` — the pipeline is strictly linear (STATE.md decision: "Pipeline IR is strictly linear: AST → NameResolved → Typed → Module; no in-place AST mutation").
- **Storing `&str` references into the DefMap:** The AST uses owned `String`; the DefMap should too. No lifetime parameters needed (matches the established pattern from lowering).
- **Stopping on first error:** The existing `LoweringContext` pattern accumulates all errors and never halts. The resolver must do the same. Use `Option<ResolvedType>` returns and emit diagnostics via context, not `Result` propagation.
- **Ambiguity errors at the `using` site:** The spec says ambiguity is detected at the usage site. Two `using` statements that import conflicting names are legal as long as neither name is used unqualified (spec §23.7). Do not error at the `using` declaration.
- **Re-exporting via using:** Spec §23.4 explicitly forbids re-export. A `using` in file A does not make those names visible to files that import A's namespace. Never propagate `using` scope across file boundaries.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fuzzy name suggestions | Custom Levenshtein | `strsim` (Jaro-Winkler) | Unicode edge cases, threshold tuning, already used by rustc |
| Diagnostic rendering with colors, multi-span | Custom terminal formatter | `ariadne 0.6` | Multi-file, multi-label, overlap avoidance, 8-bit colors — 500+ lines of complexity |
| TOML config parsing | `toml::Value` manual traversal | `toml` + `serde::Derive` | Serde gives type-safe structs with zero boilerplate |
| Recursive file discovery | `std::fs::read_dir` recursion | `walkdir` | Symlink handling, depth limits, proper error propagation |

**Key insight:** The rendering and fuzzy-matching are the two areas most likely to be hand-rolled badly. `ariadne` handles the span geometry problem (non-overlapping labels, multi-line alignment) which is non-trivial. `strsim` handles Unicode normalization and threshold selection correctly.

---

## Common Pitfalls

### Pitfall 1: Namespace Merging Across Files

**What goes wrong:** Treating each file as its own isolated namespace. Files `A.writ` and `B.writ` both declare `namespace survival;` and their pub declarations must be visible to each other without `::` qualification. A naive file-by-file approach produces false "undefined" errors.

**Why it happens:** The spec says namespaces can span multiple files (§23.2). The two-pass design explicitly addresses this — Pass 1 merges all files' pub declarations into a shared `DefMap` keyed by fully-qualified name.

**How to avoid:** In Pass 1, iterate ALL files before resolving ANY body. The `DefMap` is fully populated before Pass 2 begins.

**Warning signs:** Tests where `struct A` in file 1 and `struct B` in file 2 share a namespace but B's method cannot see A.

### Pitfall 2: Declarative vs. Block Namespace Context

**What goes wrong:** `namespace a::b;` (declarative) applies to ALL declarations in the file. `namespace a { namespace b { ... } }` (block) applies only to declarations inside the block. A file can have multiple block namespaces but at most one declarative namespace. Mixing up the scoping causes declarations to land in the wrong namespace.

**Why it happens:** The `AstNamespaceDecl` enum has two variants: `Declarative { path }` and `Block { path, items }`. The resolver needs different logic for each. Declarative sets the file-level namespace; block pushes/pops like the lowering context does.

**How to avoid:** Track a `file_namespace: Option<Vec<String>>` separately from a `current_namespace_stack: Vec<String>`. At Pass 1 entry to each file, if a declarative namespace is found, set `file_namespace` and use it as the prefix for all top-level decls. For block namespaces, push/pop the stack.

**Warning signs:** Declarations from declarative-namespace files appearing in the root namespace, or block-namespace items appearing in the wrong parent.

### Pitfall 3: Private Declarations and Same-Namespace Cross-File Access

**What goes wrong:** A private (no modifier) struct in `survival/potions.writ` is NOT visible from `survival/crafting.writ` — even though they share the `survival` namespace. Private means file-local, not namespace-local (spec §23.5, §23.6).

**Why it happens:** Conflating "same namespace" with "same visibility domain". The visibility rule is: private = file-local; pub = globally accessible.

**How to avoid:** DefEntry stores `file_id`. When resolving a name from a different file within the same namespace, check that `def.vis == Pub`. Only pub declarations are included in the namespace's cross-file symbol table. Private declarations go into `file_private[file_id]`.

**Warning signs:** A private struct in file A being resolved successfully from file B in the same namespace.

### Pitfall 4: `using` Scope Is Lexical, Not File-Wide

**What goes wrong:** A `using` inside a block namespace `namespace game { using survival; fn foo() { } }` should not be visible outside that block. If you store all usings in a flat list, you leak scope.

**Why it happens:** `using` scope mirrors the namespace block hierarchy (spec §23.4.3). File-level `using` is file-wide; block-level `using` is block-local.

**How to avoid:** The `ScopeChain` must push/pop `active_using` entries in sync with namespace block entry/exit, exactly as `LoweringContext` pushes/pops namespace segments.

**Warning signs:** A `using` inside an inner namespace block resolving names in an outer block.

### Pitfall 5: Prelude Names Are Errors, Not Shadows

**What goes wrong:** A user declares `struct Option { }`. Normally shadowing produces a warning. For prelude names, it must be a hard error.

**Why it happens:** Standard shadowing logic treats any name as shadowed. Prelude names are a special category.

**How to avoid:** In Pass 1, when inserting a declaration into DefMap, check `is_prelude_name(name)` first. If true, emit a "reserved prelude type" error and skip insertion. The prelude entry wins.

**Warning signs:** Defining `struct Option { }` compiles without error, causing downstream type confusion.

### Pitfall 6: Impl Block Target Resolution Order

**What goes wrong:** `impl ContractName for TypeName { }` — both `ContractName` and `TypeName` are `AstType` references that must be resolved. If the impl block appears before the type declaration in the AST, Pass 2 must still succeed.

**Why it happens:** Because Pass 1 already collected ALL top-level types before Pass 2 begins, both names are in DefMap regardless of AST order. The pitfall is if the resolver tries to resolve the impl target by walking AST items in order (sequential resolution) instead of using the fully-populated DefMap.

**How to avoid:** Pass 2 always looks up names in the already-complete DefMap. It never queries "what has been resolved so far." Order independence is a property of the two-pass design.

### Pitfall 7: Path vs. Enum Variant Resolution

**What goes wrong:** `QuestStatus::InProgress` looks like a qualified path. But `InProgress` is an enum variant, not a namespace member. The `::` operator in this case resolves the right side as a variant, not a sub-namespace lookup.

**Why it happens:** Paths and enum variant accesses share the same AST node (`AstExpr::Path` or `AstType::Named`/`Generic`). The resolver must check: is the left-hand segment a namespace? If yes, namespace lookup. Is it an enum type? If yes, variant lookup. These are mutually exclusive per the spec (§23.10: "namespaces and types occupy separate name spaces").

**How to avoid:** After resolving the first path segment, check `DefKind`. If `Enum`, do variant lookup. If `Namespace` (or resolves to a namespace), do sub-lookup in that namespace.

---

## Code Examples

Verified patterns from official sources and codebase inspection:

### ariadne Multi-File Report (ariadne 0.6)
```rust
// Source: ariadne docs https://docs.rs/ariadne/latest/ariadne/
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};

// For multi-file: implement Cache trait or use FnCache
// FnCache maps (file_id: FileId) -> &str
let mut sources = std::collections::HashMap::new();
sources.insert("file_a.writ", "namespace survival;\npub struct Item {}");
sources.insert("file_b.writ", "using survival;\nfn foo() { let x = Item(); }");

Report::build(ReportKind::Error, ("file_b.writ", 24..27))
    .with_code(42)
    .with_message("ambiguous name `Item`")
    .with_label(
        Label::new(("file_b.writ", 24..27))
            .with_message("`Item` could be `survival::Item` or `combat::Item`")
            .with_color(Color::Red),
    )
    .with_label(
        Label::new(("file_a.writ", 7..11))  // span of struct Item in file_a
            .with_message("candidate 1: defined here in `survival`")
            .with_color(Color::Blue),
    )
    .with_note("use a fully-qualified path to disambiguate")
    .finish()
    .print(ariadne::sources(sources))
    .unwrap();
```

### strsim "Did You Mean?" Pattern
```rust
// Source: strsim crate (https://crates.io/crates/strsim)
use strsim::jaro_winkler;

pub fn suggest_similar(
    unresolved: &str,
    candidates: impl Iterator<Item = String>,
) -> Vec<String> {
    const THRESHOLD: f64 = 0.8;
    const MAX_SUGGESTIONS: usize = 3;

    let mut scored: Vec<(f64, String)> = candidates
        .map(|c| (jaro_winkler(unresolved, &c), c))
        .filter(|(score, _)| *score >= THRESHOLD)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    scored.into_iter().take(MAX_SUGGESTIONS).map(|(_, name)| name).collect()
}
```

### DefMap Pass 1 Skeleton
```rust
// Recommended implementation pattern (adapted from rust-analyzer)
pub fn collect_declarations(
    asts: &[(FileId, Ast)],
) -> (DefMap, Vec<Diagnostic>) {
    let mut def_map = DefMap::default();
    let mut diags = Vec::new();

    for (file_id, ast) in asts {
        let mut ns_stack: Vec<String> = Vec::new();
        let file_ns = extract_declarative_namespace(ast); // Option<Vec<String>>

        if let Some(ref ns) = file_ns {
            ns_stack = ns.clone();
        }

        for decl in &ast.items {
            collect_decl(decl, *file_id, &mut ns_stack, &mut def_map, &mut diags);
        }
    }

    (def_map, diags)
}

fn collect_decl(
    decl: &AstDecl,
    file_id: FileId,
    ns_stack: &mut Vec<String>,
    def_map: &mut DefMap,
    diags: &mut Vec<Diagnostic>,
) {
    match decl {
        AstDecl::Namespace(AstNamespaceDecl::Block { path, items, .. }) => {
            let prev_len = ns_stack.len();
            ns_stack.extend(path.iter().cloned());
            for item in items {
                collect_decl(item, file_id, ns_stack, def_map, diags);
            }
            ns_stack.truncate(prev_len);
        }
        AstDecl::Struct(s) => {
            let fqn = make_fqn(ns_stack, &s.name);
            def_map.insert(fqn, DefEntry {
                kind: DefKind::Struct,
                vis: ast_vis_to_def_vis(&s.vis),
                file_id,
                namespace: ns_stack.join("::"),
                name_span: s.name_span,
                generics: s.generics.iter().map(|g| g.name.clone()).collect(),
                ..
            }, diags);
        }
        // ... other variants
        _ => {}
    }
}
```

### writ.toml serde Config
```rust
// In writ-compiler/src/config.rs
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct WritConfig {
    pub project: ProjectSection,
    pub locale: LocaleSection,
    #[serde(default)]
    pub compiler: CompilerSection,
    #[serde(default)]
    pub conditions: HashMap<String, bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CompilerSection {
    #[serde(default = "default_sources")]
    pub sources: Vec<String>,
    pub output: Option<String>,
}

fn default_sources() -> Vec<String> { vec!["src/".to_string()] }

pub fn load_config(project_root: &std::path::Path) -> Result<WritConfig, Box<dyn std::error::Error>> {
    let toml_path = project_root.join("writ.toml");
    let text = std::fs::read_to_string(&toml_path)?;
    Ok(toml::from_str(&text)?)
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| codespan-reporting (original standard) | ariadne (richer output) | 2021-2022 | Better overlap avoidance, multi-file labels, visual quality |
| Hand-rolled Levenshtein for suggestions | strsim (Jaro-Winkler) | 2019+ | Unicode correctness, established threshold, no maintenance |
| Monolithic single-pass resolver | Two-pass (collect then resolve) | Standard in rustc since early days | Eliminates forward-reference errors for top-level declarations |

**Deprecated/outdated:**
- `failure` crate: replaced by `thiserror` + `anyhow` — already using `thiserror`
- `codespan` (original, not codespan-reporting): unmaintained — use codespan-reporting or ariadne

---

## Open Questions

1. **Output IR structure: annotated AST vs. new IR type**
   - What we know: STATE.md says "Pipeline IR is strictly linear: AST → NameResolved → Typed → Module; no in-place AST mutation." CONTEXT.md marks this as Claude's discretion.
   - What's unclear: Whether `NameResolvedAst` should be a fully new type hierarchy (e.g., `ResolvedDecl` replacing `AstDecl`) or an annotated wrapper (e.g., `AstDecl` + a side-table of `HashMap<NodeId, ResolvedType>`).
   - Recommendation: New IR type (`ResolvedDecl` etc.) is cleaner and matches the "strictly linear" commitment. Side-table approach is faster to implement but couples phases. Suggest new type: `resolve/ir.rs` defines `NameResolvedAst`, `ResolvedDecl`, `ResolvedType`. The typed IR phase will then define its own `TypedAst`.

2. **Impl block orphan/cross-namespace rules**
   - What we know: CONTEXT.md marks impl block association as Claude's discretion for "orphan rules, cross-namespace impls."
   - What's unclear: Should an impl for `survival::Item` in `namespace combat` be allowed? The Writ spec doesn't explicitly address orphan rules.
   - Recommendation: For Phase 22, allow cross-namespace impls (don't check orphan rules). Writ is not Rust; orphan rules prevent library conflicts that Writ doesn't face at this stage. Orphan checking can be added as a future restriction.

3. **Generic bounds checking in resolution vs. type checking**
   - What we know: `AstGenericParam.bounds: Vec<AstType>` — the bounds are type names that must be resolved. CONTEXT.md marks this as Claude's discretion.
   - What's unclear: Should `T: Add + Eq` resolve `Add` and `Eq` to DefEntries in Phase 22, or just store them as unresolved strings for the type checker?
   - Recommendation: Resolve bound names to DefEntries in Phase 22 (they are just type name lookups). Verifying that the bound is actually a contract (not a struct) can also be done in Phase 22 since DefKind is available. Verifying that the bound is *satisfied* (concrete types) is deferred to type checking.

4. **Component-type validation in `entity use` clauses**
   - What we know: `AstComponentSlot.component: String` — the component name needs to be resolved to a DefEntry. CONTEXT.md marks this as Claude's discretion.
   - What's unclear: Phase boundary — resolve the name (Phase 22) vs. validate it's actually a component (Phase 22 via DefKind check) vs. validate field types (Phase 23).
   - Recommendation: Phase 22 resolves the component name to a DefEntry AND validates DefKind == Component (since DefKind is available during resolution). Field type compatibility is Phase 23.

5. **Pending todo from STATE.md: multi-file compilation mechanism**
   - What we know: STATE.md explicitly calls out "Decide multi-file compilation mechanism before Phase 22 implementation begins (merge-before-resolve vs. per-file resolve with cross-file DefMap joining)."
   - Resolution: Use merge-before-resolve. Call the parser on every discovered `.writ` file, run `lower()` on each, then collect ALL resulting `Ast` values into Pass 1 together. This is simpler and the two-pass design already makes it correct.

---

## Sources

### Primary (HIGH confidence)
- Writ language spec `language-spec/spec/24_23_modules_namespaces.md` — namespace model, `using` scoping, visibility rules
- Writ language spec `language-spec/spec/22_21_scoping_rules.md` — lexical scoping, shadowing rules
- Writ language spec `language-spec/spec/17_16_attributes.md` — [Singleton], [Conditional] rules
- Writ language spec `language-spec/spec/03_2_project_configuration_writ_toml.md` — writ.toml format
- Writ language spec `language-spec/spec/47_2_18_writ_runtime_module_contents.md` — prelude types, contracts
- Writ language spec `language-spec/spec/44_2_15_il_type_system.md` — TypeRef encoding (resolution output format)
- Codebase: `writ-compiler/src/ast/` — all AstDecl, AstType, AstExpr, AstStmt types (direct inspection)
- Codebase: `writ-compiler/src/lower/context.rs` — LoweringContext pattern (resolution context follows same pattern)
- Codebase: `writ-compiler/src/lower/error.rs` — error accumulation pattern
- [ariadne 0.6 docs](https://docs.rs/ariadne/latest/ariadne/) — rendering API, Cache trait, Label/Report types
- `.planning/STATE.md` — architectural decisions: linear IR, two-pass, id-arena + rustc-hash approved

### Secondary (MEDIUM confidence)
- [rust-analyzer DefMap](https://rust-lang.github.io/rust-analyzer/hir_def/nameres/struct.DefMap.html) — DefMap pattern for name resolution, verified against rust-analyzer source
- [strsim crate](https://crates.io/crates/strsim) — Jaro-Winkler/Levenshtein for fuzzy suggestions
- [toml 0.9 + serde](https://docs.rs/toml) — TOML parsing pattern

### Tertiary (LOW confidence)
- WebSearch: "ariadne 0.6 multiline spans" — confirmed version 0.6 is current; specific 0.6 changelog not inspected directly

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — libraries verified against crates.io docs and existing project usage (ariadne in writ-parser dev-deps; thiserror already used; id-arena and rustc-hash in STATE.md)
- Architecture: HIGH — two-pass DefMap is the canonical pattern (rustc, rust-analyzer); patterns derived from existing writ-compiler code
- Pitfalls: HIGH — derived from careful spec reading (§23.x) and cross-checking with AST types; not speculation

**Research date:** 2026-03-02
**Valid until:** 2026-06-01 (stable libraries; ariadne 0.6 unlikely to have breaking changes in 90 days)
