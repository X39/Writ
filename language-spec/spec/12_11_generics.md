# Writ Language Specification
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
    fn consume(who: Entity) -> T;
    fn getCharges() -> int;
}

impl Consumable<HealEffect> for HealthPotion {
    fn consume(who: Entity) -> HealEffect {
        HealEffect(self.healAmount)
    }
    fn getCharges() -> int {
        self.charges
    }
}
```

### 11.1 Compiler Implementation Notes

The compiler performs generic validation at declaration sites (ensuring bounded type parameters are used correctly) and
at call sites (verifying concrete types satisfy bounds). The runtime uses type-tagged dynamic dispatch — a lookup table
of `(type_tag, contract, method_name) → function pointer`. Hot paths can be JIT-compiled to monomorphized versions.

---

