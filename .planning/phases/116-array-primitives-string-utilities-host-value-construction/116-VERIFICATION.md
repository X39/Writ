---
phase: 116-array-primitives-string-utilities-host-value-construction
verified: 2026-03-29T00:00:00Z
status: gaps_found
score: 12/14 must-haves verified
gaps:
  - truth: "HOST-02: Host can construct values inside an ImmediateWithHeap extern handler"
    status: partial
    reason: "ImmediateWithHeap variant exists and is wired correctly through dispatch/calls.rs, but the integration test file host_construct_tests.rs fails to compile because it passes integer literals (0) where TypeDefKind enum values are required by add_type_def(). The HOST-02 test section is a placeholder comment with no actual tests."
    artifacts:
      - path: "writ-runtime/tests/host_construct_tests.rs"
        issue: "Test file does not compile: lines 16 and 41 pass integer literal `0` where `TypeDefKind` enum is expected. Error: mismatched types, expected `TypeDefKind`, found integer."
    missing:
      - "Fix host_construct_tests.rs: replace `0` with `TypeDefKind::Struct` (requires `use writ_module::TypeDefKind;` import)"
      - "Add at least one actual ImmediateWithHeap test after the placeholder comment on line 137"

  - truth: "REQUIREMENTS.md reflects completion of HOST-01, HOST-02, HOST-03"
    status: failed
    reason: "REQUIREMENTS.md still shows HOST-01, HOST-02, HOST-03 as unchecked (- [ ]) and 'Pending' in the status table, even though the implementation artifacts exist in code."
    artifacts:
      - path: ".planning/REQUIREMENTS.md"
        issue: "Lines 45-47 show unchecked checkboxes for HOST-01/02/03. Lines 105-107 show 'Pending' status."
    missing:
      - "Update REQUIREMENTS.md: change HOST-01/02/03 from '- [ ]' to '- [x]' and from 'Pending' to 'Complete'"
human_verification:
  - test: "Confirm ImmediateWithHeap handler receives heap and can allocate strings"
    expected: "Handler registered via on_with_heap() runs when extern is called; can call heap.alloc_string(); returns Value::Ref"
    why_human: "The test file does not compile so this path cannot be verified automatically"
---

# Phase 116: Array Primitives, String Utilities, Host Value Construction Verification Report

**Phase Goal:** Users can call mutation methods on arrays, string utility methods, and host Rust code can construct type-validated Writ values
**Verified:** 2026-03-29
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can call arr.add(x) on any T[] and the element is appended | VERIFIED | `"add"` at access.rs:185, builtins.rs:96; dispatches ArrayAdd |
| 2 | User can call arr.remove_at(i) on any T[] | VERIFIED | `"remove_at"` at access.rs:186, builtins.rs:102; dispatches ArrayRemove |
| 3 | User can call arr.insert(i, x) on any T[] | VERIFIED | `"insert"` at access.rs:187, builtins.rs:108; dispatches ArrayInsert |
| 4 | User can call arr.contains(x) on any T[] and receives bool | VERIFIED | `"contains"` at access.rs:188, builtins.rs:115; ArrayContains 0x0909 full e2e |
| 5 | User can call arr.slice(start, end) | VERIFIED | Existing opcode, wired; golden test array_primitives.writ passes |
| 6 | User can call s.split(sep) and receive a string[] | VERIFIED | access.rs:324, builtins.rs:185, StrSplit 0x0E09, exec_str_split in arith.rs:632 |
| 7 | User can call s.trim() | VERIFIED | access.rs:308, builtins.rs:152, StrTrim 0x0E03, exec_str_trim in arith.rs:518 |
| 8 | User can call s.starts_with(p) and s.ends_with(s) | VERIFIED | access.rs:311-316, builtins.rs:167/173, dispatch/mod.rs:531-532 |
| 9 | User can call s.contains(sub) | VERIFIED | access.rs:319, builtins.rs:179, StrContains 0x0E08, exec_str_contains arith.rs:608 |
| 10 | User can call s.replace(from, to) | VERIFIED | access.rs:323, builtins.rs:191, StrReplace 0x0E0A, exec_str_replace arith.rs:667 |
| 11 | User can call s.to_upper() and s.to_lower() | VERIFIED | access.rs:309-310; uses to_ascii_uppercase/to_ascii_lowercase; golden test passes |
| 12 | Hashable contract registered for int, string, bool, float | VERIFIED | virtual_module.rs:168, IntrinsicId IntHash/FloatHash/BoolHash/StringHash in mod.rs:90; FNV-1a at intrinsics.rs:1074 |
| 13 | Host Rust code can construct a Writ struct value and receive a valid Value | VERIFIED | runtime.rs:756 `pub fn construct_value`; all 4 error message strings present |
| 14 | HOST-02: Host can construct values inside ImmediateWithHeap extern handler | PARTIAL | Variant exists (extern_registry.rs:59), dispatch wired (calls.rs:197), but integration test file fails to compile |

