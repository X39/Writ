# Phase 31: Test Harness and Functions Golden Test - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the compile-disassemble-compare golden test infrastructure and lock the first set of regression golden files covering function IL. This phase delivers the harness framework and a focused golden test suite for function declarations, parameters, return types, local variables, mutual calls, and recursion. All subsequent golden test phases (32–36) will add new golden files to the same harness.

</domain>

<decisions>
## Implementation Decisions

### Harness invocation style
- Rust API only — no CLI subprocess spawning
- The compile and disassemble steps share **no state**: compile produces bytes, bytes are deserialized via `Module::from_bytes()`, then disassemble runs on the freshly loaded module
- Round-trip: `compile_source(src) → Vec<u8> → Module::from_bytes() → disassemble()` — tests what is actually serialized, not what is in compiler memory
- Harness lives in a new `writ-golden` workspace crate
- Golden files (`.writ` source + `.expected` IL text) live in `writ-golden/tests/golden/`

### Bless / update workflow
- **Claude's Discretion**: Use `BLESS=1` env var pattern to overwrite `.expected` files with current output (`BLESS=1 cargo test`)
- On test failure, show a unified diff (`--- expected` / `+++ actual`) so developers see exactly which IL lines changed

### Disassembly verbosity
- **Claude's Discretion**: Use clean `writ_assembler::disassemble()` (no hex offset comments)
- Rationale: verbose hex offsets break on any instruction size change upstream; clean format only breaks when IL content changes, which is the signal we want

### Functions golden fixture scope
- Multiple focused golden files, one concern per file (not one monolithic functions.writ)
- Phase 31 scope: the function-IL-focused golden files listed below
- Pattern for future phases: each adds its own golden files to `writ-golden/tests/golden/` using the same harness

**Golden files for Phase 31:**
- `fn_basic_call.writ` — void-return function, a call from main, CALL + RET_VOID wiring
- `fn_typed_params.writ` — int and bool typed parameters, typed return values, multiple param functions
- `fn_recursion.writ` — self-recursive function (factorial-style), verifies call-stack correctness and self-call token

### Claude's Discretion
- Exact diff library choice (e.g., `similar` crate or manual line comparison)
- Whether `writ-golden` depends on `writ-compiler` directly or goes through `writ-cli`'s compile helper
- Cargo.toml placement and workspace member setup for `writ-golden`
- Test function naming convention within `writ-golden`

</decisions>

<specifics>
## Specific Ideas

- "All caches must have been invalidated" between compile and disassemble — the golden test must go through the serialized bytes (Module::from_bytes), not share an in-memory Module from the compiler
- Golden files should eventually cover all spec features (phases 32–36 will add more); Phase 31 is the harness + function IL anchor only
- Existing `hello.writ` fixture stays for CLI smoke tests; the new golden files in `writ-golden/tests/golden/` are separate

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ_assembler::disassemble(module: &Module) -> String`: clean disassembler, ready to use
- `writ_assembler::disassemble_verbose(module: &Module) -> String`: verbose variant (not used for golden files)
- `writ_module::Module::from_bytes(bytes: &[u8])`: deserialize bytes → Module; the isolation boundary between compile and disassemble
- `compile_source()` helper in `writ-cli/tests/e2e_compile_tests.rs`: exact pattern to replicate in writ-golden (runs full pipeline: parse → lower → resolve → typecheck → emit_bodies → bytes)

### Established Patterns
- `writ-parser/tests/cases/` — numbered `.writ` files as test fixtures; `writ-golden/tests/golden/` follows the same pattern
- End-to-end tests use Rust API directly (no subprocess); `writ-golden` continues this pattern
- Compile pipeline spawns a 16 MB stack thread (see `cmd_compile` in `writ-cli/src/main.rs`) due to deep AST recursion — `writ-golden` test helpers must do the same

### Integration Points
- `writ-golden` workspace member depends on: `writ-compiler`, `writ-assembler`, `writ-module`, `writ-diagnostics`, `writ-parser`
- `Cargo.toml` (workspace root) needs `"writ-golden"` added to `members`

</code_context>

<deferred>
## Deferred Ideas

- Covering all spec features in golden files — phases 32–36 per the roadmap (structs, enums, entities, dialogue, closures, concurrency, etc.)
- Verbose golden files with hex offsets for debugging — could be a per-test opt-in in the future

</deferred>

---

*Phase: 31-test-harness-and-functions-golden-test*
*Context gathered: 2026-03-04*
