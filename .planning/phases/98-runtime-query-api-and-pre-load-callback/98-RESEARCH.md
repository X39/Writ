# Phase 98: Runtime Query API and Pre-Load Callback - Research

**Researched:** 2026-03-27
**Domain:** Rust API design — writ-runtime attribute inspection, pre-load hook, domain query methods
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — auto-generated infrastructure phase (discuss skipped).

Key design notes locked by earlier STATE.md decisions:
- `ModuleAttributeView` (not `&Domain`) must be the pre-load callback argument from day one — retrofitting is a breaking change
- No attribute causes automatic instantiation or invocation — hosts must explicitly act on query results

### Claude's Discretion
All implementation choices are at Claude's discretion. Use the ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| QAPI-01 | Runtime exposes `query_attributes(name)` returning all matching declarations with decoded arguments | Domain iterates module's `attribute_defs` table, filters by name string, decodes blob via `decode_attr_args` |
| QAPI-02 | Runtime exposes `query_attributes_on(type_id)` returning all attributes on a specific type | Filter `attribute_defs` where `owner` token resolves to the given TypeDef index and `owner_kind != ATTR_OWNER_KIND_DECL` |
| QAPI-03 | Runtime exposes `query_attribute_value(def, name)` returning decoded args for a specific attribute on a specific definition | Lookup single `AttributeDefRow` by metadata token + name, decode blob, return `Vec<AttrValue>` |
| QAPI-04 | Runtime provides a pre-load callback that fires before any module code executes, giving the host full attribute inspection | New `on_module_load` method on `RuntimeHost` trait; fires in `RuntimeBuilder::build` before `domain.add_module` for user module |
| QAPI-05 | Pre-load callback returns allow/reject decision; rejected modules are not loaded | `on_module_load` returns `Result<(), String>` (or similar); `Err` propagates as `RuntimeError::LoadError` and halts builder |
| QAPI-06 | No attribute causes automatic instantiation or invocation — host must explicitly act on query results | `ModuleAttributeView` is read-only; no side effects; query methods return owned data only |
</phase_requirements>

---

## Summary

Phase 98 adds two distinct capabilities to the writ-runtime: (1) a pre-load callback on `RuntimeHost` that fires after binary parsing but before `Domain::add_module`, allowing the host to inspect all attribute metadata and reject the module; and (2) a set of `Domain` query methods that let the host inspect attribute data on an already-loaded module at any time.

The existing infrastructure from Phase 93 provides everything needed: `AttributeDefRow` in `writ-module/src/tables.rs`, `decode_attr_args` / `AttrValue` in `writ-module/src/attr.rs`, and the `Module` struct with its `attribute_defs: Vec<AttributeDefRow>`. The runtime side needs two new items: `ModuleAttributeView` (a lightweight wrapper around `&writ_module::Module` that exposes query methods before Domain loading), and `on_module_load` on `RuntimeHost`.

The owner_kind encoding is already stable: `0=type`, `1=method`, `2=field/global`, `3=declaration (ATTR_OWNER_KIND_DECL)`. Applications (0/1/2) have an `owner` token pointing to the TypeDef/MethodDef/etc. row. Declarations (3) have `owner = MetadataToken::NULL`. This distinction is central to filtering in QAPI-02.

**Primary recommendation:** Implement `ModuleAttributeView` in `writ-runtime/src/host.rs` (or a new `writ-runtime/src/attr_view.rs`), add `on_module_load` to `RuntimeHost` with a default no-op, and add `Domain::query_attributes`, `Domain::query_attributes_on`, `Domain::query_attribute_value` methods.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `writ-module` (local) | workspace | `AttributeDefRow`, `AttrValue`, `decode_attr_args`, `MetadataToken` | All attribute data structures already defined here in Phase 93 |
| `writ-runtime` (local) | workspace | `RuntimeHost`, `Domain`, `RuntimeBuilder` | Existing host/domain/builder infrastructure to extend |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `writ-module::attr::decode_attr_args` | — | Decode blob heap bytes to `Vec<AttrValue>` | Every query path that returns argument values |
| `writ-module::heap::read_string` | — | Resolve `u32` string heap offset to `&str` | Reading `name` field of `AttributeDefRow` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New `ModuleAttributeView` wrapper | Pass `&writ_module::Module` directly | Direct module reference leaks the module format into the host trait; `ModuleAttributeView` can evolve independently and is safer to expose as public API |
| `on_module_load` on `RuntimeHost` trait | Separate callback type / builder method | Keeping it on `RuntimeHost` is consistent with all other host callbacks (`on_request`, `on_log`, `on_gc_complete`) and requires no new trait objects |