**Score:** 13/14 truths verified (1 partial = gap)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-module/src/instruction.rs` | ArrayContains 0x0909 + 8 StrXxx opcodes | VERIFIED | ArrayContains at line 157; StrTrim-StrReplace at lines 250-264 with encode/decode |
| `writ-compiler/src/check/check_expr/access.rs` | add, remove_at, insert, contains (array) + 8 string methods | VERIFIED | Array methods lines 185-191; string methods lines 308-329 |
| `writ-compiler/src/emit/body/expr/builtins.rs` | Emitter arms for all array and string methods | VERIFIED | Array methods lines 96-120; string methods lines 152-195 |
| `writ-runtime/src/dispatch/objects.rs` | exec_array_contains | VERIFIED | Line 323 |
| `writ-runtime/src/dispatch/arith.rs` | 8 exec_str_* functions | VERIFIED | Lines 518-667, all 8 present |
| `writ-runtime/src/virtual_module.rs` | Hashable contract registration | VERIFIED | Lines 167-168, contract 20 |
| `writ-runtime/src/runtime.rs` | `pub fn construct_value` | VERIFIED | Line 756; all 4 error paths present |
| `writ-runtime/src/extern_registry.rs` | ImmediateWithHeap variant + on_with_heap | VERIFIED | Line 59 (variant), line 134 (convenience method), line 317 (on_extern_call_with_heap impl) |
| `writ-runtime/src/host.rs` | on_extern_call_with_heap default method | VERIFIED | Line 127 |
| `writ-runtime/tests/host_construct_tests.rs` | Integration tests for construct_value + ImmediateWithHeap | PARTIAL | construct_value tests present but file does not compile (TypeDefKind mismatch); ImmediateWithHeap test section is placeholder only |
| `writ-golden/tests/golden/array_primitives.writ` | Golden test for all array methods | VERIFIED | File exists; golden test passes |
| `writ-golden/tests/golden/string_utilities.writ` | Golden test for all string methods | VERIFIED | File exists; golden test passes |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| access.rs (TyKind::Array) | builtins.rs (TyKind::Array) | Method names match exactly | VERIFIED | "add", "remove_at", "insert", "contains" present in both files |
| builtins.rs (ArrayContains) | dispatch/mod.rs | Instruction variant matches dispatch arm | VERIFIED | ArrayContains at mod.rs:466 |
| access.rs (TyKind::String) | builtins.rs (TyKind::String) | 8 string method names match exactly | VERIFIED | All 8 names present in both files |
| builtins.rs (StrSplit/StrReplace/StrTrim) | dispatch/mod.rs | Instruction variants match dispatch arms | VERIFIED | All 8 dispatch arms present at mod.rs:528-535 |
| dispatch/calls.rs | host.rs on_extern_call_with_heap | Dispatch loop tries heap-aware path first | VERIFIED | calls.rs:197 calls on_extern_call_with_heap before on_request |
| extern_registry.rs | host.rs ImmediateWithHeap | ExternHost implements on_extern_call_with_heap dispatching ImmediateWithHeap | VERIFIED | extern_registry.rs:317-337 |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Array primitives golden test | `cargo test -p writ-golden -- array_primitives` | 1/1 pass | PASS |
| String utilities golden test | `cargo test -p writ-golden -- string_utilities` | 1/1 pass | PASS |
| writ-runtime lib tests | `cargo test -p writ-runtime --lib` | 156/156 pass | PASS |
| writ-compiler lib tests | `cargo test -p writ-compiler --lib` | 27/27 pass | PASS |
| host_construct_tests integration | `cargo test -p writ-runtime --test host_construct_tests` | compile error | FAIL |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| STR-01 | 116-02 | s.split(sep) returns string[] | SATISFIED | StrSplit 0x0E09, exec_str_split, golden test |
| STR-02 | 116-02 | s.trim() strips whitespace | SATISFIED | StrTrim 0x0E03, exec_str_trim (uses Rust trim()) |
| STR-03 | 116-02 | s.starts_with / s.ends_with | SATISFIED | StrStartsWith/StrEndsWith 0x0E06/07, both dispatch arms present |
| STR-04 | 116-02 | s.contains(substr) | SATISFIED | StrContains 0x0E08, exec_str_contains |
| STR-05 | 116-02 | s.replace(from, to) | SATISFIED | StrReplace 0x0E0A, exec_str_replace |
| STR-06 | 116-02 | s.to_upper() / s.to_lower() | SATISFIED | to_ascii_uppercase/to_ascii_lowercase in arith.rs:536/550 |
| HOST-01 | 116-03 | construct_value by name with type validation | SATISFIED (impl) | runtime.rs:756; error paths verified; REQUIREMENTS.md not updated |
| HOST-02 | 116-03 | Heap access in immediate extern handlers | PARTIAL | ImmediateWithHeap wired; test file fails to compile; no ImmediateWithHeap tests run |
| HOST-03 | 116-03 | Clear error on wrong count/type mismatch | SATISFIED (impl) | Error strings at runtime.rs:752-827; test assertions pass IF file compiled |

**Note on REQUIREMENTS.md:** STR-01..STR-06 are correctly marked `[x]` Complete. HOST-01/02/03 remain marked `[ ]` Pending in both the checkbox list and the status table, despite implementation existing. This is a documentation gap, not a code gap, but is flagged because the phase contract is not reflected in project state.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| writ-runtime/tests/host_construct_tests.rs | 16, 41 | Integer literal `0` where `TypeDefKind` enum expected; Rust won't coerce | BLOCKER | `cargo test -p writ-runtime --test host_construct_tests` fails to compile; HOST-02 cannot be verified |
| writ-runtime/tests/host_construct_tests.rs | 137 | `// Task 2 Tests: ImmediateWithHeap will be added after Task 2 is implemented` — placeholder, no tests | WARNING | HOST-02 behavioral coverage absent |
| .planning/REQUIREMENTS.md | 45-47, 105-107 | HOST-01/02/03 still marked Pending/unchecked despite implementation | INFO | Project state is inconsistent with code |

