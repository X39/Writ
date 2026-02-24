# 1. Writ Language Specification
## 23. Modules & Namespaces

Every Writ source file belongs to a namespace. Namespaces organize declarations into logical groups and prevent name
collisions. Multiple files may contribute to the same namespace. Access across namespaces uses the `::` operator.

### 23.1 Declarative Namespace

The declarative form assigns the entire file to a single namespace:

```
// file: survival/potions.writ
namespace survival;

pub struct HealthPotion {
    charges: int,
    healAmount: int,
}

pub fn heal(target: Entity, amount: int) {
    target[Health].current += amount;
}
```

**Rules:**

1. At most one declarative `namespace` statement per file.
2. Must appear before any declarations other than `using` statements.
3. All declarations in the file belong to the declared namespace.
4. The namespace name may be a qualified path (e.g., `namespace survival::items;`) to place the file in a nested
   namespace.
5. Does not support defining sub-namespaces within the file — for that, use block form (Section 23.2).

### 23.2 Block Namespace

The block form wraps declarations in a `namespace name { }` block and supports nesting:

```
namespace survival {
    pub struct HealthPotion {
        charges: int,
        healAmount: int,
    }

    namespace items {
        pub struct Bread {
            freshness: float,
        }

        pub struct Water {
            purity: float,
        }
    }
}
```

`HealthPotion` is `survival::HealthPotion`. `Bread` is `survival::items::Bread`.

**Rules:**

1. Declarative and block forms are **mutually exclusive** within a file — a file uses one or the other (or neither).
2. Multiple top-level block namespaces may appear in the same file.
3. Block namespaces may nest to arbitrary depth.
4. A namespace may span multiple files. Two files both contributing `namespace survival { ... }` merge their
   declarations into the same namespace.

### 23.3 Root Namespace

If a file contains no `namespace` declaration (neither declarative nor block), its declarations are in the **root
namespace**. Root namespace declarations are accessible without any `::` prefix from all other namespaces:

```
// file: globals.writ
// (no namespace declaration)

pub const MAX_LEVEL: int = 50;

// file: game/main.writ
namespace game;

fn example() {
    let cap = MAX_LEVEL;   // accessible without qualification — pub + root namespace
}
```

> **Note:** The root namespace is intended for small projects or truly global declarations. Larger projects should
> namespace everything.

### 23.4 `using` Declarations

The `using` keyword brings names from another namespace into scope, eliminating the need for `::` qualification:

```
// file: game/combat.writ
namespace game;

using survival;

fn example() {
    let pot = new HealthPotion { charges: 3, healAmount: 50 };
    heal(pot, 25);
}
```

Without the `using`, these would require `survival::HealthPotion` and `survival::heal`. Only `pub` declarations from
the target namespace are brought into scope.

#### 23.4.1 Alias Form

The alias form binds a namespace to a shorter name:

```
using items = survival::items;

fn example() {
    let bread = new items::Bread { freshness: 1.0 };
}
```

The alias does **not** bring individual names into scope — it only shortens the namespace prefix. `Bread` alone would
not resolve; `items::Bread` is required.

#### 23.4.2 Placement Rules

- In **declarative-form files**: `using` may appear before or after the `namespace` declaration, but must appear before
  any other declarations.
- In **block-form files**: `using` may appear at file level (before any namespace blocks) or inside a namespace block (
  scoped to that block).
- In **files with no namespace**: `using` must appear before any declarations.

```
// Declarative — using before or after namespace
using combat;
namespace survival;
using quest_system;

// ... declarations ...
```

```
// Block — using inside a namespace block
namespace game {
    using survival;
    using combat;

    fn example() {
        let pot = new HealthPotion { charges: 3, healAmount: 50 };
    }
}
```

#### 23.4.3 Scope of `using`

A `using` declaration is scoped to its enclosing context:

- File-level `using` (in declarative or no-namespace files): visible throughout the entire file.
- `using` inside a namespace block: visible only within that block and its nested blocks.

`using` does **not** re-export. A file that does `using survival;` makes `survival`'s names available locally, but
consumers of that file's namespace must add their own `using` or use `::` qualification.

### 23.5 Same-Namespace Visibility

`pub` declarations within the same namespace are visible to each other without `::` qualification, regardless of which
file they are defined in:

```
// file: survival/potions.writ
namespace survival;

pub struct HealthPotion {
    charges: int,
    healAmount: int,
}

// file: survival/crafting.writ
namespace survival;

fn brewPotion() -> HealthPotion {
    // HealthPotion is visible — same namespace, pub, no :: needed
    new HealthPotion { charges: 3, healAmount: 50 }
}
```

Non-`pub` top-level declarations are file-local and not visible from other files, even within the same namespace.

### 23.6 Visibility Modifiers

Writ has two visibility keywords: `pub` and `priv`. Declarations default to **private**.

| Modifier | Meaning                                                                                   |
|----------|-------------------------------------------------------------------------------------------|
| (none)   | **Private** — file-local for top-level declarations, type-private for members             |
| `priv`   | **Private** (explicit) — same as no modifier, for when the author wants to be intentional |
| `pub`    | **Public** — visible outside the file/type, accessible via `::` or `using`                |

