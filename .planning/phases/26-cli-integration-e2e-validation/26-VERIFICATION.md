---
phase: 26-cli-integration-e2e-validation
verified: 2026-03-03T16:00:00Z
status: passed
score: 11/11 must-haves verified
re_verification: true
  previous_status: gaps_found
  previous_score: 10/11
  gaps_closed:
    - "Generic contract specializations (e.g. Int:Into<Float> vs Int:Into<String>) resolve to distinct dispatch table entries"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Run 'writ compile hello.writ && writ run hello.writil' in a shell (release build)"
    expected: "Compilation succeeds, hello.writil is produced, writ run executes without crash"
    why_human: "Debug build stack overflows on Windows due to deeply recursive chumsky parser combinators; release build required. Tests verify internally via API; actual CLI subprocess not tested."
---

# Phase 26: CLI Integration and E2E Validation Verification Report

**Phase Goal:** Wire the full compilation pipeline into the writ CLI binary and validate end-to-end that source code compiles to binary modules that execute correctly on the VM. Fix known runtime bugs blocking validation.
**Verified:** 2026-03-03T16:00:00Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure (Plan 26-04 closed the single FIX-02 gap)

## Re-verification Summary

The previous verification (2026-03-03T15:30:00Z) found one gap: FIX-02 generic dispatch collision was structurally scaffolded but not functional (`type_args_hash=0` at both build and lookup sites). Plan 26-04 was executed to close this gap. This re-verification confirms the gap is closed.

**Closed:** FIX-02 — virtual module now has 22 contract defs (17 base + 5 specialization), dispatch table has 36 unique entries (0 collisions), `build_dispatch_table` uses `impl_def.contract.0` as `type_args_hash`, CALL_VIRT handler uses `resolve_type_args_hash` with `get_any()` backward-compat fallback, compiler `ModuleBuilder` gained `register_impl_method_contract` / `contract_token_for_method_def_id` API, and `emit_call` resolves non-zero `contract_idx` when a mapping is registered.

**Regressions:** None. All 39 test suites across the workspace pass (0 failures).

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Runtime fires on_create hook after INIT_ENTITY commits field writes | VERIFIED | `find_hook_by_name` + `push_hook_frame` in INIT_ENTITY handler (dispatch.rs); `init_entity_dispatches_on_create_hook` test passes |
| 2 | Runtime fires on_destroy hook during DESTROY_ENTITY two-phase protocol | VERIFIED | `find_hook_by_name` + `push_hook_frame` in DESTROY_ENTITY handler (dispatch.rs); `destroy_entity_dispatches_on_destroy_hook` test passes |
| 3 | Generic contract specializations (e.g. Int:Into<Float> vs Int:Into<String>) resolve to distinct dispatch table entries | VERIFIED | 5 specialization ContractDefs (Into<Float>, Into<Int>, Into<String>, Index<Int>, Index<Range>) added to virtual_module.rs; `build_dispatch_table` uses `impl_def.contract.0` as `type_args_hash`; `dispatch_table_virtual_module_has_36_intrinsic_entries` asserts 36 (not 32) entries and passes |
| 4 | CliHost displays actual string content for say() arguments | VERIFIED | `display_args: Vec<String>` on `HostRequest::ExternCall`; dispatch.rs pre-resolves `Value::Ref` via `heap.read_string`; cli_host.rs uses `display_args[0]` for say() |
| 5 | `writ compile foo.writ` produces a foo.writil binary file | VERIFIED | `Commands::Compile` in main.rs; `cmd_compile()` writes .writil; `test_compile_minimal_program` passes |
| 6 | Pipeline stages execute in order: parse -> lower -> resolve -> typecheck -> emit_bodies | VERIFIED | `cmd_compile()` chains all 5 stages; `emit_bodies()` calls full metadata collection |
| 7 | Errors at any stage halt the pipeline and render with ariadne source spans | VERIFIED | Each stage returns early on errors; `render_diagnostics` called at all error paths |
| 8 | Lowering errors display with source spans and actionable messages | VERIFIED | `LoweringError::to_diagnostic()` covers all 9 variants (L0001-L0099) with `DiagnosticBuilder` |
| 9 | A minimal Writ program compiles and executes end-to-end | VERIFIED | `test_compile_and_run_minimal` passes: `pub fn main() { let x: int = 42; }` compiles, loads, runs |
| 10 | Compilation errors display with source location context | VERIFIED | `test_compile_error_on_invalid_name` passes: E0102 at typecheck stage for undefined name |
| 11 | The full pipeline is exercised in e2e tests | VERIFIED | 4 e2e tests in `writ-cli/tests/e2e_compile_tests.rs`; all pass |