---

## Architecture Patterns

### Recommended Project Structure

New code touches:
```
writ-runtime/src/
├── host.rs          # Add ModuleAttributeView struct + on_module_load to RuntimeHost
├── runtime.rs       # Fire on_module_load in RuntimeBuilder::build before domain.add_module
└── domain.rs        # Add query_attributes, query_attributes_on, query_attribute_value
```

Optionally extract `ModuleAttributeView` to:
```
writ-runtime/src/
└── attr_view.rs     # ModuleAttributeView + AttributeMatch + AttrQueryResult
```
Either location is acceptable; the existing pattern of large files (`host.rs` is already the host-related file) favors keeping `ModuleAttributeView` in `host.rs`.

### Pattern 1: ModuleAttributeView

`ModuleAttributeView` is a read-only wrapper around a parsed (but not yet loaded) module. It borrows the `writ_module::Module` and the module's name for display purposes.

```rust
// In writ-runtime/src/host.rs

/// A read-only view of a parsed module's attribute metadata.
///
/// Passed to `RuntimeHost::on_module_load` before the module is added to the
/// Domain. The host may inspect attributes and return `Err` to reject the load.
pub struct ModuleAttributeView<'a> {
    module: &'a writ_module::Module,
}

impl<'a> ModuleAttributeView<'a> {
    pub(crate) fn new(module: &'a writ_module::Module) -> Self {
        Self { module }
    }

    /// The module name as declared in its header.
    pub fn module_name(&self) -> &str {
        writ_module::heap::read_string(&self.module.string_heap, self.module.header.module_name)
            .unwrap_or("<unknown>")
    }

    /// Return all AttributeDef application rows whose name matches `attr_name`.
    /// Excludes declaration rows (owner_kind == ATTR_OWNER_KIND_DECL).
    pub fn query_attributes(&self, attr_name: &str) -> Vec<AttributeMatch> {
        // iterate module.attribute_defs, filter, decode blob
    }

    /// Return all AttributeDef application rows whose owner token resolves to
    /// the given typedef_idx (0-based) in this module.
    pub fn query_attributes_on(&self, typedef_idx: usize) -> Vec<AttributeMatch> {
        // filter by owner token table == TypeDef and row_index == typedef_idx + 1
    }
}

/// A single matched attribute application with its decoded argument values.
#[derive(Debug, Clone)]
pub struct AttributeMatch {
    /// The attribute name (e.g. "Quest").
    pub name: String,
    /// The decoded argument values from the blob heap.
    pub args: Vec<writ_module::attr::AttrValue>,
    /// The owner token (points to the TypeDef/MethodDef/etc. row).
    pub owner: writ_module::token::MetadataToken,
    /// The owner_kind discriminant (0=type, 1=method, 2=field/global).
    pub owner_kind: u8,
}
```

### Pattern 2: RuntimeHost::on_module_load

Add a new default method to `RuntimeHost`. Default is `Ok(())` so existing host implementations do not need changes.

```rust
// In RuntimeHost trait (writ-runtime/src/host.rs)

/// Called after a module binary is parsed but BEFORE it is added to the Domain.
///
/// The host receives a read-only `ModuleAttributeView` of the module's attribute
/// table. Return `Ok(())` to allow loading, or `Err(reason)` to reject the module.
/// A rejected module causes `RuntimeBuilder::build` to return `RuntimeError::LoadError`.
///
/// No code from this module executes before this callback fires.
fn on_module_load(&mut self, _view: &ModuleAttributeView<'_>) -> Result<(), String> {
    Ok(())
}
```

### Pattern 3: Fire the hook in RuntimeBuilder::build

The hook fires after `Module::from_bytes` (or direct `Module` construction) produces a parsed module, before `domain.add_module`.

```rust
// In RuntimeBuilder::build (writ-runtime/src/runtime.rs)
// ...
// Add user module last — fire pre-load hook first
let view = ModuleAttributeView::new(&self.module);
if let Err(reason) = self.host.on_module_load(&view) {
    return Err(RuntimeError::LoadError(format!(
        "module rejected by host: {}", reason
    )));
}
let user_idx = domain.add_module(self.module)?;
```

The virtual module and library modules do NOT fire the hook (they are trusted infrastructure). Only the user-supplied module fires the hook.

### Pattern 4: Domain query methods

