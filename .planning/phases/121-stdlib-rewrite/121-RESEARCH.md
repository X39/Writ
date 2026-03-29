# Phase 121: Stdlib Rewrite - Research

**Researched:** 2026-03-29
**Domain:** Writ standard library source (collections.writ) — rewrite four collection classes to use fixed-size array API
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — discuss phase was skipped per `workflow.skip_discuss`.

### Claude's Discretion
All implementation choices are at Claude's discretion. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STD-01 | List<T> uses resize + indexed assignment internally (no removed array methods) | Old impl identified; replacement pattern using resize+copy_from confirmed working via array_primitives.writ golden test |
| STD-02 | Map<K,V> uses resize + indexed assignment internally | Old impl identified; append-and-shift pattern derived from resize+copy_from |
| STD-03 | Set<T> uses resize + indexed assignment internally | Old impl identified; Set.has() must switch from contains to manual loop |
| STD-04 | HashMap<K,V> uses resize + indexed assignment internally | Identical structure to Map<K,V> — same rewrite pattern applies |
| STD-05 | All collection integration tests pass with rewritten internals | 7 golden tests are `#[ignore]` with Phase 121 marker — must be re-enabled and blessed |
</phase_requirements>

---

## Summary

Phase 121 is a pure Writ source rewrite. No Rust compiler changes are needed. The single file `writ-std/src/collections.writ` must be updated so that all four collection classes (List<T>, Map<K,V>, Set<T>, HashMap<K,V>) implement their growth and removal operations using only `resize`, `copy_from`, indexed assignment (`arr[i] = v`), `len()`, and `slice()` — the methods that survived the Phase 120 clean-break array API.

The removed methods are `add`, `remove_at`, `insert`, and `contains` on `T[]`. Every usage of these in the current `collections.writ` must be replaced. The public API of each collection class (push, pop-equivalent, get, set, contains/has, keys, values, map, filter, reduce) must remain unchanged so existing golden test `.writ` driver files require zero edits.

Seven golden tests in `writ-golden/tests/golden_tests.rs` are currently `#[ignore]` with the comment `// Phase 121: re-enable after stdlib rewrite`. Once the collections compile cleanly, each must be un-ignored and its `.writil` snapshot re-blessed via `BLESS=1`. The `writ-cli/build.rs` currently writes an empty placeholder `.writc` when stdlib compilation fails; after the rewrite it will produce a real module.

**Primary recommendation:** Rewrite `collections.writ` with the patterns below, then un-ignore and re-bless all 7 collection golden tests.

---

## Standard Stack

This phase is entirely within the Writ language layer — no new Rust crates or library versions needed.

### Core (already present)
| Asset | Location | Purpose |
|-------|----------|---------|
| `writ-std/src/collections.writ` | Writ source | The file being rewritten |
| `writ-golden/tests/golden_tests.rs` | Rust test | 7 collection tests to un-ignore |
| `writ-golden/tests/golden/coll_*.writil` | IL snapshots | Must be re-blessed after rewrite |
| `writ-cli/build.rs` | Rust build script | Will succeed automatically once stdlib compiles |

### Array API Available After Phase 120 (HIGH confidence — verified in builtins.rs and array_primitives.writ)
| Method | Signature | IL Emitted |
|--------|-----------|------------|
| `arr.len()` | `() -> int` | `ARRAY_LEN` |
| `arr[i]` | `int -> T` | `ARRAY_LOAD` |
| `arr[i] = v` | `int, T -> void` | `ARRAY_STORE` |
| `arr.resize(n)` | `int -> void` | `ARRAY_RESIZE` |
| `arr.slice(s, e)` | `int, int -> T[]` | `ARRAY_SLICE` |
| `arr.copy_from(src, src_idx, dst_idx, len)` | `T[], int, int, int -> void` | `ARRAY_COPY` |

**Removed methods (must not appear in rewritten source):**
- `arr.add(item)` — replaced by resize+store pattern
- `arr.remove_at(i)` — replaced by shift-down+resize pattern
- `arr.insert(i, item)` — not used in collections.writ; out of scope
- `arr.contains(item)` — replaced by manual linear scan

---

## Architecture Patterns

