# Writ Language Specification
## 8. Structs

Structs are named composite types with named fields. They support methods and operator overloading via `impl` blocks.

```
struct Merchant {
    name: string,
    gold: int,
    reputation: float,
}

impl Merchant {
    fn greet() -> string {
        $"Welcome! I am {self.name}"
    }
}

// Construction uses named fields
let m = Merchant(name: "Old Tim", gold: 100, reputation: 0.8);
```

---

