# Phase 44: Extended Log with Levels - Research

**Researched:** 2026-03-06
**Domain:** Writ compiler — inbuilt namespace injection, IL ExternDef registration, CliHost dispatch
**Confidence:** HIGH

## Summary

Phase 44 replaces the old `log(msg)` root-namespace extern function with a leveled `log::` namespace providing five callsites: `log::trace(msg)`, `log::debug(msg)`, `log::info(msg)`, `log::warn(msg)`, `log::error(msg)`. The `LogLevel` enum and `RuntimeHost::on_log(level, message)` interface already exist in `writ-runtime` with all five variants — no runtime changes are required.

The implementation spans four compiler layers. The resolver must inject five synthetic ExternFn DefIds into the DefMap under FQNs `"log::trace"` through `"log::error"`. The type checker must recognize two-segment `log::level(msg)` call sites and route them through `check_call_with_sig` with the correct `callee_def_id`, so the emitter can emit `CALL_EXTERN` to the right ExternDef token. The emitter collects the five synthetic DefIds as ExternDef rows so that `token_for_def` maps each to an ExternDef MetadataToken. The CliHost receives `ExternCall` requests, matches the extern name against `"log::trace"` etc., and calls `on_log` with the appropriate level variant. Finally, `§26.4` of the spec is updated and all golden test fixtures are migrated from `log(msg)` to `log::info(msg)`.

The old `log(msg)` form disappears naturally: since `log` is no longer in the DefMap as a callable name (only `log::trace` etc. are), any bare `log(msg)` call will produce a natural resolution failure ("undefined function `log`").

**Primary recommendation:** Inject 5 synthetic ExternFn entries in the resolver (analogous to how `None`/`Some` are handled at sub-prelude priority), extend `check_call` to handle two-segment `log::level` paths with `callee_def_id` propagation, inject matching ExternDef rows in the emitter's collect pass, update CliHost to match names and route to `on_log`, then update all fixtures and the spec.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- `log` is a **compiler-known namespace**, not a value or instance — `log::debug(msg)` uses the standard `::` path separator
- `log::trace`, `log::debug`, `log::info`, `log::warn`, `log::error` are resolved as two-segment inbuilt paths
- `log` alone is not a valid expression — natural resolution failure error (no special error message needed)
- Root-qualified `::log::debug(msg)` also works, consistent with `::say`, `::choice`
- Standard shadowing applies — `let log = 5;` shadows the namespace at that scope, user error
- All 5 levels exposed: trace, debug, info, warn, error — matches existing `LogLevel` enum in `writ-runtime`
- Single argument only: `log::level(msg: string)` — no optional category parameter (deferred per TOOL-05)
- `log(msg)` no longer compiles — natural resolution failure since `log` is now a namespace, not a function
- No special migration error message — standard "cannot call namespace" or "undefined function" behavior
- User `extern fn log(msg: string);` declarations shadow the namespace via standard shadowing — old code with explicit extern declarations still compiles but uses the extern, not the leveled API
- All golden tests and fixtures using `log(msg)` are updated to `log::info(msg)` — demonstrates the new API
- `extern fn log(msg: string);` declarations removed from golden tests
- Parser test cases and spec examples updated to use `log::info(msg)` or appropriate level
- UPPERCASE level prefix: `[TRACE]`, `[DEBUG]`, `[INFO]`, `[WARN]`, `[ERROR]`
- Log output to stderr (keep current behavior) — program output (say/choice) remains on stdout

### Claude's Discretion

- IL routing: whether log::level calls use CALL_EXTERN with level-specific extern names, a dedicated instruction, or direct `on_log` dispatch
- How the compiler recognizes the `log` namespace — new `DefKind::LogNamespace`, special-cased in check_call, or virtual module registration
- Whether `log` goes in prelude, sub-prelude, or a new namespace layer
- Exact spec section numbering for the updated §26.4

### Deferred Ideas (OUT OF SCOPE)

- `log::debug(msg, category)` — optional category string for engine-side routing (TOOL-05, future phase)
- Log level filtering at compile time or runtime — not in scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TOOL-03 | User calls `log::trace/debug/info/warn/error(msg)` from Writ scripts — the unqualified `log(msg)` one-argument form is removed; all logging uses the leveled namespace | Synthetic ExternFn injection in resolver + check_call extension + ExternDef rows in emitter + CliHost name-match routing |
</phase_requirements>

