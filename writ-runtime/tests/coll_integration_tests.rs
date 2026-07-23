//! Integration tests: compile and execute writ-std collections at runtime.
//!
//! Verifies COLL-01..03, COLL-05, COLL-06 (Phase 117 Plan 03):
//! Each test inlines its collection class source rather than concatenating the
//! full writ-std, because the compiler has a method-index resolution bug when
//! multiple generic impl blocks are compiled together (tracked for Phase 119).
//!
//! The `coll_with_library_separate_modules` test exercises `RuntimeBuilder::with_library()`
//! using separately-compiled std and user modules. Cross-module type resolution was
//! implemented in Phase 122, so user code can reference `List<int>` from a library
//! module without inlining the class definition.

/// writ-std source for the with_library path test.
const WRIT_STD_SRC: &str = include_str!("../../writ-std/src/collections.writ");

/// Maximum instructions per test to prevent infinite loops from consuming all memory.
const MAX_INSTRUCTIONS: u64 = 100_000;

/// Compile Writ source to bytes. Spawns on a 16 MB stack thread to handle
/// deep AST recursion in the compiler.
fn compile(src: &str) -> Vec<u8> {
    let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || writ_compiler::compile_source(src_static).unwrap())
        .unwrap()
        .join()
        .unwrap()
}

/// Find the index of the `main` method in a module.
fn find_main_idx(module: &writ_module::Module) -> usize {
    module
        .method_defs
        .iter()
        .enumerate()
        .find(|(_, md)| {
            writ_module::heap::read_string(&module.string_heap, md.name).unwrap_or("") == "main"
        })
        .map(|(idx, _)| idx)
        .expect("no main method found")
}

fn assert_task_result(
    runtime: &writ_runtime::Runtime,
    task_id: writ_runtime::TaskId,
    expected: i64,
) {
    assert_eq!(
        runtime.task_state(task_id),
        Some(writ_runtime::TaskState::Completed),
        "task did not complete: {:?}",
        runtime.crash_info(task_id)
    );
    assert_eq!(
        runtime.return_value(task_id),
        Some(writ_runtime::Value::Int(expected))
    );
}

/// Compile and execute Writ source with instruction-limited execution.
/// Panics on infinite loop, VM crash, or an incorrect observable result.
fn run_to_completion(src: &str, expected: i64) {
    let bytes = compile(src);
    let module = writ_module::Module::from_bytes(&bytes).unwrap();
    let main_idx = find_main_idx(&module);
    let mut runtime = writ_runtime::RuntimeBuilder::new(module)
        .with_gc()
        .build()
        .unwrap();
    let task_id = runtime.spawn_task(main_idx, vec![]).unwrap();
    match runtime.tick(
        0.0,
        writ_runtime::ExecutionLimit::Instructions(MAX_INSTRUCTIONS),
    ) {
        writ_runtime::TickResult::AllCompleted | writ_runtime::TickResult::Empty => {}
        writ_runtime::TickResult::ExecutionLimitReached => {
            panic!(
                "INFINITE LOOP DETECTED: test exceeded {} instructions — likely an infinite recursion or unresolvable method dispatch",
                MAX_INSTRUCTIONS
            );
        }
        other => panic!("unexpected tick result: {:?}", other),
    }
    assert_task_result(&runtime, task_id, expected);
}