### Pattern 1: Append (replaces `arr.add(item)`)

The collections need a way to append one element to the end of a backing array. With the fixed-size API:

```writ
// Append `item` to `self.items`
let old_len: int = self.items.len();
self.items.resize(old_len + 1);
self.items[old_len] = item;
```

This is the core growth primitive. Every method that previously called `self.items.add(x)` or `self.keys.add(x)` uses this two-line pattern.

**Consequence for List<T>.add():** The public method becomes the wrapper for the append pattern. All internal calls from `map()`, `filter()`, etc. that call `result.add(f(...))` continue to call the List method — which is now backed by resize+store. No change to how the higher-order methods are structured; they already go through `result.add()`.

### Pattern 2: Remove-at-index (replaces `arr.remove_at(i)`)

Removing element at index `i` requires shifting all elements after `i` one position left, then shrinking:

```writ
// Remove element at index `i` from `self.items`
let old_len: int = self.items.len();
let mut j: int = i;
while j < old_len - 1 {
    self.items[j] = self.items[j + 1];
    j = j + 1;
}
self.items.resize(old_len - 1);
```

Alternatively, this can use `copy_from` for the shift:

```writ
// Using copy_from for the shift — receiver is destination
let old_len: int = self.items.len();
if i < old_len - 1 {
    // dst.copy_from(src, src_idx, dst_idx, len)
    // shift elements [i+1 .. old_len) one position left
    self.items.copy_from(self.items, i + 1, i, old_len - i - 1);
}
self.items.resize(old_len - 1);
```

Both approaches are valid. The `copy_from` form is more concise. Either can be used; the manual loop is simpler to reason about and has no overlap-semantics edge cases (copy_from has memmove semantics, so overlap is safe either way — verified in array_primitives.writ golden test for D-09).

### Pattern 3: Linear Scan (replaces `arr.contains(item)`)

Set<T>.has() currently delegates to `self.items.contains(item)`. With `contains` removed from T[], the scan must be explicit:

```writ
pub fn has(self, item: T) -> bool {
    let mut i: int = 0;
    while i < self.items.len() {
        if self.items[i] == item { return true; }
        i = i + 1;
    }
    false
}
```

This is the same pattern already used by Map<K,V>.has() and HashMap<K,V>.has() — no new logic needed, just apply it to Set<T>.

### Pattern 4: Parallel-array append (Map, HashMap)

Both Map and HashMap use `self.keys.add(key); self.values.add(value);` for insertion of new keys. Both arrays must be grown together:

```writ
// Append new key-value pair (used at end of set() when key not found)
let old_len: int = self.keys.len();
self.keys.resize(old_len + 1);
self.values.resize(old_len + 1);
self.keys[old_len] = key;
self.values[old_len] = value;
```

### Pattern 5: Parallel-array remove (Map, HashMap)

`remove()` calls `self.keys.remove_at(i); self.values.remove_at(i);`. Both must use the remove-at pattern independently:

```writ
// Remove entry at index i from parallel arrays
let old_len: int = self.keys.len();
let mut j: int = i;
while j < old_len - 1 {
    self.keys[j] = self.keys[j + 1];
    self.values[j] = self.values[j + 1];
    j = j + 1;
}
self.keys.resize(old_len - 1);
self.values.resize(old_len - 1);
```

### Anti-Patterns to Avoid

- **Calling `arr.add()` on the backing array inside any collection impl.** The type checker will reject it with `unknown method 'add'` — but this is what must be removed, not worked around.
- **Delegating `has()` to `arr.contains()`.** Same reason — rejected at compile time.
- **Extracting remove-at into a helper function.** The Writ stdlib is a single class-per-collection structure; there is no free-function or shared-helper pattern used today. Keep logic inline per method.
- **Changing the public API.** The golden test `.writ` driver files use `list.add()`, `list.remove_at()`, `set.has()`, `map.set()`, `map.remove()`, etc. These are the *public* methods on the collection classes — they must stay. Only the *internal array operations* change.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Growing array | Custom linked-list or chunk allocator | `resize(n+1)` + indexed store |
| Shrinking array | Manual memory tricks | Shift loop + `resize(n-1)` |
| Membership test | External hash set or bitmap | Manual `while` loop with `==` comparison |