---

## Standard Stack

### Core (all internal to Writ codebase)

| Layer | File | What changes | Why |
|-------|------|-------------|-----|
| Resolver | `writ-compiler/src/resolve/mod.rs` | Inject 5 synthetic ExternFn DefIds after Pass 1 | Makes `log::trace` etc. visible in DefMap FQN table |
| Resolver | `writ-compiler/src/resolve/prelude.rs` | Add `LOG_NAMESPACE_NAME = "log"` constant and level list | Documents the compiler-known names |
| Type checker | `writ-compiler/src/check/check_expr.rs` | Extend `check_call` for 2-segment log paths; extend `TypeEnv` injection | Ensures `callee_def_id` is set so emitter emits CALL_EXTERN |
| Type checker | `writ-compiler/src/check/env.rs` | Inject FnSig entries for the 5 synthetic DefIds | `ctx.type_env.fn_sigs.get(&def_id)` must succeed |
| Emitter | `writ-compiler/src/emit/collect.rs` | Inject ExternDef rows for the 5 synthetic DefIds before finalize | `token_for_def(def_id)` returns ExternDef MetadataToken |
| CliHost | `writ-cli/src/cli_host.rs` | Match `"log::trace"` etc. in `on_request`, call `on_log` with level | Delivers the message to the `RuntimeHost::on_log` interface |
| Spec | `language-spec/spec/27_26_standard_library_builtins.md` | Rewrite §26.4 with `log::` namespace description | Removes `log(msg)` from spec |
| Golden fixture | `writ-golden/tests/golden/fn_log_say_choice.writ` | Replace `extern fn log(...)` + `::log(msg)` with `::log::info(msg)` | Demonstrates new API; existing snapshot re-blessed |
| Parser tests | `writ-parser/tests/cases/*.writ` (~12 files) | Update `log(msg)` to `log::info(msg)` | Keeps examples consistent (parser tests only check parse, not semantics, but examples should be correct) |

### Supporting (no changes needed)

| Component | File | Current State |
|-----------|------|--------------|
| `LogLevel` enum | `writ-runtime/src/host.rs:10-16` | Already has all 5 variants: Trace, Debug, Info, Warn, Error |
| `RuntimeHost::on_log` | `writ-runtime/src/host.rs:98` | Already accepts `(level: LogLevel, message: &str)` |
| `NullHost::on_log` | `writ-runtime/src/host.rs:125` | Already silently drops — no change needed |
| IL instruction set | `writ-module/src/instruction.rs:107` | `CallExtern` already sufficient — no new instruction needed |
| Runtime dispatch | `writ-runtime/src/dispatch/calls.rs:118` | `exec_call_extern` routes to `on_request` — no change needed |

**Installation:** All changes are within the Writ workspace. No new crate dependencies.

---

## Architecture Patterns

### How the existing `log(msg)` worked (now removed)

The old pattern: users wrote `extern fn log(msg: string);` in their script. The compiler registered it as a real `DefKind::ExternFn` entry in the DefMap. The emitter added an `ExternDef` row named `"log"`. At runtime, `CALL_EXTERN` delivered `HostRequest::ExternCall { extern_idx: <log token>, ... }` to the host. `CliHost::on_request` matched the extern name `"log"` and printed `[extern] log()` (or similar). **`on_log` was never called for script-originated log messages** — it was only used for runtime internal errors.

### Recommended approach for `log::level` (Design A — virtual ExternFn injection)

Follow the existing `None`/`Some` sub-prelude injection pattern (Phase 43), extended to ExternFn rather than enum constructors:

**Step 1: Resolver injection** — After `collect_declarations` in `resolve::resolve()`, inject 5 synthetic DefEntries:

```rust
// In writ-compiler/src/resolve/mod.rs, after collect_declarations:
fn inject_log_namespace(def_map: &mut DefMap) {
    use chumsky::span::SimpleSpan;
    let synthetic_span = SimpleSpan::new(0, 0);
    for level_name in ["log::trace", "log::debug", "log::info", "log::warn", "log::error"] {
        let entry = DefEntry {
            id: None,
            kind: DefKind::ExternFn,
            vis: DefVis::Pub,
            file_id: FileId(u32::MAX), // synthetic sentinel
            namespace: "log".to_string(),
            name: level_name.split("::").nth(1).unwrap().to_string(),
            name_span: synthetic_span,
            generics: Vec::new(),
            span: synthetic_span,
        };
        // Insert with fqn = level_name (e.g. "log::debug")
        def_map.by_fqn.insert(level_name.to_string(), def_map.arena.alloc(entry));
    }
}
```

