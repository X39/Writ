# 1.9 Classes

Classes are reference types — named composite types with named fields, heap-allocated, GC-managed, shared-on-assign.
They support methods and operator overloading via `impl` blocks, and lifecycle hooks directly in the class body.
Classes are what `struct` meant in prior versions of the language.

```writ
class Merchant {
    name: string,
    mut gold: int,
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

## 1.9.1 Construction

Classes are constructed with the `new` keyword followed by the type name and brace-enclosed field initializers. Fields
with default values may be omitted when the compiler has the default expression available. Fields without defaults are
required at every construction site. Version 9's cross-module limitation is specified in Section 2.11.

`new` for a class **allocates memory on the GC heap**. The variable holds a reference (pointer) to the heap object.
Assigning a class value copies the reference — both variables refer to the same object. This contrasts with structs,
where `new` initializes a value inline with no heap allocation (see Section 1.8).

```writ
let m = new Merchant { name: "Old Tim", gold: 100 };   // reputation defaults to 0.5
let m2 = new Merchant { name: "Sue", gold: 50, reputation: 0.9 };

let mut m3 = m;   // m3 and m point to the same Merchant object on the heap
m3.gold = 80;     // allowed because m3 and Merchant.gold are both mutable
// m.gold is now also 80 — they are the same object
```

The `new` keyword disambiguates construction from block expressions, making the syntax unambiguous for the parser.

For convenience factories, use static methods in `impl` blocks:

```writ
impl Merchant {
    fn create(name: string) -> Merchant {
        new Merchant { name: name, gold: 0 }
    }
}

let m = Merchant::create("Tim");
```

## 1.9.2 Lifecycle Hooks

Classes may define lifecycle hooks using the `on` keyword directly in the class body. All hooks receive an implicit
`mut self` parameter.

```writ
class NativeConnection {
    url: string,
    mut handle: int = 0,

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

The hooks' implicit `mut self` permits mutation through the receiver, but it does not override field metadata:
`handle` must also be declared `mut`. Fields such as `url` remain read-only after construction.

| Hook             | When                              | Purpose                                                 |
|------------------|-----------------------------------|---------------------------------------------------------|
| `on create`      | After all fields are initialized  | Post-initialization logic                               |
| `on finalize`    | GC is about to collect the object | Last-chance cleanup of native resources                 |
| `on serialize`   | Before the object is serialized   | Park native state (clear handles, prepare for snapshot) |
| `on deserialize` | After the object is deserialized  | Recreate native state from stored fields                |

**Implicit `on create`:** Every class conceptually has an `on create` hook. The compiler generates code to initialize
all fields (explicit values and defaults) before the user-written `on create` body runs. If no user `on create` is
written, construction simply initializes fields and returns.

**`on finalize` semantics:** The finalizer runs when the garbage collector determines the object is unreachable. Timing
is non-deterministic. Storing `self` in a reachable location during `on finalize` (resurrection) is undefined behavior.

**Hook failure semantics:** If any lifecycle hook crashes (via `!` unwrap, out-of-bounds access, etc.), the crash
unwinds and terminates the calling task's entire call stack. The runtime must log the failure to the host via the
runtime logging interface (see IL spec §1.14.7). Specific consequences by hook:

- `on create` crash: The object has already been fully initialized, but construction does not return it to the caller.
  The crash terminates the task that called `new`.
- `on serialize` crash: The runtime logs the error. Whether the save proceeds without this object or fails entirely is
  runtime-defined.
- `on deserialize` crash: The runtime logs the error. The object exists but may have unrecovered native state.
- `on finalize` crash: The runtime logs the error and continues GC collection. The finalizer does not retry.

Value-type structs have no lifecycle hooks. If lifecycle hooks are needed, use a `class` (or `entity` for game objects).

## 1.9.3 Construction Sequence (IL)

`new Merchant { name: "Tim", gold: 100 }` compiles to the following IL:

```writ
LOAD_STRING   r1, "Tim"_idx
LOAD_INT      r2, 100
LOAD_FLOAT    r3, 0.5
NEW           r0, Merchant_type, 3, r1 // allocate with name, gold, reputation
CALL          r_, Merchant::__on_create, r0
```

The full sequence:

1. Evaluate exactly one initializer for every field in declaration order. Explicit values replace defaults, and the
   resulting values are placed in consecutive registers.
2. **NEW** validates the type, exact field count, and initializer register range before allocation, then allocates the
   class with its complete field vector. No zeroed, partially initialized class reference is exposed.
3. **CALL `__on_create`** — run the user-defined `on create` body, if present. At this point, all fields are fully
   initialized.

Construction may establish read-only fields. Once `NEW` returns, every write is an ordinary post-construction write;
`SET_FIELD` must crash before changing a field whose metadata is read-only.

Entities are specialized classes with additional capabilities. See Section 1.15 (Entities) for entity-specific lifecycle
hooks (`on destroy`, `on interact`) and the component system.

---
