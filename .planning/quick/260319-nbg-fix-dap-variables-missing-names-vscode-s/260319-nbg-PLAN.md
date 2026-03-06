---
phase: quick-260319-nbg
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - writ-compiler/src/emit/serialize.rs
  - writ-dap/src/server/helpers.rs
  - writ-dap/tests/test_protocol.rs
autonomous: true
requirements: [NBG-01, NBG-02, NBG-03]

must_haves:
  truths:
    - "DAP variables response contains source-level variable names (not empty strings or register indices)"
    - "Temporary/unnamed registers are filtered out of the variables list"
    - "Variable scope ranges use byte-offset PCs consistently (not instruction indices)"
  artifacts:
    - path: "writ-dap/src/server/helpers.rs"
      provides: "collect_frame_variables filters out unnamed temporaries"
      contains: "name.is_empty"
    - path: "writ-compiler/src/emit/serialize.rs"
      provides: "build_debug_locals converts instruction-index PCs to byte-offset PCs"
      contains: "instr_byte_starts"
    - path: "writ-dap/tests/test_protocol.rs"
      provides: "Integration test asserting variable names in breakpoint inspection"
      contains: "variable_names"
  key_links:
    - from: "writ-compiler/src/emit/serialize.rs"
      to: "writ-dap/src/server/helpers.rs"
      via: "DebugLocal.start_pc/end_pc byte offsets consumed by collect_frame_variables"
      pattern: "dl\\.start_pc.*byte"
---

<objective>
Fix DAP variables response to show source-level variable names instead of unnamed/empty entries.

Purpose: When VSCode hits a breakpoint and requests variables, the response currently includes ALL registers (including unnamed temporaries with empty names) and uses instruction-index PCs for variable scope ranges instead of byte-offset PCs. This causes the Variables panel to show unnamed entries and potentially misscoped variables.

Output: Two bug fixes (filter unnamed temporaries, fix PC conversion) and one integration test validating variable names on breakpoint hits.
</objective>

<execution_context>
@.planning/STATE.md
</execution_context>

<context>
Key existing code:

**collect_frame_variables** (writ-dap/src/server/helpers.rs:51-81):
Iterates `debug_locals`, filters by PC range, maps to DAP Variable structs.
Currently includes ALL registers including temporaries with name offset 0 (reads as "" from string heap).

**build_debug_locals** (writ-compiler/src/emit/serialize.rs:535-571):
Builds DebugLocal entries for ALL registers (0..reg_count). Unnamed registers get name_offset=0.
start_pc values from emitter.debug_locals are instruction indices, NOT byte offsets.
end_pc (u32::MAX sentinel) is clamped to total_code_size (which IS byte offset).
Does NOT receive or use instr_byte_starts for PC conversion (unlike build_source_spans which does).

**build_source_spans** (writ-compiler/src/emit/serialize.rs:601-621):
Shows the correct pattern: receives instr_byte_starts and converts instruction indices to byte offsets.

**Test fixture** (writ-golden/tests/golden/fn_typed_params.writ):
```writ
pub fn add(a: int, b: int) -> int {
    let result: int = a + b;
    result
}
pub fn is_positive(n: int) -> bool {
    n > 0
}
pub fn main() {
    let x: int = add(3, 4);
    let flag: bool = is_positive(x);
}
```
Breakpoint on line 11: `let x: int = add(3, 4);` — after stopping, variables in `main` should include `x` (once assigned).

**DapClient test helper** (writ-dap/tests/common/mod.rs):
Provides `variables(vars_ref)` method returning the variables body JSON.

**Existing test** (writ-dap/tests/test_protocol.rs:72-210):
`test_breakpoint_hit_and_inspect` already runs the full stopped->threads->stackTrace->scopes->variables chain but only asserts `!variables.is_empty()`, never checks variable names.
</context>

<tasks>

<task type="auto">
  <name>Task 1: Fix build_debug_locals PC conversion and filter unnamed temporaries</name>
  <files>writ-compiler/src/emit/serialize.rs, writ-dap/src/server/helpers.rs</files>
  <action>
**Bug fix 1 — writ-compiler/src/emit/serialize.rs (build_debug_locals):**