**Step 2: TypeEnv injection** — In `TypeEnv::build()`, after building the env from resolved decls, inject FnSig entries for the 5 log-level DefIds:

```rust
// Synthetic FnSig: (msg: string) -> void
// Injected for each log level DefId so ctx.type_env.fn_sigs.get(&def_id) succeeds.
for level in ["log::trace", "log::debug", "log::info", "log::warn", "log::error"] {
    if let Some(def_id) = resolved.def_map.get(level) {
        let sig = FnSig {
            name: level.to_string(),
            params: vec![("msg".to_string(), interner.string_ty())],
            ret: interner.void(),
            generics: vec![],
            self_param: None,
            bounds: vec![],
        };
        env.fn_sigs.insert(def_id, sig);
    }
}
```

**Step 3: check_call extension** — In `check_call`, add a fast-path for two-segment log paths (analogous to the existing single-segment `::log` fast-path at lines 786-804):

```rust
// In check_call, after the single-segment path fast-path:
// Special case: two-segment log namespace call: `log::debug(msg)` or `::log::debug(msg)`
if let AstExpr::Path { segments, span: path_span } = callee {
    if segments.len() == 2 {
        let first = segments[0].strip_prefix("::").unwrap_or(&segments[0]);
        let second = &segments[1];
        if first == "log" {
            let fqn = format!("log::{}", second);
            if let Some(def_id) = ctx.def_map.get(&fqn) {
                if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                    return check_call_with_sig(
                        ctx, &fqn, def_id, sig.clone(), args, span, *path_span,
                    );
                }
            }
        }
    }
}
```

**Step 4: Emitter ExternDef injection** — In `collect_defs()` (or a new `inject_log_extern_defs()` called from `emit_bodies`), before `builder.finalize()`:

```rust
// In emit/collect.rs or emit/mod.rs, inject log-level ExternDef rows:
fn inject_log_extern_defs(
    def_map: &DefMap,
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
) {
    let sig_blob = encode_void_string_sig(interner, builder); // (string) -> void
    for level in ["log::trace", "log::debug", "log::info", "log::warn", "log::error"] {
        if let Some(def_id) = def_map.get(level) {
            builder.add_extern_def(
                level,            // name (e.g. "log::debug")
                sig_blob.clone(),
                level,            // import_name = same as name
                1,                // flags: pub
                Some(def_id),     // DefId for token_for_def lookup
            );
        }
    }
}
```

**Step 5: CliHost dispatch** — In `on_request`, match the resolved extern name against log levels:

```rust
// In cli_host.rs, on_request ExternCall arm:
let name = self.resolve_extern_name(*extern_idx);
match name {
    "say" => { /* existing */ }
    "choice" => { /* existing */ }
    "log::trace" => {
        let msg = display_args.first().cloned().unwrap_or_default();
        self.on_log(LogLevel::Trace, &msg);
        HostResponse::Value(Value::Void)
    }
    "log::debug" => {
        let msg = display_args.first().cloned().unwrap_or_default();
        self.on_log(LogLevel::Debug, &msg);
        HostResponse::Value(Value::Void)
    }
    // ... info, warn, error ...
    other => { /* existing [extern] fallthrough */ }
}
```

**Step 6: CliHost format change** — Change `on_log` format from debug `{level:?}` to uppercase prefix:

```rust
fn on_log(&mut self, level: LogLevel, message: &str) {
    let prefix = match level {
        LogLevel::Trace => "TRACE",
        LogLevel::Debug => "DEBUG",
        LogLevel::Info  => "INFO",
        LogLevel::Warn  => "WARN",
        LogLevel::Error => "ERROR",
    };
    eprintln!("[{prefix}] {message}");
}
```

### Golden test update pattern

```
// fn_log_say_choice.writ — BEFORE:
pub extern fn log(msg: string);
pub extern fn say(text: string);
pub extern fn choice();

pub fn main() {
    ::log("saying Test");
    ::say("Test");
    ::log("showing choice");
    ::choice();
}

// fn_log_say_choice.writ — AFTER:
pub extern fn say(text: string);
pub extern fn choice();

pub fn main() {
    ::log::info("saying Test");
    ::say("Test");
    ::log::info("showing choice");
    ::choice();
}
```