/// Run user_src with writ-std loaded as a SEPARATE library module via with_library().
/// This tests the cross-module loading code path that writ-cli depends on.
///
/// Uses `compile_with_libraries` so user code can reference types defined in
/// writ-std (e.g. `List<int>`) without inlining the class definition.
fn run_with_library(user_src: &str, expected: i64) {
    let std_bytes = compile(WRIT_STD_SRC);
    let std_module = writ_module::Module::from_bytes(&std_bytes).unwrap();
    // Use compile_with_libraries so user code can reference types from std_module
    let user_src_static: &'static str = Box::leak(user_src.to_string().into_boxed_str());
    let user_bytes = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            writ_compiler::compile_with_libraries(user_src_static, &[&std_module])
                .expect("compile_with_libraries failed")
        })
        .unwrap()
        .join()
        .unwrap();
    let user_module = writ_module::Module::from_bytes(&user_bytes).unwrap();
    let main_idx = find_main_idx(&user_module);
    // Rebuild std_module for the runtime (already consumed above via move)
    let std_bytes2 = compile(WRIT_STD_SRC);
    let std_module2 = writ_module::Module::from_bytes(&std_bytes2).unwrap();
    let mut runtime = writ_runtime::RuntimeBuilder::new(user_module)
        .with_library(std_module2)
        .with_gc()
        .build()
        .unwrap();
    let task_id = runtime.spawn_task(main_idx, vec![]).unwrap();
    match runtime.tick(
        0.0,
        writ_runtime::ExecutionLimit::Instructions(MAX_INSTRUCTIONS),
    ) {
        writ_runtime::TickResult::AllCompleted | writ_runtime::TickResult::Empty => {}
        writ_runtime::TickResult::ExecutionLimitReached => {
            panic!(
                "INFINITE LOOP DETECTED: test exceeded {} instructions",
                MAX_INSTRUCTIONS
            );
        }
        other => panic!("unexpected tick result: {:?}", other),
    }
    assert_task_result(&runtime, task_id, expected);
}

// ── with_library path test ─────────────────────────────────────────────────────

/// Tests RuntimeBuilder::with_library() — compiles std and user code as distinct modules.
/// Exercises the cross-module type resolution path that writ-cli depends on.
///
/// Cross-module type resolution is implemented in Phase 122. User code that
/// references `List<int>` is compiled with `compile_with_libraries` so the
/// compiler can see the List type from the separately-compiled writ-std module.
#[test]
fn coll_with_library_separate_modules() {
    run_with_library(
        r#"
fn main() -> int {
    let mut list: List<int> = new List<int> { items: [] };
    list.add(42);
    list.get(0)
}
"#,
        42,
    );
}

// ── List<T> tests ─────────────────────────────────────────────────────────────

#[test]
fn coll_list_add_get_len() {
    run_to_completion(
        r#"
pub class List<T> { mut items: T[] }
impl<T> List<T> {
    pub fn add(mut self, item: T) {
        let old_len: int = self.items.len();
        self.items.resize(old_len + 1);
        self.items[old_len] = item;
    }
    pub fn get(self, index: int) -> T { self.items[index] }
    pub fn set(mut self, index: int, item: T) { self.items[index] = item; }
    pub fn len(self) -> int { self.items.len() }
    pub fn remove_at(mut self, index: int) {
        let old_len: int = self.items.len();
        let mut j: int = index;
        while j < old_len - 1 {
            self.items[j] = self.items[j + 1];
            j = j + 1;
        }
        self.items.resize(old_len - 1);
    }
    pub fn has(self, item: T) -> bool {
        let mut i: int = 0;
        while i < self.items.len() {
            if self.items[i] == item { return true; }
            i = i + 1;
        }
        false
    }
}
fn main() -> int {
    let mut list: List<int> = new List<int> { items: [] };
    list.add(10);
    list.add(20);
    list.add(30);
    list.set(1, 25);
    list.remove_at(0);
    if list.len() != 2 { return -1; }
    if list.get(0) != 25 { return -2; }
    if list.get(1) != 30 { return -3; }
    if list.has(20) { return -4; }
    if !list.has(25) { return -5; }
    203
}
"#,
        203,
    );
}

// ── Map<K, V> tests ───────────────────────────────────────────────────────────

#[test]
fn coll_map_set_get_remove() {
    run_to_completion(
        r#"
pub class Map<K: Ord + Eq, V> { mut keys: K[], mut values: V[] }
impl<K: Ord + Eq, V> Map<K, V> {
    pub fn len(self) -> int { self.keys.len() }
    pub fn has(self, key: K) -> bool {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { return true; }
            i = i + 1;
        }
        false
    }
    pub fn get(self, key: K) -> V {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { return self.values[i]; }
            i = i + 1;
        }
        self.values[0]
    }
    pub fn set(mut self, key: K, value: V) {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { self.values[i] = value; return; }
            i = i + 1;
        }
        let old_len: int = self.keys.len();
        self.keys.resize(old_len + 1);
        self.values.resize(old_len + 1);
        self.keys[old_len] = key;
        self.values[old_len] = value;
    }
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
}
fn main() -> int {
    let mut map: Map<string, int> = new Map<string, int> { keys: [], values: [] };
    map.set("a", 1);
    map.set("b", 2);
    map.remove("a");
    if map.len() != 1 { return -1; }
    if map.has("a") { return -2; }
    if !map.has("b") { return -3; }
    map.get("b")
}
"#,
        2,
    );
}

