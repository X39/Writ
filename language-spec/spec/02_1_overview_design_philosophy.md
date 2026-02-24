# Writ Language Specification
## 1. Overview & Design Philosophy

Writ is a statically-typed game scripting language designed around two core constructs: `fn` for general-purpose C-style
logic, and `dlg` for dialogue authoring. The language also provides `entity` and `component` declarations for defining
game objects within an entity-component architecture. All higher-level constructs (`dlg`, `entity`) lower to simpler
primitives at compile time.

### 1.1 Design Goals

- **Trivial tokenization.** The lexer should be able to classify any token with minimal lookahead (ideally one token).
  Sigils and keywords create unambiguous mode switches.
- **Dialogue-first ergonomics.** Writing dialogue lines should require minimal ceremony. Non-developers should be able
  to author `dlg` blocks without understanding the type system.
- **C-style scripting familiarity.** The `fn` world uses braces, semicolons, and standard operators. Anyone who has
  written C, Java, JavaScript, or Rust will feel at home.
- **Rust-inspired type safety.** Contracts (traits), bounded generics, tagged enums, Result/Option types, and
  immutability by default catch errors at compile time.
- **Entity-component architecture.** Game objects are declared as entities with composable components, providing a
  familiar and flexible game development model.
- **Runtime agnosticism.** The language compiles to an intermediate representation. Runtimes can interpret it directly
  or JIT-compile hot paths.

### 1.2 Construct Hierarchy

All higher-level constructs lower to simpler ones at compile time:

```
dlg      →  fn calls (say, choice, etc.)
entity   →  struct + impl + component registration
contract →  dispatch table entries
operator →  contract impl (Add, Sub, etc.)
T?       →  Option<T>
null     →  Option::None
```

> **Note:** The runtime only needs to understand functions, structs, and a dispatch table. Everything else is compiler
> sugar.

---