The snapshot (`fn_log_say_choice.writil`) must be re-blessed after the fixture change. The extern token numbers will change (log::info gets a new ExternDef row index; say/choice shift if ordering changes).

### Recommended Project Structure for new code

No new files needed — all changes are in existing files:

```
writ-compiler/src/
  resolve/mod.rs          <- inject_log_namespace() call after Pass 1
  resolve/prelude.rs      <- LOG_NAMESPACE_LEVELS constant
  check/env.rs            <- inject log-level FnSigs after TypeEnv::build
  check/check_expr.rs     <- check_call two-segment log fast-path
  emit/collect.rs         <- inject_log_extern_defs() call in collect_defs
writ-cli/src/
  cli_host.rs             <- log::* name match in on_request; on_log format fix
language-spec/spec/
  27_26_standard_library_builtins.md  <- §26.4 rewrite
writ-golden/tests/golden/
  fn_log_say_choice.writ  <- fixture update
  fn_log_say_choice.writil <- re-bless
writ-parser/tests/cases/
  07_functions.writ, 09_entities.writ, 11_error_handling.writ,
  14_attributes.writ, 15_ranges_indexing.writ, 16_generics.writ,
  17_globals_atomic.writ, 18_extern.writ, 20_comprehensive.writ
  <- update log(msg) to log::info(msg) (parser tests only check parse)
writ-cli/tests/fixtures/
  hello.writ              <- update ::log(test_string) to ::log::info(test_string)
```

### Anti-Patterns to Avoid

- **New IL instruction for logging:** There is no `LOG_MESSAGE` instruction and adding one requires touching the binary format, serializer, deserializer, disassembler, and runtime. CALL_EXTERN is sufficient.
- **Routing through on_request as an anonymous extern:** If the ExternDef name is just `"log"` with a level parameter, the host has no way to know the level. Use level-specific names `"log::debug"` etc.
- **Adding `DefKind::LogNamespace`:** Not needed — ExternFn with a two-segment FQN is sufficient. Adding a new DefKind kind would require touching every match arm in the compiler.
- **Injecting log namespace into TypeEnv after the build function returns:** TypeEnv is immutable after construction; inject during or immediately after build.
- **Forgetting to update `hello.writ`:** `writ-cli/tests/fixtures/hello.writ` uses `::log(test_string)` and will cause CLI integration test failures.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Level-encoded log messages | Custom `LOG_MESSAGE` IL instruction | CALL_EXTERN with level-specific extern names; CliHost matches by name |
| Namespace visibility | New DefKind::LogNamespace + special handling everywhere | Synthetic ExternFn DefIds with two-segment FQNs in `by_fqn` |
| FnSig for synthetic entries | Parse a synthetic Writ source file | Directly construct `FnSig { params: [(string)], ret: void }` |

**Key insight:** The `None`/`Some` injection from Phase 43 is the closest analogue. The difference is that `None`/`Some` are enum constructors handled as special IR nodes in the check layer. For `log::level`, we need real ExternDef tokens so CALL_EXTERN carries the right `extern_idx`, which requires actual DefIds in the DefMap and ExternDef rows in the builder.

---

## Common Pitfalls

### Pitfall 1: Token ordering changes break the golden snapshot
**What goes wrong:** Adding ExternDef rows for log levels before `say`/`choice` (or in between) shifts their token numbers, breaking `fn_log_say_choice.writil`.
**Why it happens:** ExternDef tokens are assigned 1-based in insertion order during `builder.finalize()`.
**How to avoid:** Inject log-level ExternDef rows after all user-declared externs, or re-bless the snapshot unconditionally after the fixture change. BLESS=1 re-blesses correctly.
**Warning signs:** Golden test fails with mismatched extern_idx values in CALL_EXTERN instructions.