### Human Verification Required

#### 1. ImmediateWithHeap handler end-to-end behavior

**Test:** After fixing the TypeDefKind compile error, register a handler via `registry.on_with_heap("test_fn", |_args, heap| { let href = heap.alloc_string("hello"); Ok(Value::Ref(href)) })`, call the extern from a Writ script, and confirm the returned Value::Ref reads back as "hello".
**Expected:** Handler executes, heap allocation succeeds, caller receives correct Value::Ref.
**Why human:** The test file currently does not compile, and even after fixing it, the test for ImmediateWithHeap is a stub comment with no test body.

## Gaps Summary

Two gaps block full goal achievement:

**Gap 1 (Blocker): host_construct_tests.rs does not compile.** The test file passes integer literals where `TypeDefKind` enum values are required by `ModuleBuilder::add_type_def`. This causes `cargo test -p writ-runtime --test host_construct_tests` to fail with two type mismatch errors. The fix is straightforward: add `use writ_module::TypeDefKind;` at the top of the file, then replace `0` with `TypeDefKind::Struct` on lines 16 and 41.

**Gap 2 (Warning): No ImmediateWithHeap integration tests exist.** The test section for Task 2 of Plan 116-03 is a single placeholder comment. The ImmediateWithHeap dispatch is correctly wired in production code, but the behavioral contract (handler receives heap, can allocate, returns Value) has no automated test.

The STR-01..06 requirements are fully implemented and verified end-to-end. The HOST-01/03 implementation is substantively correct and would be verified if the test file compiled. HOST-02 cannot be declared verified without a passing test.

---

_Verified: 2026-03-29_
_Verifier: Claude (gsd-verifier)_