// ── Set<T> tests ──────────────────────────────────────────────────────────────

#[test]
fn coll_map_distinguishes_different_string_keys() {
    run_to_completion(
        r#"
pub class Map<K: Ord + Eq, V> { mut keys: K[], mut values: V[] }
impl<K: Ord + Eq, V> Map<K, V> {
    pub fn set(mut self, key: K, value: V) {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { self.values[i] = value; return; }
            i = i + 1;
        }
        let old_len: int = self.keys.len();
        self.keys.resize(old_len + 1);
        self.values.resize(old_len + 1);
        self.keys[old_len] = key;
        self.values[old_len] = value;
    }
    pub fn len(self) -> int { self.keys.len() }
}
fn main() -> int {
    let mut map: Map<string, int> = new Map<string, int> { keys: [], values: [] };
    map.set("alpha", 1);
    map.set("beta", 2);
    map.len()
}
"#,
        2,
    );
}

#[test]
fn coll_set_add_dedup_remove() {
    run_to_completion(
        r#"
pub class Set<T: Eq> { mut items: T[] }
impl<T: Eq> Set<T> {
    pub fn add(mut self, item: T) {
        if self.has(item) { return; }
        let old_len: int = self.items.len();
        self.items.resize(old_len + 1);
        self.items[old_len] = item;
    }
    pub fn remove(mut self, item: T) {
        let mut i: int = 0;
        while i < self.items.len() {
            if self.items[i] == item {
                let old_len: int = self.items.len();
                let mut j: int = i;
                while j < old_len - 1 {
                    self.items[j] = self.items[j + 1];
                    j = j + 1;
                }
                self.items.resize(old_len - 1);
                return;
            }
            i = i + 1;
        }
    }
    pub fn has(self, item: T) -> bool {
        let mut i: int = 0;
        while i < self.items.len() {
            if self.items[i] == item { return true; }
            i = i + 1;
        }
        false
    }
    pub fn len(self) -> int { self.items.len() }
}
fn main() -> int {
    let mut s: Set<int> = new Set<int> { items: [] };
    s.add(1);
    s.add(2);
    s.add(1);
    s.remove(1);
    if s.len() != 1 { return -1; }
    if s.has(1) { return -2; }
    if !s.has(2) { return -3; }
    s.len()
}
"#,
        1,
    );
}

// ── HashMap<K, V> tests ───────────────────────────────────────────────────────

#[test]
fn coll_hashmap_set_get_remove() {
    run_to_completion(
        r#"
pub class HashMap<K: Hashable, V> { mut keys: K[], mut values: V[] }
impl<K: Hashable, V> HashMap<K, V> {
    pub fn len(self) -> int { self.keys.len() }
    pub fn has(self, key: K) -> bool {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { return true; }
            i = i + 1;
        }
        false
    }
    pub fn get(self, key: K) -> V {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { return self.values[i]; }
            i = i + 1;
        }
        self.values[0]
    }
    pub fn set(mut self, key: K, value: V) {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { self.values[i] = value; return; }
            i = i + 1;
        }
        let old_len: int = self.keys.len();
        self.keys.resize(old_len + 1);
        self.values.resize(old_len + 1);
        self.keys[old_len] = key;
        self.values[old_len] = value;
    }
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
}
fn main() -> int {
    let mut hm: HashMap<string, int> = new HashMap<string, int> { keys: [], values: [] };
    hm.set("x", 10);
    hm.set("y", 20);
    hm.remove("x");
    if hm.len() != 1 { return -1; }
    if hm.has("x") { return -2; }
    if !hm.has("y") { return -3; }
    hm.get("y")
}
"#,
        20,
    );
}