### Pitfall 2: `callee_def_id` is None for two-segment log paths
**What goes wrong:** The emitter emits `CALL` instead of `CALL_EXTERN` for `log::debug(msg)` because `callee_def_id` is `None`.
**Why it happens:** `check_call` currently only has a fast-path for single-segment paths (lines 786-804). Two-segment paths fall through to the general `check_expr(callee)` → `TypedExpr::Path`, which has no `callee_def_id` field.
**How to avoid:** The two-segment log fast-path in `check_call` must call `check_call_with_sig` with `def_id` from `ctx.def_map.get("log::debug")` — same as the single-segment case.
**Warning signs:** Golden snapshot shows `CALL r0, 0, r1, 1` instead of `CALL_EXTERN r0, <token>, r1, 1`.

### Pitfall 3: `::log::debug(msg)` (root-qualified two-segment) not handled
**What goes wrong:** `::log::debug(msg)` parses as `Path { segments: ["::log", "debug"] }` (leading `::` prepended to first segment only). The two-segment fast-path must strip `::` from the first segment before comparison.
**Why it happens:** `lower_expr` in `lower/expr.rs:81` prepends `::` to `segs[0]` only when `rooted = true`.
**How to avoid:** In the two-segment fast-path: `let first = segments[0].strip_prefix("::").unwrap_or(&segments[0])` before comparing to `"log"`.
**Warning signs:** `::log::info("msg")` compiles fine but `log::info("msg")` does not (or vice versa).

### Pitfall 4: CliHost `on_log` called with wrong level due to partial name match
**What goes wrong:** A future extern named `"log::warn_critical"` accidentally matches the `"log::warn"` prefix.
**Why it happens:** `match` on exact string literals avoids this, but `starts_with` would not.
**How to avoid:** Use exact `match name { "log::trace" => ..., "log::debug" => ... }` — not prefix matching.