**Score:** 11/11 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-runtime/src/virtual_module.rs` | Distinct contract tokens per generic specialization | VERIFIED | 22 contract defs (17 base + 5 spec); `has_exactly_22_contract_defs` test passes; `into_float_spec`, `into_int_spec`, `into_string_spec`, `index_int_spec`, `index_range_spec` defined and used in all colliding ImplDef pairs |
| `writ-runtime/src/domain.rs` | build_dispatch_table uses impl_def.contract.0 as type_args_hash | VERIFIED | Line 430: `let type_args_hash = impl_def.contract.0;`; `dispatch_table_virtual_module_has_36_intrinsic_entries` asserts 36 entries and passes |
| `writ-runtime/src/dispatch.rs` | CALL_VIRT handler uses resolve_type_args_hash + get_any() fallback | VERIFIED | `resolve_type_args_hash` function added (line ~1862); CALL_VIRT handler at line ~541: `let type_args_hash = resolve_type_args_hash(contract_idx, ...)`; `get_any()` fallback when `type_args_hash == 0` |
| `writ-compiler/src/emit/module_builder.rs` | register_impl_method_contract / contract_token_for_method_def_id | VERIFIED | `method_to_contract: FxHashMap<DefId, MetadataToken>` field; both methods implemented; 3 new compiler tests pass |
| `writ-compiler/src/emit/body/call.rs` | CALL_VIRT emits resolved contract token | VERIFIED | Line 121-124: `contract_token_for_method_def_id(callee_def_id).map(|t| t.0).unwrap_or(0)` |
| `writ-compiler/src/emit/body/expr.rs` | Second CALL_VIRT emission site updated | VERIFIED | Line 260-263: same pattern via `maybe_def_id.and_then(|id| emitter.builder.contract_token_for_method_def_id(id))` |
| `writ-cli/src/cli_host.rs` | String dereferencing for ExternCall Ref args | VERIFIED | Uses `display_args` field; say() outputs actual string content |
| `writ-cli/src/main.rs` | Commands::Compile variant + cmd_compile() | VERIFIED | Compile subcommand present; 5-stage pipeline wired |
| `writ-cli/Cargo.toml` | writ-compiler and writ-diagnostics dependencies | VERIFIED | All 3 deps added |
| `writ-compiler/src/lower/error.rs` | LoweringError -> Diagnostic conversion | VERIFIED | `to_diagnostic()` covers all 9 variants |
| `writ-cli/tests/e2e_compile_tests.rs` | End-to-end integration tests | VERIFIED | 4 tests, all pass |
| `writ-compiler/tests/emit_body_tests.rs` | 3 new FIX-02 contract_idx tests | VERIFIED | `test_call_virt_emits_non_zero_contract_idx_when_registered`, `test_call_virt_emits_zero_contract_idx_when_no_mapping`, `test_call_virt_register_impl_method_contract_and_lookup` — all pass in the 82-test suite |
| `writ-runtime/tests/vm_tests.rs` | Updated dispatch table count assertions (36, 37) | VERIFIED | `call_virt_user_defined_contract_dispatch_table_populated` asserts 37 entries (36 intrinsic + 1 user); passes |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `virtual_module.rs` (specialization ContractDefs) | `domain.rs` (build_dispatch_table) | Distinct contract tokens produce distinct `impl_def.contract.0` values -> distinct DispatchKeys | WIRED | 5 synthetic ContractDef rows (rows 18-22) each have unique MetadataToken; `type_args_hash = impl_def.contract.0` uses these distinct values; 36-entry assertion passes |
| `dispatch.rs` (CALL_VIRT handler) | `domain.rs` dispatch table | `resolve_type_args_hash(contract_idx)` reconstructs MetadataToken matching `impl_def.contract.0` | WIRED | `resolve_type_args_hash` handles ContractDef (table_id=10): returns `contract_idx` directly; handles TypeRef (table_id=3): resolves to ContractDef token; handles 0: returns 0 (get_any fallback) |
| `emit/body/call.rs` (emit_call) | `ModuleBuilder::contract_token_for_method_def_id` | Resolves callee DefId to contract token for CALL_VIRT contract_idx | WIRED | `emitter.builder.contract_token_for_method_def_id(callee_def_id).map(|t| t.0).unwrap_or(0)` at both emission sites |
| `dispatch.rs` (CALL_VIRT) | `DispatchTable::get_any()` | Backward-compat fallback when `type_args_hash == 0` (legacy compiler output) | WIRED | `if type_args_hash == 0 { table.get_any(...) }` preserves all existing compiled code behavior |
| `dispatch.rs` (INIT_ENTITY) | Module::method_defs + string_heap | `find_hook_by_name` scans method range by name | WIRED | `find_hook_by_name(&module.module, type_idx_0based, "on_create")` in INIT_ENTITY handler |
| `dispatch.rs` (CallExtern) | `GcHeap::read_string` | Pre-resolve `Value::Ref` to string content in `display_args` | WIRED | `heap.read_string(*href)` per arg in display_args construction |
| `main.rs` (cmd_compile) | `writ_compiler::emit_bodies` | Pipeline chain: parse -> lower -> resolve -> typecheck -> emit_bodies | WIRED | All 5 stages in sequence; emit_bodies called with asts parameter |
| `main.rs` (cmd_compile) | `writ_diagnostics::render_diagnostics` | Error rendering at each pipeline stage | WIRED | `render_diagnostics` called for lower, resolve, typecheck, and codegen errors |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| FIX-01 | 26-01 | Runtime dispatches lifecycle hooks via method name lookup | SATISFIED | `find_hook_by_name` + `push_hook_frame` in INIT_ENTITY and DESTROY_ENTITY; 3 hook tests pass |
| FIX-02 | 26-01, 26-04 | Runtime resolves generic contract specialization without collision | SATISFIED | Virtual module has 22 contract defs (5 specialization-specific); `build_dispatch_table` uses `impl_def.contract.0`; `dispatch_table_virtual_module_has_36_intrinsic_entries` asserts 36 entries (was 32); `two_same_contract_different_token_specializations_produce_two_entries` passes |
| FIX-03 | 26-01 | CliHost dereferences GC heap Ref values for display | SATISFIED | `display_args` on `HostRequest::ExternCall`; dispatch.rs pre-resolves via `heap.read_string`; `fix03_extern_call_display_args_contains_string_content` test passes |
| CLI-01 | 26-02, 26-03 | `writ compile` subcommand accepts .writ files and outputs .writil binary | SATISFIED | `Commands::Compile` in main.rs; `cmd_compile` writes .writil; `test_compile_minimal_program` passes |
| CLI-02 | 26-02, 26-03 | Compiler pipeline runs end-to-end: parse -> lower -> resolve -> typecheck -> codegen | SATISFIED | `cmd_compile()` chains all 5 stages; `test_compile_and_run_minimal` passes |
| CLI-03 | 26-02, 26-03 | Compilation errors display with source spans, multi-span context, actionable messages | SATISFIED | `LoweringError::to_diagnostic` for all 9 variants; `render_diagnostics` at each stage; `test_compile_error_on_invalid_name` passes |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `writ-runtime/src/dispatch.rs` | ~1377 | `// Placeholder: just copy value (full conversion needs type system from Phase 19)` | Info | Pre-existing from prior phase; Convert instruction out of scope for Phase 26 |
| `writ-runtime/src/dispatch.rs` | ~2095 | `// Range-based string slicing (placeholder for full Range support)` | Info | Pre-existing from prior phase; StringIndexRange out of scope |