**Key insight:** The entire rewrite is mechanical substitution of 3 primitive operations (add, remove_at, contains) with explicit sequences using the 3 surviving array operations (resize, indexed store, len). No algorithmic changes are needed; the O(n) characteristics are unchanged.

---

## Common Pitfalls

### Pitfall 1: Forgetting remove_at removes from public API too
**What goes wrong:** Developer replaces `self.items.remove_at(i)` in `List.remove_at()` but also removes the public method `remove_at` from `List`.
**Why it happens:** The name `remove_at` appears in both the public method signature and the old internal call.
**How to avoid:** Only change the *body* of `remove_at` — the method signature stays exactly as is. The method remains public.
**Warning signs:** Golden test `golden_coll_list_basic` exercises `list.remove_at(0)` — if the method is missing, the test will fail with a resolution error, not a type error.

### Pitfall 2: Off-by-one in shift loop
**What goes wrong:** The shift loop `while j < old_len` instead of `while j < old_len - 1` overwrites a valid element or reads out of bounds.
**Why it happens:** Index arithmetic is easy to get wrong by one.
**How to avoid:** The loop must stop when `j == old_len - 2` (the second-to-last position), so the condition is `j < old_len - 1`. Verify: if array is `[A, B, C]` and we remove index 0, loop runs for j=0 and j=1; j=0 writes `arr[0] = arr[1]`, j=1 writes `arr[1] = arr[2]`. Then resize(2). Result: `[B, C]`. Correct.
**Warning signs:** Incorrect IL output in blessed snapshot — `ARRAY_LOAD` with an out-of-range index, or a resize to the wrong size.

### Pitfall 3: Not re-blessing golden snapshots after rewrite
**What goes wrong:** Tests fail with "mismatch" because the `.writil` snapshots still contain `ARRAY_ADD` and `ARRAY_REMOVE` opcodes from the old implementation.
**Why it happens:** Golden tests compare compiled output byte-for-byte against the blessed `.writil` file. The new implementation emits different IL.
**How to avoid:** After the rewrite compiles cleanly, run `BLESS=1 cargo test -p writ-golden` to regenerate all 7 collection snapshots.
**Warning signs:** Test output says `--- expected` shows `ARRAY_ADD`, `+++ actual` shows `ARRAY_RESIZE` / `ARRAY_STORE`.

### Pitfall 4: copy_from argument order confusion
**What goes wrong:** `copy_from(src, src_idx, dst_idx, len)` — the source array is arg 0, source index arg 1, DESTINATION index arg 2. This is the opposite of the `memcpy(dst, src, n)` convention.
**Why it happens:** The receiver of `copy_from` is the destination array; argument order was designed to be destination-explicit (D-07). Easy to swap src_idx and dst_idx.
**How to avoid:** Read the call as "receiver.copy_from(where_to_read_from, start_in_source, start_in_dest, how_many)". Verified in `access.rs`: `copy_from(src: T[], src_idx: int, dst_idx: int, len: int)`.
**Warning signs:** Wrong values at wrong positions in the IL snapshot; off-by-one in copied ranges.

### Pitfall 5: writ-cli placeholder not being replaced
**What goes wrong:** `writ-cli/build.rs` still writes an empty placeholder even after the stdlib compiles.
**Why it happens:** Not a bug — the build script already handles success correctly. But if the stdlib still has a compilation error somewhere, the placeholder logic will silently swallow it with a `cargo:warning`.
**How to avoid:** After the rewrite, explicitly verify the build script succeeds by running `cargo build -p writ-cli` and confirming no `writ-std compilation failed` warning appears.
**Warning signs:** `cargo build -p writ-cli` emits a `cargo:warning=writ-std compilation failed` line.

---

## Code Examples

### Rewritten List<T>.add() — append pattern
```writ
// Source: builtins.rs array API, array_primitives.writ (verified)
pub fn add(mut self, item: T) {
    let old_len: int = self.items.len();
    self.items.resize(old_len + 1);
    self.items[old_len] = item;
}
```