### Pitfall 5: Parser test cases break after log() update
**What goes wrong:** `writ-parser/tests/cases/18_extern.writ` declares `extern fn log(msg: string);`. If the parser test compiles (beyond parsing), this `extern fn log` will shadow the log namespace — the test may behave differently than expected.
**Why it happens:** Parser tests test parsing only (they don't run the full compiler pipeline), so the extern declaration is fine there. But examples should use the new API.
**How to avoid:** Update all parser test cases to use `log::info(msg)` and remove `extern fn log(...)` declarations. Verify parser tests still parse correctly (parser does not care about semantics).

### Pitfall 6: `hello.writ` not updated causes CLI integration test failure
**What goes wrong:** `writ-cli/tests/fixtures/hello.writ` uses `::log(test_string)` — this will fail to compile once `log` is no longer a callable function.
**Why it happens:** `hello.writ` doesn't have `extern fn log(msg: string)` — it relies on the old inbuilt `log`.
**How to avoid:** Update `hello.writ` to `::log::info(test_string)` in the same pass as golden fixture updates.

### Pitfall 7: ExternDef sig blob encoding for synthetic entries
**What goes wrong:** `encode_fn_sig_from_ast_sig` requires an `AstFnSig` from the AST — no AST exists for synthetic entries.
**Why it happens:** `collect_extern_fn` in `emit/collect.rs` uses `find_extern_fn_sig(asts, entry)` to find the AST node.
**How to avoid:** For synthetic log-level ExternDef rows, encode the sig blob directly: signature for `(string) -> void` is a known byte pattern. Or add a helper `encode_string_to_void_sig(interner, builder)` that constructs the TypeRef blob without going through AST.

---

## Code Examples

### Existing: how `None`/`Some` are injected (Phase 43 pattern)

```rust
// writ-compiler/src/check/check_expr.rs — check_ident, lines 441-455
// Sub-prelude builtin variant constructors.
match name {
    "None" | "Some" => {
        let infer_var = ctx.unify.new_var();
        let infer_ty = ctx.interner.intern(TyKind::Infer(infer_var));
        let opt_ty = ctx.interner.option(infer_ty);
        return TypedExpr::Var { ty: opt_ty, span, name: name.to_string() };
    }
    _ => {}
}
```

The log namespace uses ExternFn DefIds instead of this pure check-layer pattern, because CALL_EXTERN requires a real ExternDef token index.

### Existing: single-segment root-qualified path fast-path

```rust
// writ-compiler/src/check/check_expr.rs — check_call, lines 786-804
// Special case: callee is a root-qualified single-segment Path (e.g. `::log`).
if let AstExpr::Path { segments, span: path_span } = callee {
    if segments.len() == 1 {
        let raw = &segments[0];
        let normalized = raw.strip_prefix("::").unwrap_or(raw.as_str());
        if let Some(def_id) = find_fn_def_id(ctx, normalized) {
            if let Some(sig) = ctx.type_env.fn_sigs.get(&def_id) {
                return check_call_with_sig(ctx, normalized, def_id,
                    sig.clone(), args, span, *path_span);
            }
        }
    }
}
```

The two-segment log fast-path follows the same structure: strip `::`, match `first == "log"`, join FQN, look up by `ctx.def_map.get(&fqn)`.

### Existing: ExternDef token detection in analyze_callee

```rust
// writ-compiler/src/emit/body/call.rs — analyze_callee, lines 265-271
// Check if the DefId maps to an ExternDef token
if let Some(token) = emitter.builder.token_for_def(callee_def_id) {
    use crate::emit::metadata::TableId;
    if token.table() == TableId::ExternDef {
        return CallKind::Extern;
    }
}
```

This is why the synthetic DefIds must be registered with `builder.add_extern_def(..., Some(def_id))` — so `token_for_def(def_id)` returns an ExternDef token.

### Existing: ExternDef insertion in builder

```rust
// writ-compiler/src/emit/module_builder.rs — add_extern_def, lines 416-434
pub fn add_extern_def(
    &mut self,
    name: &str,
    sig_blob: Vec<u8>,
    import_name: &str,
    flags: u16,
    def_id: Option<DefId>,
) -> usize {
    let name_offset = self.string_heap.intern(name);
    let import_name_offset = self.string_heap.intern(import_name);
    let sig_offset = self.blob_heap.intern(&sig_blob);
    self.extern_defs.push(ExternDefRow {
        name: name_offset,
        signature: sig_offset,
        import_name: import_name_offset,
        flags,
    });
    self.extern_def_def_ids.push(def_id);
    self.extern_defs.len() - 1
}
```

Call this once per log level with `def_id = Some(log_level_def_id)`.

### Existing: CliHost `on_log` current format (to update)

```rust
// writ-cli/src/cli_host.rs — line 155-157
fn on_log(&mut self, level: LogLevel, message: &str) {
    eprintln!("[{level:?}] {message}");  // outputs "[Debug] msg" -- needs changing
}
// NEW:
fn on_log(&mut self, level: LogLevel, message: &str) {
    let prefix = match level {
        LogLevel::Trace => "TRACE",
        LogLevel::Debug => "DEBUG",
        LogLevel::Info  => "INFO",
        LogLevel::Warn  => "WARN",
        LogLevel::Error => "ERROR",
    };
    eprintln!("[{prefix}] {message}");
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| `log(msg)` as root-namespace extern declared by user | `log::trace/debug/info/warn/error(msg)` compiler-known namespace | Phase 44 | Removes user declaration burden; adds severity levels |
| `on_log` only called for runtime internal errors | `on_log` called for all script log output | Phase 44 | Host now receives structured level info |
| `[{level:?}]` debug format (e.g. `[Debug]`) | `[DEBUG]` UPPERCASE format | Phase 44 | Consistent with standard log tooling conventions |

**Deprecated/outdated:**
- `extern fn log(msg: string)` in user scripts: removed (except old code with explicit extern decl, which still works via shadowing but is no longer the standard pattern)
- `::log("msg")` single-segment path: natural failure once `log` is no longer a callable in DefMap

---

## Open Questions

1. **Sig blob encoding for synthetic ExternDef entries**
   - What we know: `collect_extern_fn` uses `find_extern_fn_sig(asts, entry)` to find the AST node, then calls `encode_fn_sig_from_ast_sig`. No AST exists for synthetic entries.
   - What's unclear: Whether there's a simpler helper to encode `(string) -> void` directly.
   - Recommendation: Write a small helper `encode_string_void_sig(interner: &TyInterner, builder: &ModuleBuilder) -> Vec<u8>` that constructs the TypeRef blob for `(string) -> void` directly. Look at `type_sig.rs` for the encoding format. Alternatively, create a minimal synthetic AST `AstFnSig` struct — but direct encoding is cleaner.

2. **DefMap arena mutability after injection**
   - What we know: `DefMap::insert` takes `&mut self` and a mutable diags vec. But to inject after Pass 1, we need `def_map` before it's moved into `NameResolvedAst`.
   - What's unclear: Whether injecting in `resolve::resolve()` between Pass 1 and Pass 2 is safe (Pass 2 resolves body references against DefMap — it won't find `log::debug` in any body, so this is fine).
   - Recommendation: Inject in `resolve::resolve()` immediately after `collect_declarations` returns, before `resolve_bodies` is called. Use `def_map.by_fqn.insert(fqn, id)` directly (bypass the normal duplicate check) for synthetic entries.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test + cargo test |
| Config file | `Cargo.toml` (workspace) |
| Quick run command | `cargo test -p writ-compiler -- log 2>&1` |
| Full suite command | `cargo test --workspace 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TOOL-03 | `log::info("msg")` compiles without error | unit | `cargo test -p writ-compiler -- typecheck 2>&1` | ✅ `writ-compiler/tests/typecheck_tests.rs` |
| TOOL-03 | `log::trace/debug/warn/error("msg")` compile without error | unit | `cargo test -p writ-compiler -- typecheck 2>&1` | ❌ Wave 0: add test cases |
| TOOL-03 | `log("msg")` (unqualified) no longer compiles | unit | `cargo test -p writ-compiler -- typecheck 2>&1` | ❌ Wave 0: add negative test |
| TOOL-03 | `log::info("msg")` emits `CALL_EXTERN` (not `CALL`) | unit | `cargo test -p writ-golden -- fn_log_say_choice 2>&1` | ✅ `writ-golden/tests/golden_tests.rs` (after re-bless) |
| TOOL-03 | `::log::debug(msg)` root-qualified form compiles | unit | `cargo test -p writ-compiler -- typecheck 2>&1` | ❌ Wave 0: add test case |
| TOOL-03 | CliHost prints `[DEBUG] msg` format | unit | `cargo test -p writ-cli -- on_log 2>&1` | ❌ Wave 0: add unit test |
| TOOL-03 | `log` alone is not a valid callable | unit | `cargo test -p writ-compiler -- typecheck 2>&1` | ❌ Wave 0: add negative test |
| TOOL-03 | Golden snapshot matches new `log::info` IL | golden | `cargo test -p writ-golden -- fn_log_say_choice 2>&1` | ✅ (after re-bless) |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-compiler && cargo test -p writ-golden`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `writ-compiler/tests/typecheck_tests.rs` — add `test_log_namespace_compiles` covering all 5 levels
- [ ] `writ-compiler/tests/typecheck_tests.rs` — add `test_log_bare_fails` (negative: `log("msg")` without extern decl produces error)
- [ ] `writ-compiler/tests/typecheck_tests.rs` — add `test_log_root_qualified` (`::log::debug(msg)` succeeds)
- [ ] `writ-cli/src/cli_host.rs` (tests section) — add `test_on_log_debug_prefix` verifying `[DEBUG]` format

*(The golden test `fn_log_say_choice` already exists and will be re-blessed after fixture update — no new file needed there.)*

---

## Sources

### Primary (HIGH confidence)

- Direct source inspection: `writ-runtime/src/host.rs` — LogLevel enum, on_log interface
- Direct source inspection: `writ-cli/src/cli_host.rs` — on_log format, on_request ExternCall dispatch
- Direct source inspection: `writ-compiler/src/check/check_expr.rs` — check_call fast-paths, find_fn_def_id
- Direct source inspection: `writ-compiler/src/check/env.rs` — TypeEnv::build, fn_sigs population
- Direct source inspection: `writ-compiler/src/emit/collect.rs` — collect_extern_fn, ExternDef registration
- Direct source inspection: `writ-compiler/src/emit/body/call.rs` — analyze_callee, CALL_EXTERN detection
- Direct source inspection: `writ-compiler/src/emit/body/expr.rs` — callee_def_id propagation
- Direct source inspection: `writ-compiler/src/resolve/prelude.rs` — SUB_PRELUDE_VARIANT_NAMES pattern
- Direct source inspection: `.planning/phases/44-extended-log-with-levels/44-CONTEXT.md` — locked decisions

### Secondary (MEDIUM confidence)

- Phase 43 implementation pattern for sub-prelude injection (from `STATE.md` decision log)

### Tertiary (LOW confidence)

- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all files inspected directly
- Architecture: HIGH — full pipeline traced from parse through runtime
- Pitfalls: HIGH — derived from direct code inspection of call dispatch and token ordering

**Research date:** 2026-03-06
**Valid until:** Stable (internal codebase, no external dependencies)