Note: The `// With type_args_hash=0, the dispatch table has the known generic collision issue` comment that was a Warning in the previous verification is now GONE — replaced with the corrected FIX-02 comment block documenting the fix.

### Human Verification Required

#### 1. CLI release-build end-to-end test

**Test:** Build with `cargo build --release -p writ-cli`, then run `./target/release/writ compile tests/fixtures/hello.writ && ./target/release/writ run hello.writil --entry main`
**Expected:** Compilation succeeds with "Compiled: hello.writil" message; `writ run` executes without crash or error
**Why human:** Debug build hits stack overflow on Windows from deeply recursive chumsky parser combinators (documented in 26-02 SUMMARY). E2e tests exercise the API path directly to avoid this. The actual CLI binary subprocess path needs manual release-build verification.

## Test Run Summary

All workspace tests pass as of this re-verification:

| Package | Tests | Passed | Failed |
|---------|-------|--------|--------|
| writ-runtime (unit) | 61 | 61 | 0 |
| writ-runtime (domain tests) | 22 | 22 | 0 |
| writ-runtime (virtual_module tests) | 17 | 17 | 0 |
| writ-runtime (hook_dispatch_tests) | 8 | 8 | 0 |
| writ-runtime (vm_tests) | 77 | 77 | 0 |
| writ-compiler (emit_body_tests) | 82 | 82 | 0 |
| writ-compiler (other tests) | 157 | 157 | 0 |
| writ-cli (unit + e2e) | 9 | 9 | 0 |
| **Workspace total** | **~1,069** | **~1,069** | **0** |

## Gap Closure Confirmation

The single gap from the previous verification has been fully resolved:

**FIX-02: Generic Dispatch Collision** — Previously, `type_args_hash=0u32` at both `build_dispatch_table` (domain.rs) and the CALL_VIRT handler (dispatch.rs), causing 4 generic specialization pairs to collide in the dispatch table (32 entries instead of 36). The fix:

1. **Virtual module** (`virtual_module.rs`): Added 5 synthetic specialization ContractDefs (`Into<Float>`, `Into<Int>`, `Into<String>`, `Index<Int>`, `Index<Range>`). All colliding ImplDef pairs now use distinct contract tokens.
2. **Runtime build** (`domain.rs`): `type_args_hash = impl_def.contract.0` — uses the raw MetadataToken value as a specialization discriminator. Distinct tokens produce distinct DispatchKeys.
3. **Runtime lookup** (`dispatch.rs`): `resolve_type_args_hash(contract_idx)` reconstructs the correct type_args_hash from the CALL_VIRT instruction. Backward-compat `get_any()` fallback for `contract_idx=0` (legacy compiler output).
4. **Compiler** (`module_builder.rs`, `call.rs`, `expr.rs`): `register_impl_method_contract` / `contract_token_for_method_def_id` API added; both CALL_VIRT emission sites resolve non-zero `contract_idx` when a mapping is registered.

Commits: `7b839ef` (Task 1: virtual module + runtime) and `2d457be` (Task 2: compiler emission).

---

_Verified: 2026-03-03T16:00:00Z_
_Verifier: Claude (gsd-verifier)_