#### 23.6.1 Top-Level Declarations

Top-level declarations (`fn`, `struct`, `enum`, `contract`, `entity`, `component`, `const`, `global`) default to
**private**, meaning they are visible only within the declaring file. `pub` makes them visible to all files and
namespaces.

```
namespace survival;

// Private (default) — only visible within this file
struct PotionRecipe {
    ingredients: string[],
    brewTime: float,
}

// Public — visible to any file via survival::HealthPotion or `using survival;`
pub struct HealthPotion {
    charges: int,
    healAmount: int,
}

// Public — visible to any namespace
pub fn heal(target: Entity, amount: int) {
    target[Health].current += amount;
}

// Private — helper function, file-local
fn calculateHealAmount(level: int) -> int {
    level * 10
}

// Explicit priv — identical to no modifier, expresses intent
priv fn internalHelper() {
    // ...
}
```

From another file in the same namespace:

```
// file: survival/crafting.writ
namespace survival;

fn example() {
    let pot = new HealthPotion { charges: 3, healAmount: 50 };   // OK — HealthPotion is pub
    heal(player, 25);                                           // OK — heal is pub
    let r = new PotionRecipe { ingredients: [], brewTime: 5.0 };  // ERROR — PotionRecipe is private to its file
}
```

From another namespace:

```
namespace game;
using survival;

fn example() {
    heal(player, 25);                       // OK — heal is pub
    let pot = new HealthPotion {};           // OK — HealthPotion is pub
    let pot = new survival::HealthPotion {}; // OK — fully qualified also works
    calculateHealAmount(5);             // ERROR — private, not visible outside its file
}
```

**Exception — `dlg` declarations default to `pub`.** Dialogue blocks are intended to be called from other files and
namespaces (via transitions, entity hooks, or direct invocation). A `dlg` can be made private with an explicit `priv`:

```
namespace quest;

// Public by default — can be called from other files and namespaces
dlg mainQuest(player: Entity) {
    @Narrator Your adventure begins.
}

// Explicitly private — only used within this file as a helper
priv dlg internalBranch() {
    @Narrator This is an internal branch.
}
```

#### 23.6.2 Type Members

Members of structs, entities, and components (fields, properties, and methods) default to **type-private** — only the
type's own methods can access them. `pub` makes members visible wherever the type itself is visible.

```
pub struct Merchant {
    pub name: string,             // public — accessible wherever Merchant is visible
    gold: int,                    // private — only Merchant's own methods can access
    priv discount: float = 0.1,  // private (explicit) — same as no modifier
}

impl Merchant {
    pub fn greet(self) -> string {
        $"Welcome! I am {self.name}"
    }

    fn applyDiscount(self, price: int) -> int {
        // Can access private fields — we're inside the type
        price - (price * self.discount)
    }
}
```

```
namespace survival;

fn example(m: Merchant) {
    let n = m.name;               // OK — pub
    let g = m.gold;               // ERROR — private, only Merchant methods can access
    let d = m.discount;           // ERROR — private
}
```

#### 23.6.3 Entity and Component Members

Entities and components follow the same rules as structs:

```
pub entity Guard {
    pub name: string = "Guard",
    alertLevel: int = 0,

    use Speaker {
        displayName: "Guard",
    },
    use Health {
        current: 80,
        max: 80,
    },

    pub fn greet(self) -> string {
        $"Halt! I am {self.name}"
    }

    fn raiseAlert(mut self) {
        self.alertLevel += 1;
    }

    on interact(who: Entity) {
        self.raiseAlert();        // OK — on hooks are part of the type
        -> guardDialog(self, who)
    }
}
```

Lifecycle hooks (`on`) do not take visibility modifiers — they are always type-private (invoked by the runtime, not
called by user code).

Component `use` declarations do not take visibility modifiers — component attachment is visible wherever the entity is
visible. Component field visibility is governed by the component's own declarations.

#### 23.6.4 Contracts and Implementations

Contract method signatures do not take visibility modifiers. Contract methods define a public interface — any type
implementing the contract must expose those methods publicly:

```
contract Tradeable {
    fn getInventory(self) -> List<Item>;    // no modifier — always part of the public interface
    fn trade(mut self, item: Item, with: Entity);
}
```

Methods in `impl` blocks that fulfill a contract requirement are implicitly `pub` and cannot be made private:

```
impl Tradeable for Merchant {
    fn getInventory(self) -> List<Item> { ... }   // OK — implicitly pub
    fn trade(mut self, item: Item, with: Entity) { ... }

    priv fn getInventory(self) -> List<Item> { ... }   // ERROR — contract methods cannot be private
}
```

Additional non-contract methods in an `impl` block follow normal visibility rules:

```
impl Merchant {
    pub fn greet(self) -> string { ... }
    fn calculateMarkup(self) -> float { ... }    // private — only Merchant can call this
}
```

#### 23.6.5 Enum Variants