```rust
// In Domain (writ-runtime/src/domain.rs)

/// Return all attribute applications matching `attr_name` across all user modules.
/// Excludes declaration rows (owner_kind == ATTR_OWNER_KIND_DECL).
pub fn query_attributes(&self, attr_name: &str) -> Vec<DomainAttributeMatch> { ... }

/// Return all attribute applications on the given typedef_idx in the given module.
pub fn query_attributes_on(&self, module_idx: usize, typedef_idx: usize) -> Vec<DomainAttributeMatch> { ... }

/// Return the decoded argument values for the attribute named `attr_name` applied
/// to the definition identified by `owner_token` in the module at `module_idx`.
/// Returns `None` if no matching row is found.
pub fn query_attribute_value(
    &self,
    module_idx: usize,
    owner_token: writ_module::token::MetadataToken,
    attr_name: &str,
) -> Option<Vec<writ_module::attr::AttrValue>> { ... }
```

`DomainAttributeMatch` is a companion to `AttributeMatch` that also carries `module_idx`:

```rust
#[derive(Debug, Clone)]
pub struct DomainAttributeMatch {
    pub module_idx: usize,
    pub name: String,
    pub args: Vec<writ_module::attr::AttrValue>,
    pub owner: writ_module::token::MetadataToken,
    pub owner_kind: u8,
}
```

Alternatively, `query_attributes` and `query_attributes_on` can share `AttributeMatch` with an added `module_idx` field, and `ModuleAttributeView` can return the same type. Use whatever minimizes duplication.

### Anti-Patterns to Avoid

- **Filtering by owner_kind without excluding ATTR_OWNER_KIND_DECL:** Declaration rows (owner_kind==3, owner==NULL) must always be excluded from query results — they are schema rows, not application rows.
- **Exposing `&Domain` in the pre-load callback:** `Domain` is not yet populated at callback time; `ModuleAttributeView` must hold only `&writ_module::Module`.
- **Panicking on blob decode failure:** `decode_attr_args` returns `Result`. Query methods must handle `Err` gracefully (skip the row, or surface as empty args with a logged warning) — do not unwrap.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Blob decoding | Custom byte parser | `writ_module::attr::decode_attr_args` | Already handles all 4 tag types correctly with proper error handling |
| String heap reads | Direct byte slicing | `writ_module::heap::read_string` | Handles length-prefixed encoding and UTF-8 validation |
| MetadataToken arithmetic | Bit manipulation at call site | `MetadataToken::table_id()` / `MetadataToken::row_index()` | Encapsulates the 24-bit layout; `row_index()` is 1-based, convert to 0-based with `- 1` |

**Key insight:** Phase 93 already built the full encode/decode stack. Phase 98 is purely a consumption layer.

---

## Common Pitfalls

### Pitfall 1: Off-by-one on MetadataToken row index
**What goes wrong:** `token.row_index()` returns a 1-based index. Using it directly as a 0-based Vec index panics or reads the wrong row.
**Why it happens:** The token spec (section 2.16.4) stores 1-based indices; `Vec` is 0-based.
**How to avoid:** Always convert: `let idx_0based = token.row_index()? as usize - 1;`. Look at how `domain.rs` does it in `resolve_module_refs` — it uses `(scope_row - 1) as usize`.
**Warning signs:** Panic with "index out of bounds" or reading one row past the intended target.

### Pitfall 2: Including declaration rows in query results
**What goes wrong:** `query_attributes("Quest")` returns both the `Quest` attribute declaration row (owner_kind=3) and actual `[Quest(...)]` application rows.
**Why it happens:** Both rows have `name == "Quest"`.
**How to avoid:** Add `row.owner_kind != ATTR_OWNER_KIND_DECL` (i.e., `!= 3`) as a filter guard in every query method.
**Warning signs:** Query results include rows with `owner == MetadataToken::NULL`.

### Pitfall 3: Breaking existing RuntimeHost implementors
**What goes wrong:** Adding a required (non-default) method to `RuntimeHost` breaks all existing `impl RuntimeHost` blocks — NullHost, ExternHost, LSP backend, DAP server, golden tests, integration tests.
**Why it happens:** Rust traits: adding a required method is a breaking change for all implementors.
**How to avoid:** `on_module_load` MUST have a default body (`Ok(())`). The existing pattern in `host.rs` uses default methods for `on_gc_complete`, `debug_enabled`, `before_instruction`, etc. — follow the same pattern.
**Warning signs:** Compile errors in NullHost, ExternHost, or test host implementations.

