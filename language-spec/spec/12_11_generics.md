# 1. Writ Language Specification
## 11. Generics

Type parameters can be unbounded or bounded by one or more contracts.

```
// Unbounded generic
fn first<T>(items: List<T>) -> T {
    items[0]
}

// Single bound
fn sum<T: Add<T, T>>(a: T, b: T) -> T {
    a + b
}

// Multiple bounds
fn process<T: Consumable + Tradeable>(item: T) {
    // item has methods from both contracts
}

// Generic contract
contract Consumable<T> {
    fn consume(mut self, who: Entity) -> T;
    fn getCharges(self) -> int;
}

impl Consumable<HealEffect> for HealthPotion {
    fn consume(mut self, who: Entity) -> HealEffect {
        HealEffect(self.healAmount)
    }
    fn getCharges(self) -> int {
        self.charges
    }
}
```

### 11.1 Compiler and Runtime Notes

The compiler performs generic validation at declaration sites (ensuring bounded type parameters are used correctly) and
at call sites (verifying concrete types satisfy bounds). Value types passed through generic parameters are **boxed** —
wrapped in a heap-allocated container with a type tag — to allow uniform representation.

At runtime, generic dispatch resolves through the **contract dispatch table**: a mapping from
`(concrete_type_tag, contract_id, method_slot)` to a method entry point. The spec does not mandate a specific dispatch
strategy — runtimes may use hash tables, vtable arrays, inline caches, or monomorphization as they see fit.

### 11.2 Generic Call Syntax

At call sites, type arguments are provided with `<T>` directly after the function or type name.

```
let item = first<Item>(inventory);
let result = parse<int>("42");

// Type arguments can be omitted when the compiler can infer them from the arguments:
let item = first(inventory);    // T inferred as Item from List<Item>
```

> **Parser disambiguation:** The parser distinguishes `f<T>(args)` from `a < b` by syntactic lookahead —
> it examines tokens after `<` to determine whether they form a type argument list closed by `>(`.

---