### Rewritten List<T>.remove_at() — shift-and-shrink pattern
```writ
pub fn remove_at(mut self, index: int) {
    let old_len: int = self.items.len();
    let mut j: int = index;
    while j < old_len - 1 {
        self.items[j] = self.items[j + 1];
        j = j + 1;
    }
    self.items.resize(old_len - 1);
}
```

### Rewritten List<T>.has() — manual scan (replaces contains)
```writ
pub fn has(self, item: T) -> bool {
    let mut i: int = 0;
    while i < self.items.len() {
        if self.items[i] == item { return true; }
        i = i + 1;
    }
    false
}
```

### Rewritten Set<T>.has() — same pattern, was delegating to arr.contains
```writ
pub fn has(self, item: T) -> bool {
    let mut i: int = 0;
    while i < self.items.len() {
        if self.items[i] == item { return true; }
        i = i + 1;
    }
    false
}
```

### Rewritten Map<K,V>.set() — parallel-array append for new key
```writ
pub fn set(mut self, key: K, value: V) {
    let mut i: int = 0;
    while i < self.keys.len() {
        if self.keys[i] == key {
            self.values[i] = value;
            return;
        }
        i = i + 1;
    }
    // Key not found — append to both arrays
    let old_len: int = self.keys.len();
    self.keys.resize(old_len + 1);
    self.values.resize(old_len + 1);
    self.keys[old_len] = key;
    self.values[old_len] = value;
}
```

### Rewritten Map<K,V>.remove() — parallel shift-and-shrink
```writ
pub fn remove(mut self, key: K) {
    let mut i: int = 0;
    while i < self.keys.len() {
        if self.keys[i] == key {
            let old_len: int = self.keys.len();
            let mut j: int = i;
            while j < old_len - 1 {
                self.keys[j] = self.keys[j + 1];
                self.values[j] = self.values[j + 1];
                j = j + 1;
            }
            self.keys.resize(old_len - 1);
            self.values.resize(old_len - 1);
            return;
        }
        i = i + 1;
    }
}
```

---

## Runtime State Inventory

Step 2.5: SKIPPED — this is not a rename/refactor/migration phase. It is a pure Writ source rewrite within a single file.

---

## Environment Availability

Step 2.6: SKIPPED — this phase has no external dependencies beyond the existing Rust toolchain and `cargo test`. Both are already in use by the project.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` (no external test framework) |
| Config file | `writ-golden/Cargo.toml` — no separate test config |
| Quick run command | `cargo test -p writ-golden -- coll` |
| Full suite command | `cargo test -p writ-golden` |
| Bless command | `BLESS=1 cargo test -p writ-golden -- coll` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | Test Exists? |
|--------|----------|-----------|-------------------|--------------|
| STD-01 | List<T> internals use resize+indexed assign | golden | `cargo test -p writ-golden -- golden_coll_list_basic` | Yes (ignored) |
| STD-01 | List<T>.map() compiles with new internals | golden | `cargo test -p writ-golden -- golden_coll_list_map` | Yes (ignored) |
| STD-01 | List<T>.filter() compiles with new internals | golden | `cargo test -p writ-golden -- golden_coll_list_filter` | Yes (ignored) |
| STD-01 | List<T>.reduce() compiles with new internals | golden | `cargo test -p writ-golden -- golden_coll_list_reduce` | Yes (ignored) |
| STD-02 | Map<K,V> internals use resize+indexed assign | golden | `cargo test -p writ-golden -- golden_coll_map_basic` | Yes (ignored) |
| STD-03 | Set<T> internals use resize+indexed assign | golden | `cargo test -p writ-golden -- golden_coll_set_basic` | Yes (ignored) |
| STD-04 | HashMap<K,V> internals use resize+indexed assign | golden | `cargo test -p writ-golden -- golden_coll_hashmap_basic` | Yes (ignored) |
| STD-05 | writ-cli/build.rs produces non-empty .writc | smoke | `cargo build -p writ-cli` (check no warning) | Indirect |
| STD-05 | iter_for_in_list uses List iterator protocol | golden | `cargo test -p writ-golden -- golden_iter_for_in_list` | Yes (ignored) |

**Note on iter_for_in_list:** This test is also `#[ignore]` with a Phase 121 comment. It tests the List Iterable/Iterator contract which is part of the same collections.writ file. It must also be un-ignored and re-blessed.