// ── Iterator protocol tests (Phase 118) ───────────────────────────────────────

#[test]
fn coll_hashmap_distinguishes_different_string_keys() {
    run_to_completion(
        r#"
pub class HashMap<K: Hashable, V> { mut keys: K[], mut values: V[] }
impl<K: Hashable, V> HashMap<K, V> {
    pub fn set(mut self, key: K, value: V) {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { self.values[i] = value; return; }
            i = i + 1;
        }
        let old_len: int = self.keys.len();
        self.keys.resize(old_len + 1);
        self.values.resize(old_len + 1);
        self.keys[old_len] = key;
        self.values[old_len] = value;
    }
    pub fn len(self) -> int { self.keys.len() }
}
fn main() -> int {
    let mut map: HashMap<string, int> = new HashMap<string, int> { keys: [], values: [] };
    map.set("alpha", 1);
    map.set("beta", 2);
    map.len()
}
"#,
        2,
    );
}

/// ITER-01: for-in loop over List<T> using Iterable<T> protocol.
#[test]
fn iter_for_in_list() {
    run_to_completion(
        r#"
pub class ListIterator<T> {
    source: T[],
    mut index: int
}
impl<T> ListIterator<T> {
    pub fn next(mut self) -> T? {
        if self.index >= self.source.len() { return null; }
        let item: T = self.source[self.index];
        self.index = self.index + 1;
        item
    }
}
impl<T> Iterator<T> for ListIterator<T> {
    pub fn next(mut self) -> T? {
        if self.index >= self.source.len() { return null; }
        let item: T = self.source[self.index];
        self.index = self.index + 1;
        item
    }
}
pub class List<T> { mut items: T[] }
impl<T> List<T> {
    pub fn add(mut self, item: T) {
        let old_len: int = self.items.len();
        self.items.resize(old_len + 1);
        self.items[old_len] = item;
    }
    pub fn len(self) -> int { self.items.len() }
}
impl<T> Iterable<T> for List<T> {
    pub fn iterator(self) -> Iterator<T> {
        new ListIterator<T> { source: self.items, index: 0 }
    }
}
fn main() -> int {
    let mut list: List<int> = new List<int> { items: [] };
    list.add(10);
    list.add(20);
    list.add(30);
    let mut sum: int = 0;
    for x in list {
        sum = sum + x;
    }
    sum
}
"#,
        60,
    );
}

/// ITER-02: for-in loop over Set<T> using Iterable<T> protocol.
#[test]
fn iter_for_in_set() {
    run_to_completion(
        r#"
pub class SetIterator<T: Eq> {
    source: T[],
    mut index: int
}
impl<T: Eq> SetIterator<T> {
    pub fn next(mut self) -> T? {
        if self.index >= self.source.len() { return null; }
        let item: T = self.source[self.index];
        self.index = self.index + 1;
        item
    }
}
impl<T: Eq> Iterator<T> for SetIterator<T> {
    pub fn next(mut self) -> T? {
        if self.index >= self.source.len() { return null; }
        let item: T = self.source[self.index];
        self.index = self.index + 1;
        item
    }
}
pub class Set<T: Eq> { mut items: T[] }
impl<T: Eq> Set<T> {
    pub fn add(mut self, item: T) {
        if self.has(item) { return; }
        let old_len: int = self.items.len();
        self.items.resize(old_len + 1);
        self.items[old_len] = item;
    }
    pub fn has(self, item: T) -> bool {
        let mut i: int = 0;
        while i < self.items.len() {
            if self.items[i] == item { return true; }
            i = i + 1;
        }
        false
    }
    pub fn len(self) -> int { self.items.len() }
}
impl<T: Eq> Iterable<T> for Set<T> {
    pub fn iterator(self) -> Iterator<T> {
        new SetIterator<T> { source: self.items, index: 0 }
    }
}
fn main() -> int {
    let mut s: Set<int> = new Set<int> { items: [] };
    s.add(1);
    s.add(2);
    s.add(1);
    let mut sum: int = 0;
    for x in s {
        sum = sum + x;
    }
    sum
}
"#,
        3,
    );
}

