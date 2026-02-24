# 1. Writ Language Specification
## 8. Structs

Structs are named composite types with named fields. They support methods and operator overloading via `impl` blocks,
and lifecycle hooks directly in the struct body.

```
struct Merchant {
    name: string,
    gold: int,
    reputation: float = 0.5,
}

impl Merchant {
    fn greet(self) -> string {
        $"Welcome! I am {self.name}"
    }
}

// Construction uses the `new` keyword with named fields
let m = new Merchant { name: "Old Tim", gold: 100 };
```

### 8.1 Construction

Structs are constructed with the `new` keyword followed by the type name and brace-enclosed field initializers. Fields
with default values may be omitted. Fields without defaults are required at every construction site.

```
let m = new Merchant { name: "Old Tim", gold: 100 };           // reputation defaults to 0.5
let m2 = new Merchant { name: "Sue", gold: 50, reputation: 0.9 };
```

The `new` keyword disambiguates construction from block expressions, making the syntax unambiguous for the parser.

For convenience factories, use static methods in `impl` blocks:

```
impl Merchant {
    fn create(name: string) -> Merchant {
        new Merchant { name: name, gold: 0 }
    }
}

let m = Merchant::create("Tim");
```

### 8.2 Lifecycle Hooks

Structs may define lifecycle hooks using the `on` keyword directly in the struct body. All hooks receive an implicit
`mut self` parameter.

```
struct NativeConnection {
    url: string,
    handle: int = 0,

    on create {
        self.handle = native_connect(self.url);
    }

    on finalize {
        native_disconnect(self.handle);
    }

    on serialize {
        self.handle = 0;
    }

    on deserialize {
        self.handle = native_connect(self.url);
    }
}
```

| Hook             | When                              | Purpose                                                 |
|------------------|-----------------------------------|---------------------------------------------------------|
| `on create`      | After all fields are initialized  | Post-initialization logic                               |
| `on finalize`    | GC is about to collect the object | Last-chance cleanup of native resources                 |
| `on serialize`   | Before the object is serialized   | Park native state (clear handles, prepare for snapshot) |
| `on deserialize` | After the object is deserialized  | Recreate native state from stored fields                |

**Implicit `on create`:** Every struct conceptually has an `on create` hook. The compiler generates code to initialize
all fields (explicit values and defaults) before the user-written `on create` body runs. If no user `on create` is
written, construction simply initializes fields and returns.

**`on finalize` semantics:** The finalizer runs when the garbage collector determines the object is unreachable. Timing
is non-deterministic. Storing `self` in a reachable location during `on finalize` (resurrection) is undefined behavior.

**Hook failure semantics:** If any lifecycle hook crashes (via `!` unwrap, out-of-bounds access, etc.), the crash
unwinds and terminates the calling task's entire call stack. The runtime must log the failure to the host via the
runtime logging interface (see IL spec §1.14.7). Specific consequences by hook:

- `on create` crash: The object is left in a partially initialized state. The crash terminates the task that called
  `new`.
- `on serialize` crash: The runtime logs the error. Whether the save proceeds without this object or fails entirely is
  runtime-defined.
- `on deserialize` crash: The runtime logs the error. The object exists but may have unrecovered native state.
- `on finalize` crash: The runtime logs the error and continues GC collection. The finalizer does not retry.

### 8.3 Construction Sequence

`new Merchant { name: "Tim", gold: 100 }` compiles to the following IL:

```
NEW           r0, Merchant_type       // 1. Allocate zeroed memory
LOAD_FLOAT    r1, 0.5                 // 2. Load default for reputation
SET_FIELD     r0, reputation_field, r1
LOAD_STRING   r1, "Tim"_idx          // 3. Apply construction-site overrides
SET_FIELD     r0, name_field, r1
LOAD_INT      r1, 100
SET_FIELD     r0, gold_field, r1
CALL          r_, Merchant::__on_create, r0  // 4. Run on create (if defined)
```

The full sequence:

1. **NEW** — allocate zeroed memory for the struct.
2. **SET_FIELD** — apply default values for all fields that have them.
3. **SET_FIELD** — apply construction-site overrides (these overwrite defaults where specified).
4. **CALL `__on_create`** — run the user-defined `on create` body, if present. At this point, all fields are fully
   initialized.

---

