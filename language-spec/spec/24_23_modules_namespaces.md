# Writ Language Specification
## 23. Modules & Namespaces

Every Writ source file belongs to a namespace. Namespaces organize declarations into logical groups and prevent name
collisions. Multiple files may contribute to the same namespace. Access across namespaces uses the `::` operator.

### 23.1 Declarative Namespace

The declarative form assigns the entire file to a single namespace:

```
// file: survival/potions.writ
namespace survival;

struct HealthPotion {
    charges: int,
    healAmount: int,
}

fn heal(target: Entity, amount: int) {
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
    struct HealthPotion {
        charges: int,
        healAmount: int,
    }

    namespace items {
        struct Bread {
            freshness: float,
        }

        struct Water {
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

const MAX_LEVEL: int = 50;

// file: game/main.writ
namespace game;

fn example() {
    let cap = MAX_LEVEL;   // accessible without qualification
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
    let pot = HealthPotion(charges: 3, healAmount: 50);
    heal(pot, 25);
}
```

Without the `using`, these would require `survival::HealthPotion` and `survival::heal`.

#### 23.4.1 Alias Form

The alias form binds a namespace to a shorter name:

```
using items = survival::items;

fn example() {
    let bread = items::Bread(freshness: 1.0);
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
        let pot = HealthPotion(charges: 3, healAmount: 50);
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

All declarations within the same namespace are visible to each other without `::` qualification, regardless of which
file they are defined in:

```
// file: survival/potions.writ
namespace survival;

struct HealthPotion {
    charges: int,
    healAmount: int,
}

// file: survival/crafting.writ
namespace survival;

fn brewPotion() -> HealthPotion {
    // HealthPotion is visible — same namespace, no :: needed
    HealthPotion(charges: 3, healAmount: 50)
}
```

This holds for all declaration kinds: structs, enums, functions, entities, components, contracts, and constants.

### 23.6 Name Conflicts

If two namespaces define a type or function with the same name, and both are brought into scope via `using`, any *
*unqualified** reference to that name is a compile error:

```
namespace ns_a;
struct Item { name: string }

namespace ns_b;
struct Item { id: int }
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

### 23.7 Cross-Namespace Access

The `::` operator accesses names within a namespace:

```
let pot = survival::HealthPotion(charges: 3, healAmount: 50);
let bread = survival::items::Bread(freshness: 1.0);
survival::heal(player, 25);
```

Fully qualified names always work, regardless of `using` declarations. They also resolve ambiguity when multiple `using`
statements bring conflicting names into scope.

### 23.8 Root Namespace Prefix (`::`)

A leading `::` with no left-hand side refers to the root namespace. This resolves ambiguity when a nested namespace
shadows an outer one:

```
namespace engine {
    namespace audio {
        struct Mixer { channels: int }
    }
}

namespace audio {
    struct Mixer { sampleRate: int }
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
let x = ::survival::HealthPotion(charges: 3, healAmount: 50);
let y: ::survival::HealthPotion = x;
::survival::heal(player, 25);
```

### 23.9 `::` Resolution

The `::` operator is used in three contexts:

1. **Root namespace access** — `::survival::HealthPotion` (leading `::`, resolve from root)
2. **Namespace access** — `survival::HealthPotion`, `survival::items::Bread`
3. **Enum variant access** — `QuestStatus::InProgress`, `Option::Some(value)`

The compiler resolves `::` by checking whether the left-hand side names a namespace or a type. If it names a namespace,
namespace lookup is performed. If it names an enum type, variant lookup is performed. A leading `::` (no left-hand side)
always starts from the root namespace. This is always unambiguous because namespaces and types occupy separate name
spaces — a namespace `Option` and an enum `Option` cannot coexist (this would be a name conflict).

### 23.10 File Path Convention

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
> files in the project directory (as defined by `writ.toml`) and uses namespace declarations — not file paths — for symbol
> resolution.

---