### Pitfall 4: Hook fires for virtual module and library modules
**What goes wrong:** `on_module_load` fires for the writ-runtime virtual module and library modules, causing unexpected behavior in host rejection logic.
**Why it happens:** If the hook is wired at the generic `add_module` level rather than at the user-module level in the builder.
**How to avoid:** Fire the hook only for the user-provided module in `RuntimeBuilder::build`, after the virtual module and libraries have been added silently. Check the `build()` code in `runtime.rs` — the user module is added last (lines 122-128); the hook fires just before that call.
**Warning signs:** Test that rejects based on module name fails because the virtual module is named "writ-runtime" and gets rejected first.

### Pitfall 5: decode_attr_args failure silently drops data
**What goes wrong:** A module with a malformed blob causes a query to return empty results with no indication of the problem.
**Why it happens:** Calling `.unwrap_or_default()` on `decode_attr_args` silently swallows decode errors.
**How to avoid:** Log a warning via the runtime's log infrastructure when decode fails, or propagate the error to the caller. At minimum, return `Vec::new()` rather than panicking, but document the behavior.
**Warning signs:** Missing attributes in query results that are present in the compiled module.

---

## Code Examples

Verified patterns from existing codebase:

### Reading AttributeDefRow name from string heap
```rust
// Pattern from writ-runtime/src/domain.rs resolve_module_refs
use writ_module::heap::read_string;

let name = read_string(&module.string_heap, row.name)
    .unwrap_or("<unknown>");
```

### Decoding attribute arguments from blob heap
```rust
// writ-module/src/attr.rs - decode_attr_args
use writ_module::attr::decode_attr_args;

let blob_start = row.value as usize;
let args = if blob_start == 0 || blob_start >= module.blob_heap.len() {
    vec![]
} else {
    // blob heap is length-prefixed; read the length then the payload
    // Pattern: read u32 at offset, then payload bytes
    let len = u32::from_le_bytes([
        module.blob_heap[blob_start],
        module.blob_heap[blob_start + 1],
        module.blob_heap[blob_start + 2],
        module.blob_heap[blob_start + 3],
    ]) as usize;
    let payload = &module.blob_heap[blob_start + 4 .. blob_start + 4 + len];
    decode_attr_args(payload).unwrap_or_default()
};
```

Note: `writ-module/src/heap.rs` should be checked to confirm whether `blob_heap` uses the same length-prefix format as `string_heap`. Use `writ_module::heap::read_blob` if it exists, otherwise replicate the pattern used in `writ-module/src/reader.rs`.

### Checking owner token table and row
```rust
use writ_module::tables::TableId;
use writ_module::token::MetadataToken;

// Check if owner points to a TypeDef row
fn owner_is_typedef(owner: MetadataToken) -> Option<usize> {
    if owner.table_id() == TableId::TypeDef.as_u8() {
        owner.row_index().map(|r| r as usize - 1) // convert 1-based to 0-based
    } else {
        None
    }
}
```

### Filtering application rows (exclude declarations)
```rust
use writ_module::tables::ATTR_OWNER_KIND_DECL;

for row in &module.attribute_defs {
    if row.owner_kind == ATTR_OWNER_KIND_DECL {
        continue; // skip declaration rows
    }
    // row is an application row
}
```