/// ITER-03: iterate Map keys using get_keys() which returns K[] (array path).
/// Uses string keys to avoid GenericParam resolution limitations (Phase 119+).
#[test]
fn iter_for_map_keys() {
    run_to_completion(
        r#"
pub class Map<K: Ord + Eq, V> {
    mut keys: K[],
    mut values: V[]
}
impl<K: Ord + Eq, V> Map<K, V> {
    pub fn set(mut self, key: K, value: V) {
        let mut i: int = 0;
        while i < self.keys.len() {
            if self.keys[i] == key { self.values[i] = value; return; }
            i = i + 1;
        }
        let old_len: int = self.keys.len();
        self.keys.resize(old_len + 1);
        self.values.resize(old_len + 1);
        self.keys[old_len] = key;
        self.values[old_len] = value;
    }
    pub fn get_keys(self) -> K[] { self.keys }
    pub fn len(self) -> int { self.keys.len() }
}
fn main() -> int {
    let mut map: Map<string, int> = new Map<string, int> { keys: [], values: [] };
    map.set("a", 10);
    map.set("b", 20);
    map.set("c", 30);
    let mut key_count: int = 0;
    for k in map.get_keys() {
        let _k: string = k;
        key_count = key_count + 1;
    }
    key_count
}
"#,
        3,
    );
}

/// ITER-04: custom class implementing Iterable<T> works in for-in loop.
#[test]
fn iter_custom_iterable() {
    run_to_completion(
        r#"
pub class CounterIterator {
    mut current: int,
    max: int
}
impl CounterIterator {
    pub fn next(mut self) -> int? {
        if self.current >= self.max { return null; }
        let v: int = self.current;
        self.current = self.current + 1;
        v
    }
}
impl Iterator<int> for CounterIterator {
    pub fn next(mut self) -> int? {
        if self.current >= self.max { return null; }
        let v: int = self.current;
        self.current = self.current + 1;
        v
    }
}
pub class Counter {
    max: int
}
impl Iterable<int> for Counter {
    pub fn iterator(self) -> Iterator<int> {
        new CounterIterator { current: 0, max: self.max }
    }
}
fn main() -> int {
    let counter: Counter = new Counter { max: 5 };
    let mut sum: int = 0;
    for x in counter {
        sum = sum + x;
    }
    sum
}
"#,
        10,
    );
}

/// COLL-04: List map/filter/reduce chain produces correct results.
#[test]
fn coll_list_map_filter_reduce() {
    run_to_completion(
        r#"
pub class List<T> { mut items: T[] }
impl<T> List<T> {
    pub fn add(mut self, item: T) {
        let old_len: int = self.items.len();
        self.items.resize(old_len + 1);
        self.items[old_len] = item;
    }
    pub fn len(self) -> int { self.items.len() }
    pub fn map(self, f: fn(T) -> T) -> List<T> {
        let mut result: List<T> = new List<T> { items: self.items.slice(0, 0) };
        let mut i: int = 0;
        while i < self.items.len() {
            result.add(f(self.items[i]));
            i = i + 1;
        }
        result
    }
    pub fn filter(self, f: fn(T) -> bool) -> List<T> {
        let mut result: List<T> = new List<T> { items: self.items.slice(0, 0) };
        let mut i: int = 0;
        while i < self.items.len() {
            if f(self.items[i]) { result.add(self.items[i]); }
            i = i + 1;
        }
        result
    }
    pub fn reduce(self, initial: T, f: fn(T, T) -> T) -> T {
        let mut acc: T = initial;
        let mut i: int = 0;
        while i < self.items.len() {
            acc = f(acc, self.items[i]);
            i = i + 1;
        }
        acc
    }
}
fn main() -> int {
    let mut list: List<int> = new List<int> { items: [] };
    list.add(1);
    list.add(2);
    list.add(3);
    list.add(4);
    list.add(5);
    let doubled: List<int> = list.map(fn(x: int) -> int { x * 2 });
    let filtered: List<int> = doubled.filter(fn(x: int) -> bool { x > 4 });
    filtered.reduce(0, fn(acc: int, x: int) -> int { acc + x })
}
"#,
        24,
    );
}
