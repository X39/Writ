# Phase 110: Runtime Bug Fixes - Research

**Researched:** 2026-03-28
**Domain:** writ-runtime VM instruction handler (RT-02) + writ-compiler serializer (RT-03)
**Confidence:** HIGH

## Summary

Phase 110 fixes two runtime-layer bugs: a missing end-to-end test for `s.len()` correctness
(RT-02) and a serialization ordering defect that causes `Module::from_bytes` to fail with
`UnexpectedEof` when `::choice` lambdas appear in a multi-function module (RT-03).

For **RT-02**, code review of `exec_str_len` in `writ-runtime/src/dispatch/arith.rs` shows the
handler is structurally correct — it reads the string via `ctx.heap.read_string(href)` and
returns `s.len() as i64`. An existing unit test (`str_len_returns_length`) exercises the
`I2s → StrLen` path and passes. However, there is no end-to-end test that compiles `s.len()`
from Writ source through the full pipeline and verifies the integer result. The requirement
calls for a golden test or E2E test that proves the byte-length return value is correct.

For **RT-03**, a historical commit (`a7ea521`, "fix(quick-2-01): revise quest_system.writ to
avoid ::choice in multi-fn modules") confirmed the bug and its root cause: "closure extern-def
ordering in multi-body emit". The `dlg_fn_mix.writ` golden test still contains the comment
"avoids known ::choice serialization bug" and deliberately omits `::choice` usage. Simple
reproduction attempts run successfully, suggesting the bug is triggered by a combination of
multiple top-level functions PLUS choice lambdas with non-empty bodies, as in the original
`quest_system.writ` test case. The fix area is the orphaned-body/MethodDef matching logic in
`writ-compiler/src/emit/serialize.rs`.

**Primary recommendation:** For RT-02, add an E2E golden test that compiles `s.len()` from
source and checks the runtime return value. For RT-03, reproduce the original bug using the
`quest_system.writ` pattern, identify the exact MethodDef/body ordering failure in
`serialize.rs`, fix it, and add a regression golden test that exercises `::choice` in a
multi-function module.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP
phase goal, success criteria, and codebase conventions to guide decisions.

### Claude's Discretion
All implementation choices (test placement, fix approach, golden test names).

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RT-02 | `s.len()` returns byte length, not heap slot number (StrLen fix) | `exec_str_len` handler in arith.rs is correct; need E2E test via full compile+run pipeline |
| RT-03 | `::choice` with `fn() {}` lambda arguments serializes without UnexpectedEof error | Root cause in serialize.rs orphaned-body MethodDef matching; must reproduce with multi-fn module |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| writ-runtime | workspace | VM execution + dispatch | Bug site for RT-02 (exec_str_len) |
| writ-compiler | workspace | Codegen + serialization | Bug site for RT-03 (serialize.rs) |
| writ-module | workspace | Module binary format | Decode/encode involved in RT-03 round-trip |
| writ-golden | workspace | Snapshot regression tests | Test harness for E2E golden tests |
| writ-cli | workspace | compile + run pipeline | Used for E2E test helpers |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| writ-assembler | workspace | Disassembly verification | Verify IL output in golden tests |

**Installation:** All crates are in the same Cargo workspace — no installs needed.

## Architecture Patterns

### Recommended Project Structure

```
writ-runtime/tests/vm_tests.rs          -- per-instruction unit tests (RT-02 unit test here)
writ-cli/tests/e2e_compile_tests.rs     -- compile+run pipeline tests (RT-02 E2E here)
writ-golden/tests/golden/              -- golden .writ + .writil snapshots (RT-03 golden here)
writ-golden/tests/golden_tests.rs      -- test functions that call run_golden_test()
writ-compiler/src/emit/serialize.rs    -- RT-03 bug fix location
```

### Pattern 1: VM Unit Test (existing pattern)
**What:** Build a minimal module programmatically, spawn task, tick, assert return value.
**When to use:** Verifying that a specific instruction returns the correct Value.

```rust
// Source: writ-runtime/tests/vm_tests.rs (existing str_len_returns_length pattern)
#[test]
fn str_len_loadstring_returns_byte_length() {
    // LoadString allocates to heap, StrLen reads from heap
    // Add string "hello" to module string heap, load it, call StrLen
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("T", "", TypeDefKind::Struct, 0);
    let string_idx = writ_module::heap::intern_string(&mut builder.string_heap, "hello");
    let body = MethodBody {
        register_types: vec![0, 0],
        code: encode(&[
            Instruction::LoadString { r_dst: 0, string_idx },
            Instruction::StrLen { r_dst: 1, r_str: 0 },
            Instruction::Ret { r_src: 1 },
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();
    let mut rt = RuntimeBuilder::new(module).build().unwrap();
    let tid = rt.spawn_task(0, vec![]).unwrap();
    rt.tick(0.0, ExecutionLimit::None);
    assert_eq!(rt.return_value(tid), Some(Value::Int(5))); // "hello" has 5 bytes
}
```

### Pattern 2: E2E Compile+Run Test
**What:** Compile Writ source via `compile_source()`, deserialize, find method, spawn, tick, assert.
**When to use:** Verifying the full pipeline from source to runtime result.

```rust
// Source: writ-cli/tests/e2e_compile_tests.rs (existing compile_source pattern)
#[test]
fn test_str_len_returns_byte_length() {
    let src = r#"pub fn main() -> int { let s: string = "hello"; s.len() }"#;
    let bytes = compile_source(src).expect("should compile");
    let module = Module::from_bytes(&bytes).expect("should deserialize");
    // ... find main method, spawn, tick, check return value == 5
}
```

### Pattern 3: Golden Test for Choice (RT-03)
**What:** A `.writ` file with multiple functions + choice lambdas, a `.writil` snapshot.
**When to use:** Regression anchor for the choice serialization bug.

```
// File: writ-golden/tests/golden/fn_multi_choice.writ
fn helper() -> int { 42 }
entity Narrator {}
pub fn main() -> int {
    ::choice([ ::ChoiceOption("A", "a", fn() { ::log::info("chose A"); }) ]);
    helper()
}
```

### Anti-Patterns to Avoid
- **Assuming `StrLen` is broken without checking data path**: The handler is correct; the bug is that there's no E2E test proving it works via the compiler. Don't rewrite `exec_str_len` without first confirming the bug.
- **Assuming RT-03 is fixed**: The `dlg_fn_mix.writ` comment is current evidence the bug persists. Don't close RT-03 without a regression test that previously failed.
- **Using `ModuleBuilder` directly for RT-03 testing**: The serialization bug is triggered by the COMPILER's emit path, not `ModuleBuilder`. Use `compile_source()` or `RuntimeBuilder::from_source()`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Compile+run pipeline | Custom parse/lower/typecheck loop | `compile_source()` in e2e_compile_tests.rs | Already handles full pipeline, error propagation |
| Module round-trip | Raw byte manipulation | `Module::from_bytes` + `Module::to_bytes` | Already validated round-trip path |
| Golden test scaffolding | Custom file comparison | `run_golden_test(name)` + `BLESS=1` | Handles blessed snapshot comparison + diff output |
| String heap population | Manual byte writing | `writ_module::heap::intern_string()` | Correct format (u32 len prefix + UTF-8 bytes) |

**Key insight:** Both bugs are in well-isolated components. Don't over-engineer the fixes.

## Common Pitfalls

### Pitfall 1: Confusing ModuleBuilder String Heap with Runtime Heap
**What goes wrong:** `ModuleBuilder.string_heap` is the module's static string pool.
`BumpHeap` in the runtime is the dynamic heap for allocated objects. `LoadString` bridges them:
it reads from the module heap and allocates on the runtime heap as `Value::Ref(href)`.
`StrLen` operates on the runtime heap. If you accidentally pass `Value::Int(string_idx)` to
`StrLen`, `extract_ref` returns `HeapRef(u32::MAX)` and the handler crashes (not returns slot).
**Why it happens:** Confusing the two string storage layers.
**How to avoid:** Always trace `LoadString → Value::Ref` flow before assuming register contents.
**Warning signs:** `extract_ref` returning `HeapRef(u32::MAX)` → crash path in `exec_str_len`.

### Pitfall 2: Orphaned Body Ordering in serialize.rs
**What goes wrong:** `method_def_body_indices` is built by iterating
`builder.finalized_method_def_entries()`. Orphaned MethodDefs (def_id == None) are matched to
orphaned bodies by POSITION order. After `finalize()` sorts MethodDefs by parent TypeDefHandle,
the orphaned MethodDefs appear in a different order than the orphaned bodies if lambda MethodDefs
are added to TypeDefs that sort differently from the order bodies are emitted.
**Why it happens:** Reflectable `get_type()` bodies are emitted BEFORE lambda bodies (by design),
but finalize's sort may interleave reflectable and lambda MethodDefs differently in a module
with multiple TypeDefs and multiple lambdas.
**How to avoid:** The fix must ensure orphaned MethodDefs and orphaned bodies are enumerated in
EXACTLY the same order. The orphan matching loop in serialize.rs is the canonical fix point.
**Warning signs:** `Module::from_bytes` returning `UnexpectedEof` after `Module::to_bytes`
succeeds; method body count mismatch after round-trip.

### Pitfall 3: body_size Sentinel Confusion
**What goes wrong:** In `writ-module/src/writer.rs`, `method.body_size` is used as a boolean
sentinel ("has body") but in `serialize.rs` it's set to `code.len()` (not the full body size).
The writer recalculates the real size via `compute_body_size()`. This is intentional and correct
but confusing.
**Why it happens:** Two different meanings of `body_size` in the same field.
**How to avoid:** Don't change this sentinel pattern unless also fixing the reader's expectation.
**Warning signs:** body_size mismatch in writer condition; reader reading wrong byte ranges.

### Pitfall 4: RT-03 Requires Full Compiler Pipeline
**What goes wrong:** Testing the fix with `ModuleBuilder` directly doesn't exercise the
serialization bug because `ModuleBuilder` doesn't go through `pre_scan_lambdas` or
`emit_all_bodies`.
**Why it happens:** The root cause is in the emit/serialize pipeline, not in the module format.
**How to avoid:** Always test RT-03 fixes using `compile_source()` or `RuntimeBuilder::from_source()`.

## Code Examples

### exec_str_len — Current Implementation
```rust
// Source: writ-runtime/src/dispatch/arith.rs:451
pub(super) fn exec_str_len(ctx: &mut ExecContext<'_>, r_dst: u16, r_str: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let href = helpers::extract_ref(&frame.registers[r_str as usize]);
    let len = match ctx.heap.read_string(href) {
        Ok(s) => s.len() as i64,       // returns byte count -- CORRECT
        Err(_) => return ExecutionResult::Crash("StrLen: not a string".into()),
    };
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Int(len);
    ExecutionResult::Continue
}
```

### serialize.rs — Orphaned Body Matching (RT-03 bug site)
```rust
// Source: writ-compiler/src/emit/serialize.rs:115-137
// Collect orphaned body indices (bodies with method_def_id == None, i.e. lambda bodies).
// These are matched to orphaned MethodDefs (def_id == None) in discovery order.
let orphaned_body_indices: Vec<usize> = bodies
    .iter()
    .enumerate()
    .filter(|(_, b)| b.method_def_id.is_none())
    .map(|(i, _)| i)
    .collect();
let mut orphan_cursor = 0usize;

for (def_id, md) in builder.finalized_method_def_entries() {
    let body_idx = if let Some(did) = def_id {
        bodies.iter().position(|b| b.method_def_id == Some(did))
    } else {
        // Lambda MethodDef: match to the next orphaned body in order.
        let idx = orphaned_body_indices.get(orphan_cursor).copied();
        if idx.is_some() { orphan_cursor += 1; }
        idx
    };
    method_def_body_indices.push(body_idx);
    // ...
}
```

### finalize() — MethodDef Sort (RT-03 context)
```rust
// Source: writ-compiler/src/emit/module_builder.rs:582
// Methods sorted by parent TypeDefHandle index (ascending).
// Methods without parents (top-level fns) get parent = usize::MAX — sorted LAST.
self.method_defs.sort_by_key(|m| {
    m.parent.map(|p| p.0).unwrap_or(usize::MAX)
});
```

### Orphaned Body Emission Order (RT-03 context)
```rust
// Source: writ-compiler/src/emit/body/mod.rs:588-625
// Order is: Reflectable get_type() bodies FIRST, lambda bodies SECOND.
// The comment explicitly states this must match MethodDef table order after finalize().
for info in reflectable_infos {  // emitted first
    bodies.push(EmittedBody { method_def_id: None, ... });
}
// lambda bodies emitted after (lines 627-700+)
for (i, lambda_expr) in lambda_exprs.iter().enumerate() {
    bodies.push(EmittedBody { method_def_id: None, ... });
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `ChoiceOption` called `Option` | `ChoiceOption` renamed | Phase 42 | Golden tests updated |
| `choice()` took `List<...>` | `choice()` takes `Array<int>` | Commit 28ed763 | Sig blobs updated in builtins.rs |
| No closure capture | Captures populated by compiler | Phase 109 | Closure capture list now works |
| Choice in quest_system.writ | Removed as workaround (commit a7ea521) | 2026-03-12 | RT-03 is still outstanding |

**Deprecated/outdated:**
- `dlg_fn_mix.writ` comment "avoids known ::choice serialization bug": The comment is still accurate as of research date (RT-03 is not fixed).

## Open Questions

1. **Is RT-02 actually broken or is it a missing-test issue?**
   - What we know: `exec_str_len` handler code is correct; unit test passes with `I2s → StrLen` path
   - What's unclear: Whether `StrLen` can be triggered via compiler-generated code where `r_str` holds something other than `Value::Ref`
   - Recommendation: Add `LoadString → StrLen` unit test first. If it passes, the plan task is "add E2E test only". If it fails, trace back through `exec_load_string` to find the discrepancy.

2. **What exact module structure triggers RT-03?**
   - What we know: Commit a7ea521 says "multi-function module" + choice lambdas; simple 4-function test doesn't reproduce
   - What's unclear: Whether the bug requires `entity` + `impl` blocks, global variables, enum types, or a specific function count
   - Recommendation: Use the original `quest_system.writ` fragment (with `present_quest_choice` function) as the reproduction case. Start from that and simplify until minimum repro.

3. **Is the orphaned-body cursor approach the correct fix for RT-03?**
   - What we know: The `orphan_cursor` increments when a None-def_id MethodDef is found; the issue is that after `finalize()` sorts MethodDefs, the relative order of orphaned MethodDefs must match orphaned bodies
   - What's unclear: Whether the fix is in serializer (reorder the cursor matching) or in body emission order
   - Recommendation: Add assertions `assert_eq!(orphaned_body_indices.len(), expected_orphan_count)` in a debug build to confirm the mismatch, then fix ordering.

## Environment Availability

Step 2.6: SKIPPED (no external tool dependencies — all code changes are within the Cargo workspace).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`#[test]`) + insta snapshot (golden tests use custom snapshot logic) |
| Config file | `Cargo.toml` per crate, no separate test config |
| Quick run command | `cargo test -p writ-runtime str_len` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RT-02 | `s.len()` on string "hello" returns `Value::Int(5)` | unit | `cargo test -p writ-runtime str_len_loadstring_returns_byte_length` | ❌ Wave 0 |
| RT-02 | `s.len()` compiled from source returns byte length via full pipeline | integration | `cargo test -p writ-cli test_str_len_returns_byte_length` | ❌ Wave 0 |
| RT-03 | Multi-function module with choice lambdas: `Module::from_bytes` succeeds | integration | `cargo test -p writ-cli test_multi_fn_choice_round_trip` | ❌ Wave 0 |
| RT-03 | Multi-function module with choice lambdas: compiles + runs without crash | golden | `cargo test -p writ-golden test_fn_multi_choice` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime str_len`
- **Per wave merge:** `cargo test -p writ-runtime && cargo test -p writ-cli && cargo test -p writ-golden`
- **Phase gate:** Full suite `cargo test` green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/vm_tests.rs` — add `str_len_loadstring_returns_byte_length` test
- [ ] `writ-cli/tests/e2e_compile_tests.rs` — add `test_str_len_returns_byte_length` test
- [ ] `writ-cli/tests/e2e_compile_tests.rs` — add `test_multi_fn_choice_round_trip` test
- [ ] `writ-golden/tests/golden/fn_multi_choice.writ` — new golden source file
- [ ] `writ-golden/tests/golden/fn_multi_choice.writil` — golden snapshot (bless after fix)
- [ ] `writ-golden/tests/golden_tests.rs` — add `test_fn_multi_choice` registration

## Sources

### Primary (HIGH confidence)
- Direct code review: `writ-runtime/src/dispatch/arith.rs:451` — exec_str_len implementation
- Direct code review: `writ-compiler/src/emit/serialize.rs:115-137` — orphaned body matching
- Direct code review: `writ-compiler/src/emit/body/mod.rs:588-700` — body emission order
- Direct code review: `writ-compiler/src/emit/module_builder.rs:582` — finalize sort
- `writ-runtime/tests/vm_tests.rs:1143` — existing str_len_returns_length test (passes)
- `writ-compiler/tests/emit_body_tests.rs:2117` — existing test_string_len_emits_str_len (passes)

### Secondary (MEDIUM confidence)
- Git commit `a7ea521`: "fix(quick-2-01): revise quest_system.writ to avoid ::choice in multi-fn modules" — documents the RT-03 bug and its root cause
- `writ-golden/tests/golden/dlg_fn_mix.writ` comment: "avoids known ::choice serialization bug" — confirms RT-03 is still present as of research date

### Tertiary (LOW confidence)
- Manual reproduction attempts: simple 4-function module with choice lambdas runs successfully (did not reproduce RT-03 in simple cases)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates are in-workspace, no external dependencies
- Architecture (RT-02): HIGH — handler code reviewed directly, test patterns verified
- Architecture (RT-03): MEDIUM — bug location identified by commit message; exact trigger condition not fully reproduced in manual testing
- Pitfalls: HIGH — directly derived from code structure

**Research date:** 2026-03-28
**Valid until:** 2026-04-28 (stable codebase, no fast-moving external dependencies)