### Sampling Rate
- **Per task commit:** `cargo test -p writ-golden -- coll` (runs all 7 collection tests, fast)
- **Per wave merge:** `cargo test -p writ-golden` (full golden suite, ~30 seconds)
- **Phase gate:** `cargo test -p writ-golden` green + `cargo build -p writ-cli` warning-free

### Wave 0 Gaps
None — all test infrastructure exists. Tests are present but `#[ignore]`d. No new test files or framework setup needed. The only setup required is:
- Remove `#[ignore]` from 8 tests (7 coll_* + iter_for_in_list) in `golden_tests.rs`
- Re-bless snapshots via `BLESS=1 cargo test -p writ-golden -- coll iter_for_in_list`

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `arr.add(item)` | `arr.resize(n+1); arr[n] = item` | Phase 120 (v14.0) | Growth is explicit; dynamic behavior on collection classes only |
| `arr.remove_at(i)` | manual shift loop + `arr.resize(n-1)` | Phase 120 (v14.0) | Removal is explicit; O(n) same as before |
| `arr.contains(item)` | manual `while` loop with `==` | Phase 120 (v14.0) | Linear scan, same complexity |

**Deprecated/outdated in writ-std/src/collections.writ:**
- `self.items.add(item)` — 4 occurrences across List, Set, Map, HashMap
- `self.items.remove_at(i)` — 3 occurrences across List, Set (remove), Map/HashMap (remove)
- `self.keys.add(key)` / `self.values.add(value)` — 4 calls across Map + HashMap
- `self.keys.remove_at(i)` / `self.values.remove_at(i)` — 4 calls across Map + HashMap
- `self.items.contains(item)` — 2 occurrences in Set.has() and List.has()

Total: ~17 call sites to replace.

---

## Open Questions

1. **iter_for_in_list golden test scope**
   - What we know: The test is `#[ignore]` with a Phase 121 marker. It uses the `ListIterator<T>` class which is already in collections.writ and does not use any removed array methods (ListIterator.next() only uses `len()` and indexed access).
   - What's unclear: Whether the test was ignored purely because collections.writ failed to compile as a whole, or because ListIterator itself has a separate issue.
   - Recommendation: Un-ignore it together with the collection tests. If it reveals a separate issue, it can be re-ignored with a new marker.

2. **`get_keys` and `get_values` on Map<K,V>**
   - What we know: Map has `pub fn get_keys(self) -> K[]` and `pub fn get_values(self) -> V[]` which return the raw backing arrays. These methods do not use any removed array methods.
   - What's unclear: Whether there are golden tests exercising these methods that would require snapshot updates.
   - Recommendation: These methods require no changes. No action needed.

---

## Sources

### Primary (HIGH confidence)
- `writ-std/src/collections.writ` — current source, all 17 problematic call sites identified
- `writ-golden/tests/golden_tests.rs` — 8 tests confirmed `#[ignore]` with Phase 121 marker
- `writ-compiler/src/emit/body/expr/builtins.rs` — confirmed `resize`, `copy_from`, `len`, `slice` are the only array methods emitted
- `writ-compiler/src/check/check_expr/access.rs` — confirmed type checker only accepts `len`, `slice`, `resize`, `copy_from` on `TyKind::Array`
- `writ-golden/tests/golden/array_primitives.writil` — verified IL opcodes for all Phase 120 array methods
- `writ-cli/build.rs` — confirmed placeholder mechanism and success path

### Secondary (MEDIUM confidence)
- `.planning/REQUIREMENTS.md` — STD-01 through STD-05 scope confirmed
- `.planning/STATE.md` — Phase 120 decisions and decisions carried forward

---

## Metadata

**Confidence breakdown:**
- What needs changing: HIGH — source code read directly, all call sites counted
- Replacement patterns: HIGH — derived from array API verified in compiler source and golden IL
- Test scope: HIGH — golden_tests.rs read directly, all 8 ignored tests identified
- Snapshot re-blessing workflow: HIGH — BLESS=1 mechanism documented in golden_tests.rs

**Research date:** 2026-03-29
**Valid until:** Stable — this phase is entirely self-contained within a single Writ source file and the existing test harness. No external library changes are relevant.
