# Phase 44: Extended Log with Levels - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace the `log(msg)` root-namespace inbuilt call with a leveled `log::` namespace providing `log::trace(msg)`, `log::debug(msg)`, `log::info(msg)`, `log::warn(msg)`, and `log::error(msg)`. The old `log(msg)` one-argument form is removed. All logging routes through `RuntimeHost::on_log(level, message)`. Golden tests and spec examples are updated to the new API.

</domain>

<decisions>
## Implementation Decisions

### Namespace mechanism
- `log` is a **compiler-known namespace**, not a value or instance — `log::debug(msg)` uses the standard `::` path separator
- `log::trace`, `log::debug`, `log::info`, `log::warn`, `log::error` are resolved as two-segment inbuilt paths
- `log` alone is not a valid expression — natural resolution failure error (no special error message needed)
- Root-qualified `::log::debug(msg)` also works, consistent with `::say`, `::choice`
- Standard shadowing applies — `let log = 5;` shadows the namespace at that scope, user error

### Log levels
- All 5 levels exposed: trace, debug, info, warn, error — matches existing `LogLevel` enum in `writ-runtime`
- Single argument only: `log::level(msg: string)` — no optional category parameter (deferred per TOOL-05)

### log(msg) removal
- `log(msg)` no longer compiles — natural resolution failure since `log` is now a namespace, not a function
- No special migration error message — standard "cannot call namespace" or "undefined function" behavior
- User `extern fn log(msg: string);` declarations shadow the namespace via standard shadowing — old code with explicit extern declarations still compiles but uses the extern, not the leveled API

### Golden test / fixture updates
- All golden tests and fixtures using `log(msg)` are updated to `log::info(msg)` — demonstrates the new API
- `extern fn log(msg: string);` declarations removed from golden tests
- Parser test cases and spec examples updated to use `log::info(msg)` or appropriate level

### CliHost output format
- UPPERCASE level prefix: `[TRACE]`, `[DEBUG]`, `[INFO]`, `[WARN]`, `[ERROR]`
- Log output to stderr (keep current behavior) — program output (say/choice) remains on stdout

### Claude's Discretion
- IL routing: whether log::level calls use CALL_EXTERN with level-specific extern names, a dedicated instruction, or direct `on_log` dispatch
- How the compiler recognizes the `log` namespace — new `DefKind::LogNamespace`, special-cased in check_call, or virtual module registration
- Whether `log` goes in prelude, sub-prelude, or a new namespace layer
- Exact spec section numbering for the updated §26.4

</decisions>

<specifics>
## Specific Ideas

- REQUIREMENTS.md uses informal `log.debug(msg)` notation — the actual Writ syntax is `log::debug(msg)` with the standard path separator
- STATE.md had an earlier decision about "additive alongside log(msg)" — this was superseded by REQUIREMENTS.md which explicitly removes the unleveled form
- The existing `LogLevel` enum already has all 5 variants (Trace, Debug, Info, Warn, Error) — no runtime changes needed for the enum itself

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-runtime/src/host.rs`: `LogLevel` enum with Trace/Debug/Info/Warn/Error — already defined, no changes needed
- `RuntimeHost::on_log(level, message)` — already the target interface for all log calls
- `writ-cli/src/cli_host.rs:155`: `on_log` implementation — needs format change from `[{level:?}]` to `[UPPERCASE]`
- `writ-compiler/src/resolve/prelude.rs`: Pattern for prelude/sub-prelude names — `log` namespace can follow same pattern
- `writ-compiler/src/check/check_expr.rs:470`: Existing `::log` prefix stripping — extends to `::log::debug` etc.

### Established Patterns
- Inbuilt call handling: `log`/`say`/`choice` are resolved as root-namespace extern functions via `extern fn` declarations or special DefMap entries
- Two-segment paths: `Option::None`, `Option::Some`, `Result::Ok`, `Result::Err` — `log::debug` follows the same pattern
- Sub-prelude injection (Phase 43): `None`/`Some` injected below user definitions — `log` namespace could use similar priority

### Integration Points
- Resolver (`resolve/`): Must recognize `log` as a namespace and `log::debug` etc. as callable inbuilt functions
- Type checker (`check/`): Must type-check `log::level(msg)` — verify msg is `string`, return `void`
- Emitter (`emit/`): Must emit appropriate IL to route to `RuntimeHost::on_log` with correct level
- Spec (`language-spec/spec/27_26_standard_library_builtins.md`): §26.4 table needs updating
- Golden tests: `fn_log_say_choice.writ`, `hello.writ` — update calls
- Parser test cases: ~15 `.writ` files reference `log(msg)` — update to `log::info(msg)`

</code_context>

<deferred>
## Deferred Ideas

- `log::debug(msg, category)` — optional category string for engine-side routing (TOOL-05, future phase)
- Log level filtering at compile time or runtime — not in scope

</deferred>

---

*Phase: 44-extended-log-with-levels*
*Context gathered: 2026-03-06*