Enum variants do not take individual visibility modifiers. All variants share the visibility of the enum itself:

```
pub enum QuestStatus {
    NotStarted,                    // all variants are pub because the enum is pub
    InProgress(currentStep: int),
    Completed,
    Failed(reason: string),
}
```

#### 23.6.6 Visibility Summary

| Declaration context           | `pub`  | (none) / `priv` |
|-------------------------------|--------|-----------------|
| Top-level (`fn`, `struct`, …) | Public | File-local      |
| Top-level `dlg`               | Public | File-local*     |
| Struct field                  | Public | Type-private    |
| Struct method (in `impl`)     | Public | Type-private    |
| Entity property               | Public | Type-private    |
| Entity method                 | Public | Type-private    |
| Entity lifecycle hook (`on`)  | —      | Always internal |
| Component field               | Public | Type-private    |
| Contract method signature     | —      | Always public   |
| Contract impl method          | —      | Always public   |
| Enum variant                  | —      | Inherits enum   |

*`dlg` defaults to `pub`; an explicit `priv` makes it file-local. All other declarations default to private.

### 23.7 Name Conflicts

If two namespaces define a type or function with the same name, and both are brought into scope via `using`, any
**unqualified** reference to that name is a compile error:

```
namespace ns_a;
pub struct Item { name: string }

namespace ns_b;
pub struct Item { id: int }
```

```
// file: main.writ
namespace main;

using ns_a;
using ns_b;

fn example() {
    let x = Item();         // ERROR: ambiguous — Item exists in both ns_a and ns_b
    let y = ns_a::Item();   // OK — fully qualified
    let z = ns_b::Item();   // OK — fully qualified
}
```

The error occurs at the **usage site**, not at the `using` declaration. Having two `using` statements that *could*
conflict is legal as long as no ambiguous name is actually used without qualification.

> **Note:** Only `pub` declarations are visible outside their declaring file. A `using` only brings `pub` declarations
> into scope. Private declarations are never accessible from other files, even with `::` qualification.

### 23.8 Cross-Namespace Access

The `::` operator accesses `pub` names within a namespace:

```
let pot = new survival::HealthPotion { charges: 3, healAmount: 50 };
let bread = new survival::items::Bread { freshness: 1.0 };
survival::heal(player, 25);
```

Fully qualified names always work for `pub` declarations, regardless of `using` declarations. They also resolve
ambiguity when multiple `using` statements bring conflicting names into scope. Private declarations cannot be accessed
via `::` from outside their file.

### 23.9 Root Namespace Prefix (`::`)

A leading `::` with no left-hand side refers to the root namespace. This resolves ambiguity when a nested namespace
shadows an outer one:

```
namespace engine {
    namespace audio {
        pub struct Mixer { channels: int }
    }
}

namespace audio {
    pub struct Mixer { sampleRate: int }
}
```

```
namespace engine::audio;

fn example() {
    // "audio" here resolves to engine::audio (inner takes priority)
    let a = Mixer(channels: 8);

    // Leading :: forces resolution from the root
    let b = ::audio::Mixer(sampleRate: 44100);
}
```

**Resolution rule:** When an unqualified name could refer to either a sibling/child namespace or a root-level namespace,
the **innermost** (closest enclosing) match takes priority. Use `::name` to bypass this and start resolution from the
root.

The leading `::` works in all expression and type contexts:

```
let x = new ::survival::HealthPotion { charges: 3, healAmount: 50 };
let y: ::survival::HealthPotion = x;
::survival::heal(player, 25);
```

### 23.10 `::` Resolution

The `::` operator is used in three contexts:

1. **Root namespace access** — `::survival::HealthPotion` (leading `::`, resolve from root)
2. **Namespace access** — `survival::HealthPotion`, `survival::items::Bread`
3. **Enum variant access** — `QuestStatus::InProgress`, `Option::Some(value)`

The compiler resolves `::` by checking whether the left-hand side names a namespace or a type. If it names a namespace,
namespace lookup is performed. If it names an enum type, variant lookup is performed. A leading `::` (no left-hand side)
always starts from the root namespace. This is always unambiguous because namespaces and types occupy separate name
spaces — a namespace `Option` and an enum `Option` cannot coexist (this would be a name conflict).

### 23.11 File Path Convention

Namespace structure **should** mirror the directory structure. This is a recommended convention, not a compiler-enforced
rule:

| Namespace         | Recommended Path                                 |
|-------------------|--------------------------------------------------|
| `survival`        | `survival/*.writ` or `survival.writ`             |
| `survival::items` | `survival/items/*.writ` or `survival/items.writ` |
| `quest_system`    | `quest_system/*.writ`                            |

The compiler does not validate that file paths match namespace declarations. A file at `combat/spells.writ` may declare
`namespace ui;` without error. However, violating the convention makes the project harder to navigate and the language
server flags it as a warning.

> **Note:** All files in the project are gathered and indexed before compilation. The compiler discovers all `.writ`
> files in the project directory (as defined by `writ.toml`) and uses namespace declarations — not file paths — for
> symbol
> resolution.

---