### Adding attribute defs in a test module
```rust
// From writ-module/src/builder.rs - used in test helpers
use writ_module::{ModuleBuilder, attr::{encode_attr_args, AttrValue}};

let args = encode_attr_args(&[AttrValue::String("Chapter1".to_string())]);
let typedef_token = builder.add_type_def(/* ... */);
// owner_kind=0 means "type application"
builder.add_attribute_def(typedef_token, 0, "Quest", &args);
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No attribute inspection at runtime | `AttributeDefRow` table in module binary | Phase 93 | Host can read raw binary; Phase 98 adds ergonomic API |
| No pre-load hook | `on_module_load` callback | Phase 98 | Host can gate loading based on attribute requirements |

**Deprecated/outdated:**
- None applicable.

---

## Open Questions

1. **Blob heap read helper**
   - What we know: `writ_module::heap::read_string` exists; `writ_module::heap::init_blob_heap` exists
   - What's unclear: Whether there is a `read_blob(heap, offset) -> &[u8]` helper or whether blob reads are always done inline
   - Recommendation: Check `writ-module/src/heap.rs` at the start of Plan 98-01 implementation. If no helper exists, replicate the length-prefix read pattern from `reader.rs`.

2. **query_attribute_value signature — module_idx vs. type_id**
   - What we know: The success criteria says `domain.query_attribute_value(def_token, "level")` — the first argument is a `def_token`
   - What's unclear: Whether `def_token` is a `MetadataToken` (encoding table+row) or a plain `usize` typedef index; which module it lives in
   - Recommendation: Use `(module_idx: usize, owner_token: MetadataToken, attr_name: &str)` — this matches how `domain.rs` already resolves cross-module references and is unambiguous.

3. **query_attributes_on — does "type_id" mean typedef_idx or a MetadataToken?**
   - What we know: The success criterion says `domain.query_attributes_on(type_id)` with a `type_id`
   - What's unclear: Is `type_id` a raw `usize` index or a `MetadataToken`?
   - Recommendation: Accept `(module_idx: usize, typedef_idx: usize)` for clarity. The caller already knows which module they loaded.

---

## Environment Availability

Step 2.6: SKIPPED (no external dependencies identified — this is a pure Rust codebase change with no CLI tools, services, or external runtimes required beyond the existing Cargo workspace).

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + cargo test |
| Config file | `writ-runtime/Cargo.toml` (no separate test config) |
| Quick run command | `cargo test --package writ-runtime` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| QAPI-01 | `query_attributes("Quest")` returns all matching application rows with decoded args | unit | `cargo test --package writ-runtime query_attributes` | No — Wave 0 gap |
| QAPI-02 | `query_attributes_on(module_idx, typedef_idx)` returns all attributes on that type | unit | `cargo test --package writ-runtime query_attributes_on` | No — Wave 0 gap |
| QAPI-03 | `query_attribute_value(module_idx, owner_token, "level")` returns decoded args | unit | `cargo test --package writ-runtime query_attribute_value` | No — Wave 0 gap |
| QAPI-04 | `on_module_load` fires after binary parse, before Domain::add_module | integration | `cargo test --package writ-runtime on_module_load` | No — Wave 0 gap |
| QAPI-05 | Host returning `Err` from `on_module_load` causes `RuntimeBuilder::build` to fail | integration | `cargo test --package writ-runtime module_rejected` | No — Wave 0 gap |
| QAPI-06 | No attribute causes side effects; query returns data only | unit (implicit in QAPI-01/02/03 tests) | `cargo test --package writ-runtime` | No — Wave 0 gap |

### Sampling Rate
- **Per task commit:** `cargo test --package writ-runtime`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/attr_query_tests.rs` — covers QAPI-01, QAPI-02, QAPI-03, QAPI-04, QAPI-05, QAPI-06
- [ ] No new framework install needed; existing Rust test infrastructure covers all cases

*(All test gaps are in one new file; existing test infrastructure is sufficient)*

---

## Sources

### Primary (HIGH confidence)
- `writ-module/src/attr.rs` — `AttrValue`, `encode_attr_args`, `decode_attr_args`, tag constants — read directly
- `writ-module/src/tables.rs` — `AttributeDefRow`, `ATTR_OWNER_KIND_DECL`, `MetadataToken` — read directly
- `writ-runtime/src/host.rs` — `RuntimeHost` trait, `NullHost`, default method pattern — read directly
- `writ-runtime/src/runtime.rs` — `RuntimeBuilder::build`, module loading order (virtual → libraries → user) — read directly
- `writ-runtime/src/domain.rs` — `Domain::add_module`, cross-module resolution pattern — read directly
- `writ-module/src/builder.rs` — `add_attribute_def` API for building test modules — read directly
- `.planning/STATE.md` — locked decision: `ModuleAttributeView` vs `&Domain`; owner_kind encoding — read directly

### Secondary (MEDIUM confidence)
- `writ-compiler/src/emit/collect/encoding.rs` — confirms owner_kind 0/1/2 mappings for type/method/field applications — read directly

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already exist in the workspace; confirmed by reading source
- Architecture: HIGH — `RuntimeHost` trait pattern and `Domain` struct pattern are established; `ModuleAttributeView` design follows from locked constraints
- Pitfalls: HIGH — all pitfalls are derived from reading existing code (off-by-one in MetadataToken, default method requirement, declaration row filtering)
- Open questions: LOW risk — blob heap helper and type_id vs MetadataToken are easily resolved at plan time

**Research date:** 2026-03-27
**Valid until:** 90 days (Rust codebase, no external dependencies)