Pass `instr_byte_starts: &[usize]` as an additional parameter to `build_debug_locals`. Convert each `start_pc` from instruction index to byte offset using `instr_byte_starts.get(start_pc as usize).copied().unwrap_or(0) as u32`. The `end_pc` sentinel value u32::MAX should still clamp to `total_code_size` (already byte offset). For non-sentinel `end_pc` values, also convert using `instr_byte_starts.get(end_pc as usize).copied().unwrap_or(total_code_size as usize) as u32`.

Update the call site in `translate()` (around line 311) to pass `&instr_byte_starts` which is already computed on line 289.

**Bug fix 2 — writ-dap/src/server/helpers.rs (collect_frame_variables):**

After the `.filter(|dl| dl.start_pc <= pc as u32 && (pc as u32) < dl.end_pc)` line, add a second `.filter()` that excludes entries where `dl.name == 0` (name offset 0 = empty string = unnamed temporary register). This ensures only named variables (params and let bindings) appear in the DAP variables response.

The filter should be: `.filter(|dl| dl.name != 0)` placed BEFORE or AFTER the PC range filter (order doesn't matter, but before is slightly more efficient since it avoids the string heap lookup for temporaries).
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test --package writ-compiler emit_serialize -- --nocapture 2>&1 | tail -5 && cargo test --package writ-dap test_variables_handler -- --nocapture 2>&1 | tail -5</automated>
  </verify>
  <done>
    - build_debug_locals receives instr_byte_starts and converts start_pc/end_pc from instruction indices to byte offsets
    - collect_frame_variables filters out DebugLocal entries with name offset 0 (unnamed temporaries)
    - Existing unit tests still pass
  </done>
</task>

<task type="auto">
  <name>Task 2: Add integration test validating variable names on breakpoint hit</name>
  <files>writ-dap/tests/test_protocol.rs</files>
  <action>
Add a new test function `test_breakpoint_variables_have_names` in `writ-dap/tests/test_protocol.rs`. This test should:

1. Use the existing `fn_typed_params.writ` fixture (FIXTURE constant already defined).
2. Set a breakpoint on line 12 (`let flag: bool = is_positive(x);`) — at this point, `x` should be in scope with value `7` (result of add(3,4)).
3. Follow the full DAP chain: initialize -> setBreakpoints(line 12) -> configurationDone -> launch -> stopped event -> threads -> stackTrace -> scopes -> variables.
4. Extract the variables array from the variables response.
5. Assert that:
   - At least one variable exists
   - No variable has an empty name (`""`) — this catches the unnamed-temporaries bug
   - A variable named `"x"` exists in the variables list
   - The variable named `"x"` has value `"7"` (the result of add(3, 4))
   - The variable named `"x"` has type_field `"int"`
6. Continue and verify terminated event.
7. Shutdown cleanly.

Also update the existing `test_breakpoint_hit_and_inspect` test to add a basic assertion that no variable has an empty name, as a regression guard:
After the `assert!(!variables.is_empty(), ...)` line, add:
```rust
// Regression: no variable should have an empty name (unnamed temporaries must be filtered)
for var in variables {
    let name = var.get("name").and_then(|v| v.as_str()).unwrap_or("");
    assert!(!name.is_empty(), "variable should have a non-empty name, got: {}", var);
}
```
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test --package writ-dap test_breakpoint_variables_have_names test_breakpoint_hit_and_inspect -- --nocapture 2>&1 | tail -15</automated>
  </verify>
  <done>
    - New test `test_breakpoint_variables_have_names` passes, confirming variable "x" with value "7" and type "int" is present
    - Updated `test_breakpoint_hit_and_inspect` confirms no empty-name variables
    - All existing DAP tests continue to pass
  </done>
</task>

</tasks>

<verification>
Run the full DAP test suite to ensure no regressions:
```bash
cd D:/dev/git/Writ && cargo test --package writ-dap -- --nocapture 2>&1 | tail -20
```

Run the compiler serialization tests to verify the PC conversion change:
```bash
cd D:/dev/git/Writ && cargo test --package writ-compiler emit_serialize -- --nocapture 2>&1 | tail -10
```
</verification>

<success_criteria>
- DAP variables response for a breakpoint at line 12 of fn_typed_params.writ contains variable "x" with value "7" and type "int"
- No variable in the variables response has an empty name
- build_debug_locals correctly converts instruction-index PCs to byte-offset PCs
- All existing writ-dap and writ-compiler tests pass
</success_criteria>

<output>
After completion, create `.planning/quick/260319-nbg-fix-dap-variables-missing-names-vscode-s/260319-nbg-SUMMARY.md`
</output>
